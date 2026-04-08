# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1
- BASE_WP_ID: WP-1-FEMS-Working-Memory-Checkpoint-Schema
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
  - §2.6.6.7.6.2.2 Memory taxonomy — working class: short-horizon session state
  - §4.3.9.12.7 ModelSession FEMS integration — SESSION_SCOPED memory_policy
  - §2.6.6.6.6 FEMS Job Profile — memory_extract_v0.1

## INTENT (DRAFT)
- What: Define structured checkpoint types for FEMS working memory with quality gates, porting the repomem conversation checkpoint pattern. Checkpoints become the input for `memory_extract_v0.1` jobs and the bridge from ephemeral session state to durable memory.
- Why: FEMS working memory is defined as "short-horizon state for the current session" but has no structured schema for what gets captured or when. Without checkpoints, memory extraction is unstructured and session-to-session continuity depends on model judgment alone.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Checkpoint type schema: SESSION_OPEN (session intent), PRE_TASK (current assumptions before complex op), INSIGHT (key realization — highest signal), TASK_COMPLETE (outcome), SESSION_CLOSE (triggers memory_extract job).
  - Quality gates per checkpoint type: minimum content length, structured fields (decisions, files_referenced, scope_refs).
  - Checkpoint → memory_extract bridge: SESSION_CLOSE triggers `memory_extract_v0.1` job that processes all session checkpoints into MemoryWriteProposal.
  - INSIGHT checkpoint promotion: insights that appear across 3+ sessions are candidates for semantic/procedural promotion (feeds hygiene manager).
  - Working memory GC: checkpoints from completed sessions with no insights are garbage-collected aggressively (per FEMS spec: working class "MAY be garbage-collected aggressively unless pinned").
  - Flow awareness auto-population (Windsurf pattern): auto-populate working memory from the ModelSession action stream — files read, tools called, entities queried, searches performed. No explicit "remember this" needed; the session's activity IS the working memory source. Checkpoints can be both explicit (user/model writes INSIGHT) and implicit (action stream capture).
- OUT_OF_SCOPE:
  - FEMS core MemoryItem schema (already in spec).
  - Consolidation/hygiene (WP-1-FEMS-Hygiene-Manager-Job-v1).
  - DCC conversation timeline integration (separate concern).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; defines working memory structure | Stub follow-up: THIS_STUB
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: checkpoints are written during ModelSession execution; SESSION_CLOSE triggers extract job | Stub follow-up: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: PRE_TASK checkpoints scoped to active WP/MT via scope_refs | Stub follow-up: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC conversation timeline could display checkpoint stream | Stub follow-up: NONE
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: checkpoints correlate with FR event timeline for debugging | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- Checkpoint types are defined as a typed schema with validation.
- Quality gates reject low-quality checkpoints (minimum content, required fields).
- SESSION_CLOSE triggers memory_extract_v0.1 job that produces a MemoryWriteProposal from session checkpoints.
- Cross-session insight detection identifies repeated INSIGHT topics across 3+ sessions.
- Working memory from completed sessions without insights is GC'd within configurable window.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for working memory class and memory_extract job profile.

## RISKS / UNKNOWNs (DRAFT)
- Risk: quality gates may be too restrictive for fast-paced interactive sessions. Need configurable thresholds.
- Risk: cross-session insight detection requires topic similarity matching — FTS5 vs embedding trade-off.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`.
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
