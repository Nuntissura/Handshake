# ROLE SESSION ORCHESTRATION

This file is the shared law for repo-governed multi-session launch behavior.

Default external repo-governance runtime root from a repo worktree: `../gov_runtime/roles_shared/`. This root may be overridden via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`.

## Core Rule
- Only the Orchestrator may start repo-governed Activation Manager, Coder, WP Validator, and Integration Validator sessions.
- Activation Manager, Coder, and Validator sessions may resume work, but they do not self-start a fresh repo-governed session.
- Only the Orchestrator may run fresh-start, close, cancel, or broker-stop control commands for governed role sessions. Coder and Validator sessions request repair, pause, or cancel actions through packet thread/receipt surfaces; they do not mutate the governed control ledgers directly.
- The Activation Manager is the mandatory governed pre-launch authoring lane for `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`. For `WORKFLOW_LANE=MANUAL_RELAY`, pre-launch remains Orchestrator-owned.

## Primary launch path
- Preferred host: `HANDSHAKE_ACP_BROKER`
- Compatibility launch queue: `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- Session registry: `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
- Launch/bootstrap only: governed command dispatch plus ACP settlement on the ordinary `AUTO` path.
- `AUTO` launch is headless/direct through ACP and should not open a visible system terminal on the ordinary path.
- Bridge compatibility path: `handshake.handshake-session-bridge`
- Bridge command: `handshakeSessionBridge.processLaunchQueue`
- The VS Code bridge queue remains an explicit compatibility/repair launch surface, not the ordinary `AUTO` path.

## Primary steering lane
- Control mode: `STEERABLE`
- Control transport: `CODEX_EXEC_RESUME_JSON`
- Control protocol: `HANDSHAKE_ACP_STDIO_V1`
- Control requests: `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- Control results: `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- Per-command event logs: `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/`
- Broker state: `../gov_runtime/roles_shared/SESSION_CONTROL_BROKER_STATE.json`
- Session steering is ACP-backed and thread-based: the Orchestrator starts a governed Codex thread once through the Handshake ACP broker, then resumes that same thread with governed prompts.
- A persistent Handshake ACP broker owns the active-run table, timeout settlement, and cancellation delivery for governed prompts. The wrapper client talks to that broker; it does not own command completion.
- Orchestrator-managed workflow uses these governed ACP/CLI sessions as the only normal delegation surface for the Activation Manager pre-launch lane plus the Coder and Validator lanes.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the ordinary order is Activation Manager first, then downstream Coder/Validator lanes after truthful `ACTIVATION_READINESS`. Do not bypass that pre-launch worker by keeping heavy activation reasoning in long-lived Orchestrator context.
- Helper agents/subagents may assist the Orchestrator on governance/spec/runtime/orchestrator tasks, but they are not Coder or Validator lanes.
- Do not use helper agents/subagents to perform Coder or Validator duties, and do not let them write product code, unless the Operator explicitly approved that path and the work packet records `SUB_AGENT_DELEGATION: ALLOWED` plus the exact `OPERATOR_APPROVAL_EVIDENCE`.
- `START_SESSION`, `SEND_PROMPT`, `CANCEL_SESSION`, and `CLOSE_SESSION` are first-class governed control commands. Cancel rows carry a target-command reference. Close rows clear the steerable thread registration for that governed role/WP session, settle through the same append-only request/result ledgers, and attempt deterministic reclaim of any governed system-terminal window owned by that exact session.
- governed `START_SESSION` / `SEND_PROMPT` results now carry an explicit `outcome_state` alongside terminal `status` so wrappers can distinguish steady-state conditions such as `ALREADY_READY`, `BUSY_ACTIVE_RUN`, or `REQUIRES_RECOVERY` from generic failures.
- The registry `session_thread_id` is the steering identity for that role/WP session.

## Fallback Law
- Primary launch path is ACP-direct headless.
- `VSCODE_PLUGIN` / `VSCODE` remain explicit compatibility hosts only.
- A CLI escalation window is repair-only and should be operator-directed, not the ordinary `AUTO` path.
- A CLI escalation window is allowed only after the same role/WP session has recorded 2 compatibility-host failures or timeouts.
- If compatibility-host instability reaches 2 failures across the governed batch, the session registry flips the batch into explicit CLI escalation mode for compatibility launches only until the batch is deliberately reset with `node .GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs "<reason>"`.
- Default escalation host: `SYSTEM_TERMINAL`
- Legacy compatibility: `WINDOWS_TERMINAL` is accepted as an older token, but new packets/protocol examples should use `SYSTEM_TERMINAL`.
- Manual `PRINT` output is a repair/debug surface, not the preferred runtime.
- Governed system-terminal launches must record terminal ownership in the session registry (`owned_terminal_process_id`, host kind, window title, recorded time, `owned_terminal_batch_id`) so closeout can reclaim only registry-owned governed windows from the intended governed batch.

## Wake-Up / Notice Protocol
- Primary wake channel: `VS_CODE_FILE_WATCH`
- Fallback wake channel: `WP_HEARTBEAT`
- Launch/bootstrap watch surfaces:
  - `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
- Steering watch/notice surfaces:
  - `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_BROKER_STATE.json`
- WP collaboration watch surfaces:
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/**/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/**/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/**/THREAD.md`
- The ACP broker owns the ordinary launch/bootstrap and steering state, result settlement, and per-command output logs. The VS Code bridge remains a compatibility launch surface plus operator-facing notice path when explicitly used.
- `just operator-viewport` is the canonical ACP-aware read-only operator viewport: it merges canonical task-board source/drift, broker status, session registry state, control results/output activity, packet thread/receipt activity, and packet/runtime visibility.
- `just operator-monitor` remains a compatibility alias.
- `just operator-admin` is the explicit admin-mode console for governed lifecycle actions. It remains non-authoritative and must invoke the same governed scripts the Orchestrator would run directly.
- Roles should not depend on blind continuous polling when a watch event exists.

## Deterministic State
- Launch requests are append-only JSONL records.
- Control requests and control results are append-only JSONL records.
- Per-command ACP event logs under the external repo-governance `SESSION_CONTROL_OUTPUTS/` directory are append-only detail surfaces for governed command execution, including cancel evidence and broker-settled output.
- The session registry is the current state projection for active and historical role sessions.
- The launch queue, control ledgers, broker state, output logs, and session registry are runtime artifacts. They are not packet/work-scope authority, and generic drive-agnostic scanning may treat them like operator evidence rather than normative governance text.
- Packet truth still wins over session state for scope, verdict, and acceptance.
- `TERMINAL_COMMAND_DISPATCHED` means the VS Code bridge created/reused a terminal and sent the governed command into it. It is not proof that the CLI session is alive yet.
- Treat packet-scoped receipts, runtime-state movement, or heartbeat evidence as the actual proof that the launched role session started executing.
- `READY` with a non-empty `session_thread_id` means a steerable Codex thread is registered and may be resumed through the governed control lane.
- `READY` is thread-registration proof, not by itself proof that packet-scoped WP communications are already live.
- `CLOSED` means the governed session record remains in the registry for audit, but its steerable thread registration has been intentionally cleared. A fresh `START_SESSION` is required before steering may resume.
- Heartbeat is liveness only. `validator_trigger` is a validator wake signal only. Neither one is a steering channel.
- Receipt/notification progress is the steering channel. If a governed next-actor route crosses `heartbeat_due_at` or `stale_after` without receipt progress, treat it as a relay-health signal, not as evidence that the route changed by itself.
- Automatic relay repair is bounded. Successful non-LLM re-steers consume `current_relay_escalation_cycle`, healthy routes reset it, and once `max_relay_escalation_cycles` is exhausted the lane must remain attention-visible until a fresh orchestrator decision or later repair rung intervenes.
- Direct worker interruption is stricter. Any watchdog-driven `CANCEL_SESSION` attempt consumes `current_worker_interrupt_cycle` against `max_worker_interrupt_cycles`, and restart is forbidden unless the lane verdict explicitly allows `BOUNDED_AFTER_ROUTE_REPAIR`.
- One governed role/WP session has at most one active ACP run at a time. Concurrent steering for the same governed session is not allowed.

## Session Model Policy
- Primary model: `gpt-5.4`
- Fallback model: `gpt-5.2`
- Reasoning strength: `EXTRA_HIGH`
- Launcher config: `model_reasoning_effort=xhigh`
- Codex model aliases are not allowed in new repo-governed claim fields.

## Direct-Review Contract (Current Law)

- Applies to orchestrator-managed packets that declare packet-scoped communication surfaces under `../gov_runtime/roles_shared/WP_COMMUNICATIONS/`.
- `THREAD.md` is soft coordination only. It does not satisfy a missing structured direct-review boundary.
- `correlation_id` opens one governed review/request chain.
- `ack_for` closes or answers that chain and must point back to the opener's `correlation_id`. Matching only the reply-side `correlation_id` is insufficient.
- `target_session` is required whenever the direct-review boundary is session-targeted between `CODER`, `WP_VALIDATOR`, and `INTEGRATION_VALIDATOR`.
- Receipt pairing must preserve reversed role plus session continuity. Mixed-session chains do not satisfy the boundary even if the receipt kinds look correct.
- Notification unread state and acknowledgment are session-scoped for session-targeted review traffic. Acknowledging one governed session must not clear another session's unread boundary notifications.
- In orchestrator-managed lanes, validator-authored assessment receipts also emit an `ORCHESTRATOR` governance-checkpoint notification so workflow authority can verify packet/runtime/task-board truth and ACP steering after each assessment without becoming the message broker.
- Resume helpers should follow the projected receipt/runtime route first and only fall back to legacy packet wording when no governed communication state exists. Do not let stale "handoff marker" heuristics override live `next_expected_actor` / `waiting_on` truth.

Required review pairs:
- `KICKOFF`: `VALIDATOR_KICKOFF` -> `CODER_INTENT`
- `HANDOFF`: `CODER_HANDOFF` -> `VALIDATOR_REVIEW`
- `VERDICT`: for `PACKET_FORMAT_VERSION >= 2026-03-22`, one direct coder <-> integration-validator review pair must exist before final verdict clearance
- Before PASS commit clearance in the orchestrator-managed final lane, run `just phase-check CLOSEOUT WP-{ID}`. If it fails, final review is not closeout-ready: do not write partial closure truth, do not compensate with narrative repair, and fix the topology/runtime issue first.

Blocking rule:
- If `just phase-check <STARTUP|HANDOFF|VERDICT|CLOSEOUT> WP-{ID} ...` or the underlying `just wp-communication-health-check WP-{ID} KICKOFF|HANDOFF|VERDICT` fails, treat the boundary as not proven. Do not compensate with narrative relay or manual interpretation.

## Session-Control Repair Playbook (Shared)

Use these rules when governed runtime/session truth drifts or looks stale.

- If the packet is obsolete, terminal, superseded, or blocked by legacy remediation policy, do not resume or steer the old governed session. Close or retire the stale session projection instead.
- If the assigned worktree no longer exists on disk, do not resume the governed session just because it still has a thread id. Repair the worktree/packet truth first or recreate the session through the Orchestrator.
- If broker state looks stale, compare `just handshake-acp-broker-status` with `just session-registry-status` and packet/runtime truth before acting. Use `just handshake-acp-broker-stop` only when no governed runs are active.
- Broker startup and the governed `session-*` helpers now run a recoverable self-settlement pass for missing terminal result rows. If an old request was rejected, already terminal in the session registry, or left without an active broker run, prefer the governed helpers over manual ledger edits and let runtime truth converge first.
- When the broker sees a same-session `active_run`, it should first prune or recover obviously stale runs (already-settled row, dead child process, or expired timeout) before surfacing `BUSY_ACTIVE_RUN`. A repeated launch should mean a truly live competing run, not stale broker residue.
- Before refusing a broker restart because `active_runs` still exist, the ACP client should first prune/self-settle recoverable broker-state residue. Only genuinely live active runs should block a restart.
- If packet communication routing looks wrong, run `just wp-communication-health-check`, `just check-notifications` (add `--history` only when you need suppressed terminal or superseded residue), and `just ack-notifications` with the explicit role/session identity before considering any deeper repair.
- Do not hand-edit session-control ledgers, broker state, packet receipts, or packet notifications to "unstick" a session. Prefer the governed helpers or a controlled session close/recreate flow.
- If a governed session launched through a system terminal remains open after closeout, use `just session-reclaim-terminals WP-{ID} [ROLE] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]` instead of killing terminals by guesswork.
- If session/runtime truth disagrees with packet truth, packet truth still wins for scope, verdict, and acceptance. Repair the runtime projection; do not rewrite packet truth to match stale runtime state.
- `PRINT` launch output is a repair/debug surface only. It is not proof that a governed session is healthy or resumable.

## Parallel Session Constraints (Current Law)

- One governed role/WP session has at most one active ACP run at a time.
- The ordinary orchestrator-managed WP shape is one governed `CODER` lane plus one governed `WP_VALIDATOR` lane, with `INTEGRATION_VALIDATOR` joining from `handshake_main` only for final validation/closure when required.
- `INTEGRATION_VALIDATOR` joins from `handshake_main` for product authority only. Its live governance root must still resolve to `wt-gov-kernel/.GOV` through `HANDSHAKE_GOV_ROOT`; `handshake_main/.GOV` is a local backup copy, not the authoritative governance source for orchestrator-managed work.
- Packet-scoped direct review is session-targeted. Role identity alone is not enough once multiple governed sessions may exist in the batch.
- The Orchestrator may run multiple governed sessions in parallel across different WPs, but it must not create parallel steerable lanes that collapse authority for the same role/WP pair.
- If the repo is in an exceptional repair state with extra same-role sessions around one WP, only the governed role/WP lane tracked by the session registry and packet communications is authoritative.

## Operational Commands
- Orchestrator-only launch/bootstrap commands:
  - `just launch-activation-manager-session WP-{ID}`
  - `just launch-coder-session WP-{ID}`
  - `just launch-wp-validator-session WP-{ID}`
  - `just launch-integration-validator-session WP-{ID}`
- normal supported launch paths now auto-issue the first governed `START_SESSION`; keep `start-*` for explicit recovery or exceptional manual repair
- when a direct `START_SESSION` finds an already steerable thread, it should settle as `outcome_state=ALREADY_READY` rather than a generic wrapper failure.
- when `START_SESSION` reports `BUSY_ACTIVE_RUN` or `REQUIRES_RECOVERY`, the wrapper should still wait briefly for the session to become READY before failing. The common case is a same-attempt convergence race, not an Operator-worthy second launch.
- Orchestrator-only steering commands:
  - `just start-activation-manager-session WP-{ID}`
  - `just steer-activation-manager-session WP-{ID} "<prompt>"`
  - `just cancel-activation-manager-session WP-{ID}`
  - `just close-activation-manager-session WP-{ID}`
  - `just start-coder-session WP-{ID}`
  - `just start-wp-validator-session WP-{ID}`
  - `just start-integration-validator-session WP-{ID}`
  - `just steer-coder-session WP-{ID} "<prompt>"`
  - `just cancel-coder-session WP-{ID}`
  - `just close-coder-session WP-{ID}`
  - `just steer-wp-validator-session WP-{ID} "<prompt>"`
  - `just cancel-wp-validator-session WP-{ID}`
  - `just close-wp-validator-session WP-{ID}`
  - `just steer-integration-validator-session WP-{ID} "<prompt>"`
  - `just cancel-integration-validator-session WP-{ID}`
  - `just close-integration-validator-session WP-{ID}`
  - `just session-start <ROLE> WP-{ID}`
  - `just session-send <ROLE> WP-{ID} "<prompt>"`
  - `just session-cancel <ROLE> WP-{ID}`
  - `just session-close <ROLE> WP-{ID}`
  - `just session-reclaim-terminals WP-{ID} [ROLE] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]`
  - `just handshake-acp-broker-status`
  - `just handshake-acp-broker-stop`
- `just session-registry-status [WP-{ID}]`
- `just active-lane-brief <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> WP-{ID} [--json]`
- Activation Manager uses `just activation-manager next WP-{ID}` as its compact context digest instead of `active-lane-brief`.
- `just orchestrator-steer-next WP-{ID} "<context>" [PRIMARY|FALLBACK]`
- `just wp-relay-watchdog WP-{ID} --observe-only`
- `just operator-viewport`
- `just operator-admin`
- When a WP filter is supplied, `just session-registry-status` now prints derived relay escalation state.
- The relay-escalation block also prints the runtime relay-cycle budget, and watchdog observe-only output now prints the worker-interrupt budget, so bounded auto-repair can be inspected without opening the runtime JSON directly.
- `just wp-relay-watchdog --loop` is service-hardened for long runs: per-WP evaluation failures are surfaced inline, but the watcher continues scanning other WPs and later cycles instead of exiting on the first bad lane.
- `just active-lane-brief` is the compact authority digest for one governed lane; prefer it over rereading packet/runtime/session surfaces separately.
- If derived relay escalation is `ESCALATED`, use `just orchestrator-steer-next WP-{ID} "<context>"` instead of waiting silently.
- Prefer `just wp-relay-watchdog WP-{ID} --observe-only` before waking a lane when you need a mechanical "is this actually stalled?" verdict without disturbing a still-working active run.
