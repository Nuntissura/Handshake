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
- STUB_FORMAT_VERSION: 2026-04-06
- STUB_STATUS: STUB (NOT READY FOR DEV)
- ACTIVATION_MANAGER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Micro-Task-Executor, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-MTE-LoRA-Wiring-v1
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_CONTROL_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl
- SESSION_CONTROL_RESULTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_REPAIR_ONLY
- SESSION_COMPATIBILITY_QUEUE_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- PLANNED_EXECUTION_OWNER_RANGE: Coder-A..Coder-Z
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
