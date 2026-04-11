# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1

## STUB_METADATA
- WP_ID: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Governed-Next-Action-Alignment
- CREATED_AT: 2026-03-25T14:15:34.0047668Z
- STUB_FORMAT_VERSION: 2026-03-16
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Contract-Hardening, WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: NONE
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
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- PLANNED_EXECUTION_OWNER_RANGE: Coder-A..Coder-Z
- ROADMAP_POINTER: Master-spec closure remediation after `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- AUDIT_DRIVERS:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.178.md lines 6088-6098 StructuredCollaborationSummaryV1 compact summary contract (`next_action`)
  - Handshake_Master_Spec_v02.178.md lines 6980-6986 project-agnostic workflow state, queue reason, and governed action contract (`GovernedActionDescriptorV1`)
  - Handshake_Master_Spec_v02.178.md line 73325 Work Packet detail and board surfaces SHOULD expose governed next actions attached to the current view preset

## INTENT (DRAFT)
- What: Align live structured-collaboration `next_action` summary fields and related preview helpers to registered `GovernedActionDescriptorV1.action_id` values instead of ad hoc summary tokens or residual prose helpers.
- Why: `WP-1-Structured-Collaboration-Contract-Hardening-v1` closed `allowed_action_ids`, Task Board authority, and mailbox leak-safety, but the closeout review still found remaining spec debt around governed next actions. Current summary emitters still publish `next_action` values that are not guaranteed to resolve through the governed action registry.

## AUDIT_FINDINGS_THIS_STUB_COVERS (DRAFT)
- `StructuredCollaborationSummaryV1.next_action` is still emitted as ad hoc summary tokens such as `start_work_packet` and `start_micro_task` rather than as registered `GovernedActionDescriptorV1.action_id` values.
- Residual dead helper paths in `workflows.rs` still describe next actions as prose strings, which obscures whether the governed-action contract is actually authoritative.
- Summary validation still treats `next_action` as a generic non-empty string rather than proving registry-backed legality for the in-scope record families.
- Current tests prove summary equality between packet and summary artifacts, but they do not prove that `next_action` remains governed, registered, and compatible with the record's workflow posture.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Align structured Work Packet and Micro-Task summary `next_action` emission to a deterministic governed-action rule backed by `GovernedActionDescriptorV1.action_id`.
  - Decide and implement the canonical policy for ambiguous states: either emit one deterministic governed action id or omit `next_action`; do not emit ad hoc strings.
  - Remove or align dead residual next-action helper paths so product code has one authoritative next-action contract.
  - Harden validation and negative-path tests for in-scope summary records so unregistered `next_action` values are rejected or impossible.
- OUT_OF_SCOPE:
  - Broad workflow-state registry work.
  - Broad workflow transition and automation registry work.
  - Queue-reason or `allowed_action_ids` remediation already closed by `WP-1-Structured-Collaboration-Contract-Hardening-v1`.
  - Repo-governance / ACP workflow-harness remediation.

## CODE_REALITY_HINTS (DRAFT)
- Path: `src/backend/handshake_core/src/workflows.rs` | Covers: live summary emitters and residual next-action helpers | Notes: summary emission currently uses custom next-action tokens that do not match the governed action registry, while dead helper functions still express prose next-action intent.
- Path: `src/backend/handshake_core/src/locus/types.rs` | Covers: structured summary record contract and validation | Notes: current validation only proves `next_action` is non-empty when present.
- Path: `src/backend/handshake_core/tests/micro_task_executor_tests.rs` | Covers: summary proof | Notes: current tests compare emitted summary fields but do not enforce governed next-action legality.

## ACCEPTANCE_CRITERIA (DRAFT)
- Every emitted `next_action` in the in-scope structured Work Packet and Micro-Task summaries is either:
  - a registered `GovernedActionDescriptorV1.action_id` compatible with the record's current workflow posture, or
  - omitted when no single deterministic governed next action is defensible.
- No live emitter path in scope publishes ad hoc `next_action` tokens or prose-only next-action strings.
- In-scope validation and tests prove that unregistered or drifted `next_action` values fail mechanically.
- Dead or alternate next-action helper paths in scope are removed or aligned to the same governed-action rule so the codebase has one next-action contract.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Signed activation should cite `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW` as the evidence driver.
- Refinement must decide whether `next_action` should always be one canonical governed action id or be omitted for ambiguous states.

## RISKS / UNKNOWNs (DRAFT)
- For some workflow states, more than one governed action may be legal; forcing a single summary value can overstate certainty.
- Hidden consumers may depend on the current `start_work_packet` / `start_micro_task` token vocabulary even though no product consumer is currently obvious from repo search.
- If the summary contract remains too loose, the product can still look "machine readable" while hiding another registry split behind one unchecked string field.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Re-read `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW` before narrowing scope.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Structured-Collaboration-Governed-Next-Action-Alignment-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
