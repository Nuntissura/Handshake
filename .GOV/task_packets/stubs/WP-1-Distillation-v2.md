# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Distillation-v2

## STUB_METADATA
- WP_ID: WP-1-Distillation-v2
- BASE_WP_ID: WP-1-Distillation
- CREATED_AT: 2026-01-11T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Micro-Task-Executor, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-MTE-LoRA-Wiring-v1
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Section 9 Distillation Track + spec v02.157 distillation/context/spec-router backend pass
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Section 9 Continuous Local Skill Distillation (Skill Bank & Pipeline)
  - 2.6.6.8.13 Learning Integration
  - 5.3.6 Distillation Observability Requirements
  - 2.5.12 Context Packs AI Job Profile

## Why this stub exists
This is an additive remediation stub for `WP-1-Distillation`.

It exists because the skill-distillation backend is now explicitly modeled as a first-class backend learning substrate, while the actual late-stage adapter training, eval gating, and rollback-safe promotion path remain incomplete.

## Prior packet
- Prior WP_ID: `WP-1-Distillation`
- Prior packet: `.GOV/task_packets/WP-1-Distillation.md`

## Known gaps (Task Board summary)
- / FAIL: teacher/student lineage, Skill Bank schema, benchmark-gated eval/promotion, adapter-only late-stage training posture, and cross-tokenizer-safe replay evidence remain incomplete. [STUB]

## INTENT (DRAFT)
- What: complete the late-stage Skill Bank / distillation backend so teacher-student lineage, candidate selection, checkpoint/eval gating, adapter-only training posture, and rollback-safe promotion become explicit runtime contracts.
- Why: the spec now hardcodes LoRA / QLoRA / DoRA posture, PromptEnvelope / Context Pack reuse, and distillation evidence requirements, but the implementation path still needs a dedicated packet.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - persist full distillation lineage:
    - teacher/student ids
    - tokenizer metadata
    - Context Pack hashes / PromptEnvelope hashes when used
    - checkpoint parents, eval suite ids, promotion decisions
  - benchmark-gated adapter lifecycle:
    - rank/alpha/repeats/epochs tracked as first-class hyperparameters
    - LoRA / QLoRA / DoRA training outcomes compared against teacher + previous checkpoint
    - rollback-safe promotion and merge policy
  - export / replay posture:
    - capability-gated export controls for checkpoints and eval artifacts
    - deterministic replay metadata sufficient for later local/cloud audit
- OUT_OF_SCOPE:
  - end-user UI polish for model-training consoles
  - full-model fine-tuning beyond adapter-only posture

## ACCEPTANCE_CRITERIA (DRAFT)
- Distillation lineage is durable and queryable: teacher/student ids, tokenizer metadata, Context Pack hashes, PromptEnvelope hashes, checkpoint parents, and eval decisions are recorded.
- Adapter training hyperparameters and promotion/rollback outcomes are benchmark-gated and replayable.
- Export controls prevent off-device leakage of local-only checkpoints or eval artifacts.

## RISKS / UNKNOWNs (DRAFT)
- Poor-quality synthetic/self-distilled data can cause collapse if it dominates trusted traces.
- Cross-tokenizer assumptions can silently corrupt teacher/student comparisons if token ids are treated as interchangeable.
- Adapter merge/promotion without strict eval gates can create regressions that look like success in small tests.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Distillation-v2.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Distillation-v2` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
