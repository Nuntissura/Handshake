# ROLE SESSION ORCHESTRATION

This file is the shared law for repo-governed multi-session launch behavior.

## Core Rule
- Only the Orchestrator may start repo-governed Coder, WP Validator, and Integration Validator sessions.
- Coder and Validator sessions may resume work, but they do not self-start a fresh repo-governed session.
- Only the Orchestrator may run fresh-start, close, cancel, or broker-stop control commands for governed role sessions. Coder and Validator sessions request repair, pause, or cancel actions through packet thread/receipt surfaces; they do not mutate the governed control ledgers directly.

## Primary launch path
- Preferred host: `VSCODE_EXTENSION_TERMINAL`
- Bridge: `handshake.handshake-session-bridge`
- Bridge command: `handshakeSessionBridge.processLaunchQueue`
- Launch queue: `.GOV/roles_shared/runtime/SESSION_LAUNCH_REQUESTS.jsonl`
- Session registry: `.GOV/roles_shared/runtime/ROLE_SESSION_REGISTRY.json`
- Launch/bootstrap only: terminal creation, governed command dispatch, and bridge acknowledgment/failure projection.

## Primary steering lane
- Control mode: `STEERABLE`
- Control transport: `CODEX_EXEC_RESUME_JSON`
- Control protocol: `HANDSHAKE_ACP_STDIO_V1`
- Control requests: `.GOV/roles_shared/runtime/SESSION_CONTROL_REQUESTS.jsonl`
- Control results: `.GOV/roles_shared/runtime/SESSION_CONTROL_RESULTS.jsonl`
- Per-command event logs: `.GOV/roles_shared/runtime/SESSION_CONTROL_OUTPUTS/`
- Broker state: `.GOV/roles_shared/runtime/SESSION_CONTROL_BROKER_STATE.json`
- Session steering is ACP-backed and thread-based: the Orchestrator starts a governed Codex thread once through the Handshake ACP bridge, then resumes that same thread with governed prompts.
- A persistent Handshake ACP broker owns the active-run table, timeout settlement, and cancellation delivery for governed prompts. The wrapper client talks to that broker; it does not own command completion.
- `START_SESSION`, `SEND_PROMPT`, `CANCEL_SESSION`, and `CLOSE_SESSION` are first-class governed control commands. Cancel rows carry a target-command reference. Close rows clear the steerable thread registration for that governed role/WP session and settle through the same append-only request/result ledgers.
- The registry `session_thread_id` is the steering identity for that role/WP session.

## Fallback Law
- Primary launch path is plugin-first.
- A CLI escalation window is allowed only after the same role/WP session has recorded 2 plugin failures or timeouts.
- Default escalation host: `SYSTEM_TERMINAL`
- Legacy compatibility: `WINDOWS_TERMINAL` is accepted as an older token, but new packets/protocol examples should use `SYSTEM_TERMINAL`.
- Manual `PRINT` output is a repair/debug surface, not the preferred runtime.

## Wake-Up / Notice Protocol
- Primary wake channel: `VS_CODE_FILE_WATCH`
- Fallback wake channel: `WP_HEARTBEAT`
- Launch/bootstrap watch surfaces:
  - `.GOV/roles_shared/runtime/SESSION_LAUNCH_REQUESTS.jsonl`
  - `.GOV/roles_shared/runtime/ROLE_SESSION_REGISTRY.json`
- Steering watch/notice surfaces:
  - `.GOV/roles_shared/runtime/SESSION_CONTROL_REQUESTS.jsonl`
  - `.GOV/roles_shared/runtime/SESSION_CONTROL_RESULTS.jsonl`
  - `.GOV/roles_shared/runtime/SESSION_CONTROL_OUTPUTS/`
  - `.GOV/roles_shared/runtime/SESSION_CONTROL_BROKER_STATE.json`
- WP collaboration watch surfaces:
  - `.GOV/roles_shared/runtime/WP_COMMUNICATIONS/**/RUNTIME_STATUS.json`
  - `.GOV/roles_shared/runtime/WP_COMMUNICATIONS/**/RECEIPTS.jsonl`
  - `.GOV/roles_shared/runtime/WP_COMMUNICATIONS/**/THREAD.md`
- The VS Code bridge handles launch/bootstrap dispatch plus operator-facing notices. The ACP broker owns steering state, result settlement, and per-command output logs.
- `just operator-monitor` is the ACP-aware read-only operator viewport: it merges canonical task-board source/drift, broker status, session registry state, control results/output activity, packet thread/receipt activity, and packet/runtime visibility.
- `just operator-admin` is the explicit admin-mode console for governed lifecycle actions. It remains non-authoritative and must invoke the same governed scripts the Orchestrator would run directly.
- Roles should not depend on blind continuous polling when a watch event exists.

## Deterministic State
- Launch requests are append-only JSONL records.
- Control requests and control results are append-only JSONL records.
- Per-command ACP event logs under `.GOV/roles_shared/runtime/SESSION_CONTROL_OUTPUTS/` are append-only detail surfaces for governed command execution, including cancel evidence and broker-settled output.
- The session registry is the current state projection for active and historical role sessions.
- The launch queue, control ledgers, broker state, output logs, and session registry are runtime artifacts. They are not packet/work-scope authority, and generic drive-agnostic scanning may treat them like operator evidence rather than normative governance text.
- Packet truth still wins over session state for scope, verdict, and acceptance.
- `TERMINAL_COMMAND_DISPATCHED` means the VS Code bridge created/reused a terminal and sent the governed command into it. It is not proof that the CLI session is alive yet.
- Treat packet-scoped receipts, runtime-state movement, or heartbeat evidence as the actual proof that the launched role session started executing.
- `READY` with a non-empty `session_thread_id` means a steerable Codex thread is registered and may be resumed through the governed control lane.
- `READY` is thread-registration proof, not by itself proof that packet-scoped WP communications are already live.
- `CLOSED` means the governed session record remains in the registry for audit, but its steerable thread registration has been intentionally cleared. A fresh `START_SESSION` is required before steering may resume.
- Heartbeat is liveness only. `validator_trigger` is a validator wake signal only. Neither one is a steering channel.
- One governed role/WP session has at most one active ACP run at a time. Concurrent steering for the same governed session is not allowed.

## Session Model Policy
- Primary model: `gpt-5.4`
- Fallback model: `gpt-5.2`
- Reasoning strength: `EXTRA_HIGH`
- Launcher config: `model_reasoning_effort=xhigh`
- Codex model aliases are not allowed in new repo-governed claim fields.

## Operational Commands
- Orchestrator-only launch/bootstrap commands:
  - `just launch-coder-session WP-{ID}`
  - `just launch-wp-validator-session WP-{ID}`
  - `just launch-integration-validator-session WP-{ID}`
- Orchestrator-only steering commands:
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
  - `just handshake-acp-broker-status`
  - `just handshake-acp-broker-stop`
- `just session-registry-status [WP-{ID}]`
- `just operator-monitor`
- `just operator-admin`
- `just handshake-acp-bridge`
