# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1

## STUB_METADATA
- WP_ID: WP-1-ModelSession-Postgres-Queue-Workers-v1
- BASE_WP_ID: WP-1-ModelSession-Postgres-Queue-Workers
- CREATED_AT: 2026-05-05T17:55:00Z
- STUB_STATUS: SUPERSEDED (FOLDED INTO WP-1-Postgres-Control-Plane-Shift-Bundle-v1)
- FOLDED_INTO: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Postgres-Dev-Test-Container-Matrix, WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-ModelSession-Core-Scheduler
- BUILD_ORDER_BLOCKS: WP-1-FEMS-Postgres-Memory-Store, WP-1-DCC-Postgres-Control-Plane-Projections, WP-1-Session-Spawn-Tree-DCC-Visualization
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT ModelSession scheduler and storage portability anchors
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Multi-session orchestration and ModelSession anchors around Master Spec lines 95-96.
  - ModelSession memory policy anchor around Master Spec line 11967.

## INTENT (DRAFT)
- What: Move ModelSession scheduling work queues, worker claims, session run state, and message/checkpoint persistence onto PostgreSQL as the authoritative runtime store.
- Why: The control plane must run multiple GPT, Claude, local, and memory-worker sessions in parallel without relying on process-local state or SQLite-only assumptions.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - PostgreSQL persistence for queued model runs, active workers, session state, messages, checkpoints, and terminal outcomes.
  - Worker claim/reclaim integration with the shared lease/backpressure primitives.
  - Provider/model profile fields stored as catalog profile IDs, not ambient CLI aliases.
  - Tests for parallel workers, cancellation, crash resume, duplicate message prevention, and budget/limit metadata preservation.
- OUT_OF_SCOPE:
  - New provider adapters.
  - UI beyond projection-ready status fields.
  - FEMS memory content storage, except for linking ModelSession IDs to memory-policy requests.

## ACCEPTANCE_CRITERIA (DRAFT)
- Multiple worker processes can claim distinct ModelSession queue items without duplicate execution.
- Session resume reads canonical PostgreSQL state and does not rely on local process memory.
- Profile IDs, reasoning strength, provider family, and fallback profile are persisted with each session/run.
- Crash and cancellation paths leave enough state for deterministic DCC and Flight Recorder projection.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the foundation WP, the Postgres test matrix, control-plane lease/backpressure primitives, and the existing ModelSession scheduler contract.
- Blocks FEMS memory store integration and DCC multi-session projections.

## RISKS / UNKNOWNs (DRAFT)
- Risk: current scheduler tests may encode SQLite/local assumptions; activation should include negative-path tests before implementation.
- Unknown: whether each provider worker should own a separate queue partition or use a shared queue with profile filters.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-ModelSession-Postgres-Queue-Workers-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
