# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Hygiene-Manager-Job-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Hygiene-Manager-Job-v1
- BASE_WP_ID: WP-1-FEMS-Hygiene-Manager-Job
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
  - §2.6.6.6.6 Front End Memory Job Profile (FEMS) — memory_consolidate_v0.1, memory_forget_v0.1
  - §2.6.6.7.6.2 Front End Memory System — consolidation lifecycle, compaction, decay
  - §10.11.5.14 Front End Memory Panel — conflict/consolidation queue

## INTENT (DRAFT)
- What: Implement a scheduled FEMS hygiene job that runs consolidation, pruning, flagging, and promotion using a cost-effective model. Ports the battle-tested Memory Manager pattern from repo governance into the product Execution / Job Runtime.
- Why: FEMS defines `memory_consolidate_v0.1` and `memory_forget_v0.1` job kinds but has no scheduled orchestration. Without active hygiene, LongTermMemory grows unbounded, stale items pollute MemoryPack injection, and contradictions accumulate unresolved.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Background AI job using `memory_consolidate_v0.1` and `memory_forget_v0.1` profiles.
  - Scheduled triggers: on session close, on cron (configurable interval).
  - Hygiene rubric implementation: FLAG stale scope_refs, PRUNE old episodic clusters, REPAIR broken supersession chains, PROMOTE cross-session patterns.
  - Integration with DCC Approval Inbox for procedural memory promotions requiring human review.
  - Hygiene report artifact (MemoryCommitReport) with FR-EVT-MEM-003 emission.
  - Cost-effective model routing (cheap model for mechanical hygiene, not cloud-tier).
  - Sleep-time reflection (memclawz pattern): during idle time (no active sessions, app in background), run reflection jobs that go beyond cleanup — find cross-memory connections, detect emerging patterns across working/episodic items, pre-compute MemoryPacks for likely next sessions. Distinct from hygiene (cleanup) — reflection creates new knowledge. Uses background lane in session scheduler.
- OUT_OF_SCOPE:
  - FEMS core read/write paths (already in WP-1-Front-End-Memory-System-v1).
  - Anti-poisoning guardrails (WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1).
  - Write-time mechanical safeguards (separate stub: WP-1-FEMS-Write-Time-Safeguards-v1).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; implements the consolidation/forget lifecycle | Stub follow-up: THIS_STUB
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: hygiene manager IS an AI job in the runtime; uses ModelSession, job lifecycle, lane scheduling | Stub follow-up: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC Approval Inbox surfaces procedural promotions for human review; calibration signals in Memory Panel | Stub follow-up: NONE
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: every consolidation/forget action emits FR-EVT-MEM-003/005 events | Stub follow-up: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: UNKNOWN | NOTES: procedural memories about model failure patterns could feed distillation training priorities | Stub follow-up: WP-1-Distillation-v2

## ACCEPTANCE_CRITERIA (DRAFT)
- Hygiene job runs on schedule without manual trigger.
- Stale, contradicted, and low-importance items are flagged/pruned with logged rationale.
- Cross-session pattern promotion creates new semantic/procedural MemoryItems via governed MemoryWriteProposal.
- Procedural promotions route through DCC Approval Inbox.
- All actions emit FR-EVT-MEM-* events with artifact handles.
- Total memory store stays within configurable budget (default 500 items).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for LongTermMemory store and FEMS job profiles.

## RISKS / UNKNOWNs (DRAFT)
- Risk: cheap model may make poor consolidation decisions; need quality gate on hygiene output.
- Risk: aggressive pruning could delete useful memories; conservative defaults with operator override.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Hygiene-Manager-Job-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Hygiene-Manager-Job-v1`.
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
