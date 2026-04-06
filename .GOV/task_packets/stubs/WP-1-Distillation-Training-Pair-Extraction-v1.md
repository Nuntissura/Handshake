# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Distillation-Training-Pair-Extraction-v1

## STUB_METADATA
- WP_ID: WP-1-Distillation-Training-Pair-Extraction-v1
- BASE_WP_ID: WP-1-Distillation-Training-Pair-Extraction
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Distillation, WP-1-Session-Spawn-Contract
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Distillation pipeline fed by governed WP execution
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md Distillation pipeline sections
  - Handshake_Master_Spec_v02.179.md 4.3.9.15 Session Spawn Contract (announce-back conversations)
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-108, PARL)

## INTENT (DRAFT)
- What: After WP closure, extract structured training pairs from governed execution: task decomposition examples (refinement to MT plan), code generation examples (MT prompt to committed diff), review examples (diff to validator findings), fix examples (STEER response to fix diff). Store as JSONL in the artifact system tagged with WP_ID, MT_ID, model used, success/failure, fix cycle count.
- Why: Feeds the distillation pipeline (WP-1-Distillation-v2) and LoRA fine-tuning (WP-1-MTE-LoRA-Wiring-v1). Based on PARL trained orchestrator concept. Governed sessions produce high-quality input-output pairs grounded in real product work. Capturing these pairs enables training smaller, cheaper local models to handle routine tasks, reducing cloud model costs and enabling offline operation.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Training pair extraction from session output JSONL after WP closure.
  - Structured JSONL output with training pair types (decompose, generate, review, fix).
  - Tagging with metadata (WP_ID, MT_ID, model, success/failure, fix_cycle_count).
  - Storage in artifact system with retention policy.
  - CLI command for manual extraction: just extract-training-pairs WP-{ID}.
- OUT_OF_SCOPE:
  - The distillation pipeline core (WP-1-Distillation-v2).
  - LoRA training infrastructure (WP-1-MTE-LoRA-Wiring-v1).
  - Live streaming extraction during execution (post-closure only).

## ACCEPTANCE_CRITERIA (DRAFT)
- Training pairs are extracted from completed governed session logs after WP closure.
- Output is stored as valid JSONL with required metadata fields (WP_ID, MT_ID, model, success/failure, fix_cycle_count).
- Four training pair types are supported: decompose, generate, review, fix.
- Extracted pairs are compatible with the distillation pipeline input format.
- CLI command `just extract-training-pairs WP-{ID}` runs extraction for a specific WP.
- Artifacts are stored with retention policy in the governed artifact system.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Distillation for the pipeline input format and integration point.
- Depends on WP-1-Session-Spawn-Contract for the session log format and storage location.
- No spec blockers.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Session logs may not capture sufficient context for high-quality training pairs without additional instrumentation.
- Risk: Training pair volume may be insufficient for meaningful LoRA fine-tuning without many completed WPs.
- Unknown: Optimal granularity of training pairs (full session vs. individual tool calls vs. task-level).

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Distillation-Training-Pair-Extraction-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Distillation-Training-Pair-Extraction-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
