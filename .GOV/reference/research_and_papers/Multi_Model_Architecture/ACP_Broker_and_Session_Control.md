# ACP Broker and Session Control

## Purpose

This document explains the launch, transport, steering, and health surfaces that sit between the orchestrator and governed role sessions.
It focuses on the runtime mechanics of ACP/session control, not on role prompts or product code.

The goal is to understand:

- how sessions are launched
- how they are addressed and steered
- where runtime truth is stored
- how unhealthy sessions are detected
- where the current design is brittle

## Relationship to Other Research Documents

| Document | Role |
|---|---|
| `Gov_Kernel_Technical_Map.md` | Whole-system map |
| `ACP_Broker_and_Session_Control.md` (this file) | Deep dive on governed session launch and control |
| `Workflow_State_Packet_Truth_and_Range_Drift.md` | Deep dive on workflow truth and false gate failures |
| `Harness_Lessons_Learned.md` | Cross-cutting lessons and implications |

## Primary Code Surfaces

### Policy and runtime contract

- `.GOV/roles_shared/scripts/session/session-policy.mjs`
- `.GOV/roles_shared/scripts/session/acp-build-id.mjs`

These files define the policy surface: host preference, broker mode, registry files, request/result files, model-profile policy, runtime support, timeout values, and role-specific model defaults.

### Session-control library

- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-output-activity-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`

This is the control-plane library layer.
It defines request/result schemas, role authority strings, runtime paths, output handling, and shared session-state helpers.

### Broker and orchestrator-facing controls

- `.GOV/roles/orchestrator/scripts/session-control-broker.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs`
- `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`

These scripts are the operational entrypoints that launch, steer, and cancel governed sessions.

### Supporting health and cleanup surfaces

- `.GOV/roles_shared/scripts/session/session-stall-scan.mjs`
- `.GOV/roles_shared/scripts/session/wp-lane-health.mjs`
- `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
- `.GOV/roles_shared/scripts/session/scan-orphan-terminals.mjs`
- `.GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs`

These are the hygiene and recovery tools that exist because normal runtime operation is not fully self-stabilizing.

## Declared Runtime Model

From `session-policy.mjs`, the current declared model is approximately:

- session host preference: Handshake ACP broker
- session control mode: steerable
- primary transport: resume-json style control path
- primary protocol: `HANDSHAKE_ACP_STDIO_V1`
- broker auth mode: local token file
- session registry and control ledgers stored in shared runtime files
- role model profiles declared centrally, with provider-specific launch parameters

This is more sophisticated than a simple "spawn a terminal and hope" model.
It also means there are many places where declared runtime policy can diverge from actual behavior.

## Runtime State Model

The current runtime model is more explicit than the old narrative orchestration style.
`session-policy.mjs` declares:

- roles: `ACTIVATION_MANAGER`, `CODER`, `WP_VALIDATOR`, `INTEGRATION_VALIDATOR`, `MEMORY_MANAGER`
- runtime states:
  - `UNSTARTED`
  - `PLUGIN_REQUESTED`
  - `TERMINAL_COMMAND_DISPATCHED`
  - `PLUGIN_CONFIRMED`
  - `CLI_ESCALATION_READY`
  - `CLI_ESCALATION_USED`
  - `STARTING`
  - `READY`
  - `COMMAND_RUNNING`
  - `ACTIVE`
  - `WAITING`
  - `COMPLETED`
  - `FAILED`
  - `STALE`
  - `CLOSED`
- request statuses:
  - `QUEUED`
  - `PLUGIN_DISPATCHED`
  - `PLUGIN_CONFIRMED`
  - `PLUGIN_FAILED`
  - `PLUGIN_TIMED_OUT`
  - `CLI_ESCALATION_USED`
- command kinds:
  - `START_SESSION`
  - `SEND_PROMPT`
  - `CANCEL_SESSION`
  - `CLOSE_SESSION`
- supported ACP methods:
  - `session/new`
  - `session/load`
  - `session/prompt`
  - `session/cancel`
  - `session/close`
  - `broker/shutdown`
- command statuses:
  - `QUEUED`
  - `RUNNING`
  - `COMPLETED`
  - `FAILED`
- outcome states:
  - `NONE`
  - `SETTLED`
  - `ACCEPTED_RUNNING`
  - `ACCEPTED_QUEUED`
  - `ALREADY_READY`
  - `BUSY_ACTIVE_RUN`
  - `ACCEPTED_PENDING` (legacy pre-split accepted state)
  - `REQUIRES_START`
  - `REQUIRES_RECOVERY`
  - `FAILED`

This is good news architecturally.
It means the kernel is already trying to encode runtime truth as structured state rather than pure prompt interpretation.

## Core Ledgers and State Files

The control plane persists into several file-ledger surfaces:

### Registry and launch state

- `ROLE_SESSION_REGISTRY.json`
- `SESSION_LAUNCH_REQUESTS.jsonl`

The registry layer is managed by `session-registry-lib.mjs`.
It exposes mutation helpers such as:

- `mutateSessionRegistrySync`
- `getOrCreateSessionRecord`
- `ensureSessionStateFiles`
- `assertOrchestratorLaunchAuthority`

This is not just a convenience wrapper.
It is the durable state boundary that keeps session identity and command history outside the chat.

### Command request/result ledgers

- `SESSION_CONTROL_REQUESTS.jsonl`
- `SESSION_CONTROL_RESULTS.jsonl`

These ledgers are the durable command trail for steerable ACP operations.

### Per-command output

- `SESSION_CONTROL_OUTPUTS/<session_key>/<command_id>.jsonl`

This is the event/output trail per governed command.
It matters because recovery logic can infer missing terminal results from these files when the broker path is incomplete.

### Broker state

- `SESSION_CONTROL_BROKER_STATE.json`
- `broker_stdout.log`
- `broker_stderr.log`

The broker state includes process/build/reachability information plus active-run tracking.

## Request and Result Schemas

`session-control-lib.mjs` builds two primary message types.

### Session control request

Schema:

- `schema_id: hsk.session_control_request@1`
- `schema_version: session_control_request_v1`

Important fields:

- `command_id`
- `created_at`
- `command_kind`
- `created_by_role`
- `session_key`
- `wp_id`
- `role`
- `session_thread_id`
- `local_branch`
- `local_worktree_dir`
- `selected_model`
- `selected_profile_id`
- `reasoning_config_key`
- `reasoning_config_value`
- `prompt`
- `summary`
- `output_jsonl_file`
- `environment_overrides`
- `target_command_id` for cancel flows

Validation rules currently include:

- `created_by_role` must be `ORCHESTRATOR`
- `command_kind` must be one of the declared command kinds
- `prompt` is required except for `CANCEL_SESSION` and `CLOSE_SESSION`
- `output_jsonl_file` is required
- `target_command_id` is required for `CANCEL_SESSION`

### Session control result

Schema:

- `schema_id: hsk.session_control_result@1`
- `schema_version: session_control_result_v1`

Important fields:

- `command_id`
- `processed_at`
- `command_kind`
- `session_key`
- `wp_id`
- `role`
- `status`
- `outcome_state`
- `thread_id`
- `summary`
- `output_jsonl_file`
- `last_agent_message`
- `error`
- `duration_ms`
- `target_command_id`
- `cancel_status`
- `broker_build_id`

This is enough structure to support a real orchestrator and post-hoc audit.
It is also enough structure to justify moving this state into a stronger backend later if file-ledger scale becomes a problem.

## Core Runtime Artifacts

The current session-control system materializes state in `../gov_runtime/roles_shared/`:

- `ROLE_SESSION_REGISTRY.json`
- `SESSION_CONTROL_BROKER_STATE.json`
- `SESSION_CONTROL_REQUESTS.jsonl`
- `SESSION_CONTROL_RESULTS.jsonl`
- `SESSION_CONTROL_OUTPUTS/`
- launch queue files
- WP communication folders per work packet

This is a strong design direction because it externalizes state from the chat.
The weakness is that multiple authored and derived truths can still disagree.

## Governance Gates Before Control

The command path is not "send broker command immediately."
`session-control-command.mjs` first performs governance checks:

- validate command kind and WP shape
- resolve role config
- assert orchestrator launch authority from the current branch
- resolve launch profile and provider/model settings for `START_SESSION`
- ensure session state files exist
- create or load the durable session record
- evaluate governance state via `session-governance-state-lib.mjs`

`evaluateSessionGovernanceState` currently blocks or warns on things like:

- missing official packet for normal roles
- terminal task-board status
- missing assigned worktree

This is an important detail: ACP session control is already a governed runtime, not a raw broker client.

## Operational Flow by Command Kind

### `START_SESSION`

Current path:

1. Resolve role launch selection and model profile.
2. Assert the selected profile is supported for governed launch.
3. Ensure a durable session record exists.
4. Build the startup prompt.
5. Build a structured session-control request.
6. Call ACP method `session/new`.
7. Record ACP notifications into the workflow dossier execution log.
8. Classify the returned or recovered outcome.

Important fast paths:

- `ALREADY_READY` if the session already has a steerable thread
- recovery path if broker dispatch fails but a ready session or settled result can still be discovered

### `SEND_PROMPT`

Current path:

1. Verify steering is allowed by governance state.
2. Require an existing `session_thread_id`.
3. Build a structured request for the next prompt.
4. Call ACP method `session/prompt`.
5. If the broker reports `BUSY_ACTIVE_RUN`, surface that explicitly instead of retrying.

That last point is important.
The runtime already contains at least one anti-loop decision: explicit busy reporting instead of silent retry.

### `CANCEL_SESSION`

Current path:

1. Find the current or most recent governed command ID for the session.
2. Build a request with `target_command_id`.
3. Call ACP method `session/cancel`.
4. Wait for a settled cancel result.
5. If cancellation was merely requested, also wait for the target run to settle.
6. Sync token ledgers and append dossier execution entries.

### `CLOSE_SESSION`

Current path:

1. Build a close request.
2. Call ACP method `session/close`.
3. Wait for settlement.
4. Sync token ledgers.
5. Best-effort flush a semantic session summary into governance memory.
6. Reclaim owned terminals for the session.

This is the right shape for an agent harness.
The problem is not the existence of a broker. The problem is runtime brittleness and operator visibility.

## Broker Lifecycle and Auth

`handshake-acp-client.mjs` and `session-control-broker.mjs` show the broker model:

- auth token is stored in a repo-scoped local token file
- the broker is expected to match a specific build ID and auth mode
- broker readiness requires both state-file alignment and socket connectivity
- RPC uses a JSON-RPC style initialize handshake followed by method invocation
- broker status and shutdown are explicit orchestrator operations

The broker helper also encodes some safety logic:

- stale active runs can be detected from timeout expiry
- stale active runs can also be inferred from dead child processes
- broker shutdown includes graceful RPC then process-tree kill fallback if needed

## Recovery and Self-Settle Model

The kernel already contains a non-trivial recovery model in `session-control-self-settle-lib.mjs`.

When normal settlement is missing, it can infer recoverable results from:

- broker rejection events in output JSONL
- already-settled target commands for cancel flows
- terminal session-registry state
- absence of any surviving active broker run

That means the system already assumes that broker/result convergence is imperfect.
This is useful, but it is also evidence that the control plane is not yet robust enough to rely on normal completion alone.

## What the Tests Currently Prove

The shared tests currently give confidence in:

- prompt construction and startup/steering instructions
- profile resolution and profile-specific launch settings
- outcome classification such as `ALREADY_READY` and `BUSY_ACTIVE_RUN`

They do not, by themselves, prove runtime resilience under real host contention.
That gap matters because many of the painful failures are operational, not schema-level.

## Observed Weaknesses

### 1. Broker reliability under normal load

The lessons document already captures the recurring observation: the broker works in principle, but becomes brittle under real host load.
That means the kernel does not yet have production-grade control-plane resilience.

Key redesign question:

- what backpressure, heartbeat, and crash-recovery semantics are actually required before ACP is trusted for swarm coordination?

### 2. Session health is not obvious enough

There are health and activity helpers, but the operator still appears to learn about many failures by watching terminals or inspecting artifacts manually.

Key redesign question:

- what is the minimum push-based status and alerting model that makes session babysitting unnecessary?

### 3. Recovery is present but expensive

The presence of stall scans, orphan-terminal scans, reclaim flows, and self-settle helpers suggests the runtime already expects non-trivial repair scenarios.

That is not inherently bad.
The issue is whether these are rare safety nets or common-path workflow steps.

### 4. Command-surface drift can poison runtime control

If policy-generated commands do not match the real `just` surface, role startup and resume can fail even when the underlying work is fine.
This is a control-plane correctness problem, not a model-quality problem.

### 5. Recovery logic is a sign of maturity and a warning signal

Self-settle logic, terminal reclaim, stale-run pruning, and ready-session recovery are all useful.
But they also mean the common runtime path is expected to fragment.

Key redesign question:

- which repair paths should remain exceptional safety nets, and which ones are currently compensating for an unstable normal path?

## Evidence Anchors for This Topic

The first pass on this deep dive should repeatedly cite:

- `.GOV/roles_shared/scripts/session/session-policy.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles_shared/tests/session-control-lib.test.mjs`
- `.GOV/roles_shared/tests/session-governance-state-lib.test.mjs`
- `.GOV/roles_shared/tests/session-output-activity-lib.test.mjs`
- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
- `.GOV/roles_shared/scripts/session/handshake-acp-client.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-broker.mjs`
- `.GOV/roles_shared/checks/session-policy-check.mjs`
- `.GOV/roles_shared/checks/session-launch-runtime-check.mjs`
- `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
- `.GOV/Audits/smoketest/DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
- `.GOV/Audits/smoketest/DOSSIER_20260414_DISTILLATION_WORKFLOW_DOSSIER.md`

## What to Extract in the Next Pass

The next pass should turn this into a true mechanical deep dive by extracting:

- the session registry object model
- the broker state object model
- the launch queue and plugin handoff path
- the timeout and stale-run policies
- how model profiles affect runtime behavior and cost at the lane level
- where operator alerts are emitted and which ones are still terminal-only

## Architecture Requirements Emerging From This Topic

The next harness likely needs:

- durable session state independent of the chat
- first-class health signals and heartbeats
- push-based alerting outside the terminal
- backpressure instead of silent degradation
- one recovery model that supports both autonomous orchestration and manual relay
- strict command-surface conformance tests between policy and operational entrypoints

## Open Questions

1. Which failures are broker failures versus host/plugin failures versus workflow-state failures?
2. What is the minimum heartbeat model needed to detect silently dead sessions quickly?
3. Should session-control state remain file-ledger based, or move to a stronger state backend?
4. Which session-repair operations should stay manual even in a future swarm harness?
5. How should cost-aware provider routing interact with session launch policy?
