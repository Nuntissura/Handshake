# Task Packet Stub: WP-1-Micro-Task-Executor-v1

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-Micro-Task-Executor-v1
- BASE_WP_ID: WP-1-Micro-Task-Executor
- Created: 2026-01-22
- SPEC_TARGET: docs/SPEC_CURRENT.md (currently Handshake_Master_Spec_v02.115.md)
- SPEC_ORIGIN_VERSION: v02.114 (Micro-Task Executor Profile)
- ROADMAP_SOURCE: Handshake_Master_Spec_v02.115.md §7.6.3 Phase 1 -> Mechanical Track / Distillation Track [ADD v02.114]
- SPEC_ANCHOR_CANDIDATE:
  - Handshake_Master_Spec_v02.115.md §7.6.3 Phase 1 (Micro-Task Executor bullets)
  - Handshake_Master_Spec_v02.115.md §2.6.6.8 (Micro-Task Executor)
  - Handshake_Master_Spec_v02.115.md §6.3 / §11.8 (Mechanical Tool Bus / MEX v1.2 envelopes and gates)

## Roadmap fixed fields (copied from spec; draft)
- Goal:
  - Make "micro-task iteration" a first-class, auditable job loop with deterministic escalation and hard-gate pauses (no silent infinite loops).
- MUST deliver:
  - Micro-Task Executor core loop controller with:
    - auto-generation of MT definitions from Work Packet scope
    - fresh-context-per-iteration execution
    - completion signal parsing with anti-gaming rules
    - bounded iteration limits
  - MT validation engine wiring:
    - validation commands run through Mechanical Tool Bus (PlannedOperation envelope)
    - capability checks
    - FR-EVT-MT-012 emission
  - MT state persistence:
    - ProgressArtifact + RunLedger schemas with atomic writes
    - crash recovery
    - FR-EVT-WF-RECOVERY integration
  - MT escalation chain:
    - default 6-level escalation (7B->7B-alt->13B->13B-alt->32B->HARD_GATE)
    - LoRA selection by task_tags (auto_by_task_tags)
  - Distillation escalation artifacts (logging-only; no training in Phase 1):
    - on escalation with `enable_distillation=true`, emit DistillationCandidate artifacts + FR-EVT-MT-015
    - capture contributing_factors + remediation outcomes for LoRA feedback
- Key risks addressed in Phase 1:
  - Agentic loops become non-deterministic and un-auditable (no ledger, no caps, no escalation evidence).
  - Safety gates are bypassed by running validations outside the canonical tool bus.
- Acceptance criteria:
  - MT Executor job profile (`micro_task_executor_v1`) visible in Job History.
  - At least one Work Packet executes end-to-end with auto-generated MTs.
  - Escalation triggers FR-EVT-MT-005; hard gate pauses execution.
  - FR-EVT-MT-015 emitted and DistillationCandidate artifacts stored; no LoRA training occurs in Phase 1.
- Explicitly OUT of scope:
  - Any actual LoRA training or model promotion (Phase 2+).
- Mechanical Track:
  - YES (validation engine wiring via Tool Bus).
- Atelier Track:
  - N/A.
- Distillation Track:
  - YES (candidate capture + schema alignment; logging-only).
- Vertical slice:
  - Execute one real WP through MT executor: generate MTs, run validations via tool bus, hit at least one escalation, and inspect ledger + FR evidence in Operator Consoles.

## Why this stub exists
Handshake_Master_Spec_v02.115.md introduces the Micro-Task Executor Profile (v02.114; §2.6.6.8) and adds Phase 1 requirements tagged [ADD v02.114] for the deterministic MT loop + escalation candidate capture. This stub tracks the work to implement the loop, state persistence, escalation policy, and canonical logging so agentic refinement is replayable and safe.

## Scope sketch (draft)
- In scope:
  - MT loop controller, persistence, escalation chain, and required Flight Recorder events.
  - Validation command routing through the Mechanical Tool Bus (PlannedOperation/EngineResult).
- Out of scope:
  - Training pipelines, LoRA build automation, or automatic model promotion.

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md` Part 2.5.2).
2. USER_SIGNATURE.
3. Create `docs/refinements/WP-1-Micro-Task-Executor-v1.md`.
4. Create official task packet via `just create-task-packet WP-1-Micro-Task-Executor-v1`.
5. Update `docs/WP_TRACEABILITY_REGISTRY.md` to point `WP-1-Micro-Task-Executor` -> `WP-1-Micro-Task-Executor-v1`.
6. Update `docs/TASK_BOARD.md` to move `WP-1-Micro-Task-Executor-v1` out of STUB when activated.
