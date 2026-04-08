# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Outcome-Feedback-Loop-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Outcome-Feedback-Loop-v1
- BASE_WP_ID: WP-1-FEMS-Outcome-Feedback-Loop
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System, WP-1-FEMS-Injection-Scoring-Graceful-Degradation
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §2.6.6.7.6.2.4 Read path — retrieve, rank, pack
  - §2.6.6.8.5 MicroTaskExecutorJob — iteration outcomes
  - §2.6.6.8.13 Skill distillation — escalation-driven candidate generation

## INTENT (DRAFT)
- What: After a MemoryPack is used in a model call, track the outcome and feed it back into memory scoring. Good outcomes (MT completed, validator PASS) boost the items in that pack. Bad outcomes (MT failed, escalation) penalize them. Over time the scoring formula self-tunes. Ports the Cognee self-improving memory pattern.
- Why: The injection scoring formula uses static weights (importance, recency, trust). Without outcome feedback, a memory that consistently leads to failures keeps getting injected with the same score. The feedback loop creates a reinforcement signal: memories that help stay, memories that mislead fade.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - After each model call, record which MemoryPack items were injected (from FR-EVT-MEM-004 pack_id + item list).
  - After the session/MT reaches an outcome (completed, failed, escalated), correlate the outcome with the injected items.
  - Positive outcome → increment `access_boost` or add a small importance delta to each item that was in the pack.
  - Negative outcome → apply a small penalty to items in the pack. Not aggressive — multiple negative outcomes needed before an item drops significantly.
  - Feedback signals stored as metadata on MemoryItem (outcome_positive_count, outcome_negative_count) feeding into the injection scoring formula.
  - Connection to Skill distillation: memory items consistently correlated with escalation are candidates for LoRA training pairs ("the model needed this memory because it couldn't do X natively").
- OUT_OF_SCOPE:
  - Real-time scoring adjustment during a session (feedback is post-session).
  - Automatic procedural memory generation from feedback (that's hygiene manager promotion).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; adds feedback loop to scoring | Stub follow-up: THIS_STUB
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: MT completion/failure/escalation events are the outcome signal | Stub follow-up: NONE
  - PILLAR: Skill distillation / LoRA | STATUS: TOUCHED | NOTES: memory-correlated escalation patterns feed distillation training priorities | Stub follow-up: WP-1-Distillation-v2
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: FR-EVT-MEM-004 + FR-EVT-MT-* correlation for outcome tracking | Stub follow-up: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: session outcome state feeds back into memory metadata | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- Memory items injected into successful sessions get measurable score boost.
- Memory items injected into failed sessions get measurable score penalty.
- Feedback is post-session (not real-time) and non-destructive (soft scoring, not deletion).
- After N sessions, the scoring formula produces noticeably different rankings than static weights alone.
- Escalation-correlated memory items are flagged as distillation candidates.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for MemoryItem schema.
- Depends on: WP-1-FEMS-Injection-Scoring-Graceful-Degradation-v1 for the scoring formula to feed into.

## RISKS / UNKNOWNs (DRAFT)
- Risk: attribution problem — a bad outcome may not be caused by the memory items. Soft penalties + multiple-outcome thresholds mitigate.
- Risk: feedback loop could create self-reinforcing biases. Need periodic reset or decay on feedback scores.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Outcome-Feedback-Loop-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Outcome-Feedback-Loop-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
