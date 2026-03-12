# VS Code Session Bridge Architecture

Date: 2026-03-12
Status: Active governance note for the plugin-first session orchestration lane
Scope: New stubs and task packets created under `PACKET_FORMAT_VERSION=2026-03-12` and `STUB_FORMAT_VERSION=2026-03-12`

## Purpose

This document explains why the Handshake repo uses a thin VS Code extension bridge for repo-governed role sessions, how that bridge works with repo governance files, and where the authority boundaries live.

The core goal is deterministic multi-session execution without moving workflow authority into terminal buffers, ad hoc chat, or the extension itself.

## Intent

Handshake is moving toward parallel WP execution with isolated worktrees and explicit role sessions:

- Orchestrator starts work.
- Primary Coder owns implementation and coder-side paperwork.
- WP Validator reviews and steers the WP during execution.
- Integration Validator owns final technical verdict, merge authority, and canonical integration actions.

The bridge exists so the Orchestrator can start those CLI sessions inside VS Code integrated terminals in a governed way.

## Why A Plugin Exists

The repo intentionally does not let the extension invent policy. Policy stays in repo law and scripts.

The extension exists because the official VS Code extension API can deterministically create terminals and send command text, while the documented `code` CLI is focused on opening windows, folders, files, URLs, and extension management rather than "open an integrated terminal in this workspace and run this exact command".

That leads to the design choice:

- Repo scripts decide who may launch, which worktree to use, which model to use, and when CLI fallback is legal.
- The VS Code bridge only transports the already-governed command into an integrated terminal.

## Why Not Use VS Code Tasks As The Primary Runtime

VS Code tasks are useful, but they are a weaker fit for Handshake's workflow:

- WP creation is dynamic.
- Multiple WPs may run concurrently.
- Session names and worktree roots are packet-specific.
- The Orchestrator must target Coder, WP Validator, and Integration Validator separately.

Tasks are acceptable as a local convenience layer, but not as the primary authority surface for session orchestration.

## Governance Boundary

The bridge is transport only. It does not:

- choose work
- choose models
- mutate packets
- assign merge authority
- override validator verdicts
- interpret terminal output as source of truth

The authoritative layers remain:

- packet: scope, acceptance, lifecycle, and final packet truth
- refinement: activation and spec grounding
- `WP_COMMUNICATIONS`: deterministic role-to-role collaboration artifacts
- session registry: launch and runtime projection only

If packet and runtime/session artifacts disagree, the packet wins.

## Current Repo Law

The current shared law for this system lives in:

- `AGENTS.md`
- `.GOV/roles_shared/ROLE_SESSION_ORCHESTRATION.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/coder/CODER_PROTOCOL.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/scripts/session-policy.mjs`

Current law encoded there:

- Only the Orchestrator may start repo-governed Coder, WP Validator, and Integration Validator sessions.
- Preferred host is `VSCODE_EXTENSION_TERMINAL`.
- Launch is plugin-first.
- CLI escalation is allowed only after 2 plugin failures or timeouts for the same role/WP session, unless the Operator explicitly waives the plugin-first path.
- Primary wake channel is file-watch based.
- Heartbeat remains fallback, not primary orchestration.

## Session Model Policy

Repo-governed sessions currently use:

- primary model: `gpt-5.4`
- fallback model: `gpt-5.2`
- required reasoning strength: `EXTRA_HIGH`
- launcher config: `model_reasoning_effort=xhigh`

Important nuance:

- The repo uses the `codex` CLI tool as the local transport executable.
- The repo does not allow Codex model aliases as selected model values in new governed session metadata.
- In other words, `codex` is the launcher binary, not the model policy.

## Files And Their Roles

### Launch Queue

`/.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`

- append-only queue
- one JSON object per requested role session launch
- authored by repo scripts under Orchestrator authority
- consumed by the VS Code bridge

### Session Registry

`/.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`

- current-state projection
- records plugin attempts, runtime posture, active terminal identity, and escalation readiness
- not authoritative for packet scope or acceptance

### WP Communication Surface

`/.GOV/roles_shared/WP_COMMUNICATIONS/WP-{ID}/`

- `THREAD.md`: append-only freeform discussion
- `RUNTIME_STATUS.json`: machine-readable liveness, wait state, next actor, validator wake state
- `RECEIPTS.jsonl`: deterministic receipts for assignment, status, steering, repair, validation, and handoff

### Bridge Implementation

`/.GOV/tools/vscode-session-bridge/`

- `package.json`: VS Code extension manifest
- `extension.js`: launch queue consumer and runtime-status watcher
- `README.md`: local quickstart for extension development host usage

### Launch/Policy Scripts

- `/.GOV/scripts/session-policy.mjs`
- `/.GOV/scripts/session-registry-lib.mjs`
- `/.GOV/scripts/launch-cli-session.mjs`
- `/.GOV/scripts/validation/session-launch-runtime-check.mjs`

## End-To-End Flow

1. The Orchestrator creates or activates a WP.
2. Repo scripts prepare the correct worktree, branch, role brief, startup command, and follow-up command.
3. `launch-cli-session.mjs` writes a governed launch request into `SESSION_LAUNCH_REQUESTS.jsonl`.
4. The VS Code bridge watches the queue file.
5. The bridge validates the request shape and authority.
6. The bridge creates or reuses a named integrated terminal.
7. The bridge sends the exact governed CLI command into that terminal.
8. The bridge writes plugin confirmation or failure into `ROLE_SESSION_REGISTRY.json`.
9. Runtime and collaboration continue through the packet and `WP_COMMUNICATIONS` files.
10. If the plugin path fails twice or times out twice for the same role/WP session, CLI escalation becomes legal.

## Deterministic Session Rules

Per governed session:

- one role
- one WP
- one worktree
- one session key
- one named terminal title
- one governed model choice

This keeps concurrent WPs isolated and reduces merge conflict pressure.

## Current Technical Implementation

The current bridge implementation in `extension.js` does the following:

- resolves repo root from the active workspace
- watches `SESSION_LAUNCH_REQUESTS.jsonl`
- parses JSONL launch records
- validates required fields such as:
  - `schema_id`
  - `schema_version`
  - `launch_authority`
  - `preferred_host`
  - `abs_worktree_dir`
  - `command`
- creates or reuses a terminal using `vscode.window.createTerminal(...)`
- injects the already-governed launch command via `terminal.sendText(...)`
- updates `ROLE_SESSION_REGISTRY.json`
- watches `WP_COMMUNICATIONS/**/RUNTIME_STATUS.json`
- shows validator wake-up notices when `validator_trigger` becomes non-empty

The current launcher implementation in `launch-cli-session.mjs` does the following:

- asserts current branch is `role_orchestrator`
- ensures the target role worktree exists
- chooses role-specific branch, worktree path, terminal title, startup command, and next command
- enforces the allowed model set:
  - `gpt-5.4`
  - `gpt-5.2`
- injects `model_reasoning_effort="xhigh"`
- blocks CLI fallback before the retry budget is exhausted
- records queueing, plugin result, timeout settlement, and CLI escalation use in the registry

## Wake-Up And Monitoring Model

Primary wake-up is event-driven:

- session launch queue file watch
- `RUNTIME_STATUS.json` file watch

Fallback wake-up is heartbeat-driven:

- session heartbeat and runtime polling remain fallback safety nets

Operator overview is separate from role execution:

- `just operator-monitor` is the oversight surface
- terminal output is not the authoritative audit trail

## Current Limitations

This design is active, but still intentionally thin.

Current limitations:

- The extension is scaffolded in-repo, but still must be installed or run in a VS Code Extension Development Host locally.
- The bridge currently watches launch requests and runtime status, but it does not yet parse or notify on every `THREAD.md` append.
- The bridge currently uses `terminal.sendText(...)`. A future upgrade can prefer shell-integration-backed command execution when available and fall back to `sendText`.
- Older packets and stubs created before the `2026-03-12` format boundary may not fully comply with the new session policy and may require manual relay handling.

## Why This Design Is Better Than A Pure CLI Launcher

Benefits:

- keeps sessions inside one VS Code host instead of spraying extra external windows by default
- preserves worktree isolation per WP
- keeps launch authority deterministic
- allows file-watch-based wake-ups
- keeps repo files authoritative
- supports multiple concurrent role sessions with named terminals

Costs:

- adds extension maintenance
- requires a local extension install/dev-host workflow
- introduces one more moving part between queue and terminal creation

The tradeoff is acceptable because the extension is intentionally small and does not own repo policy.

## Why This Design Is Better Than Generic Multi-Agent Framework Adoption

Handshake deliberately borrows patterns from modern parallel-agent systems without turning the repo into a generic agent framework runtime.

Patterns worth stealing:

- isolated worktree per session
- persistent named runtime per work item
- event-driven state transitions
- durable state projection
- human interrupt and validator gates

Patterns intentionally rejected:

- moving execution authority into a framework-owned state machine
- letting a UI or terminal runtime become the primary workflow authority
- replacing repo-governed packets and receipts with opaque agent memory

## Suggested Future Improvements

- Add explicit notifications for `THREAD.md` steering messages when packet-scoped freeform relay needs attention.
- Add a small tree view or status bar surface for live session registry state.
- Add structured "wake intent" rows if validator and coder wake-up rules grow beyond `RUNTIME_STATUS.json`.
- Prefer shell-integration `executeCommand(...)` when available so the bridge can observe command execution more cleanly.
- Add operator-facing recovery actions for "requeue launch", "force open registry", and "open worktree".

## Research Basis

The current design was chosen after comparing official VS Code and OpenAI docs with active GitHub projects that manage parallel coding contexts.

### Official VS Code Sources

- VS Code Command Line Interface: https://code.visualstudio.com/docs/configure/command-line
  - why it matters: confirms the `code` CLI is focused on opening windows, files, folders, URLs, and extension management, not deterministic integrated-terminal command injection
- VS Code Terminal Basics: https://code.visualstudio.com/docs/terminal/basics
  - why it matters: confirms integrated terminal behavior and shell-integration concepts
- VS Code Terminal Shell Integration: https://code.visualstudio.com/docs/terminal/shell-integration
  - why it matters: supports a future move from raw `sendText` toward shell-integration-aware execution where available
- VS Code Extension API: https://code.visualstudio.com/api/references/vscode-api
  - why it matters: documents `createTerminal(...)`, `sendText(...)`, file watchers, and shell-integration execution APIs

### Official OpenAI Sources

- OpenAI Codex CLI docs: https://developers.openai.com/codex/cli
  - why it matters: confirms the CLI runtime surface used by the repo launcher
- OpenAI Codex CLI config docs: https://developers.openai.com/codex/cli/config
  - why it matters: supports explicit CLI configuration patterns
- OpenAI models docs: https://platform.openai.com/docs/models
  - why it matters: grounds model selection as an explicit launcher concern
- OpenAI code generation guide: https://developers.openai.com/api/docs/guides/code-generation
  - why it matters: supports using OpenAI models in coding interfaces while keeping model choice explicit
- OpenAI reasoning best practices: https://developers.openai.com/api/docs/guides/reasoning-best-practices
  - why it matters: supports taking reasoning configuration seriously as part of session quality and continuity

### GitHub Architecture References

- `coplane/par`: https://github.com/coplane/par
  - takeaway: strong validation for isolated worktrees, persistent sessions, global overview, and IDE integration
- `microsoft/autogen`: https://github.com/microsoft/autogen
  - takeaway: message-passing and event-driven orchestration patterns are useful, but Handshake should keep repo files authoritative
- LangGraph durable execution docs: https://docs.langchain.com/oss/python/langgraph/durable-execution
  - takeaway: durable state plus resumability is the right mental model for governance gates and validator interrupts
- LangGraph overview docs: https://docs.langchain.com/oss/python/langgraph
  - takeaway: human-in-the-loop and durable orchestration are first-class concerns in serious agent systems
- OpenHands CLI: https://openhands.dev/product/cli
  - takeaway: execution runtime, UI, and oversight surfaces should remain separate layers

## Decision Summary

Handshake should use:

- repo-governed packets and receipts as authority
- plugin-first integrated-terminal launch
- file-watch wake-ups
- heartbeat fallback
- strict Orchestrator-only session start authority
- validator and integration-validator role separation
- explicit model and reasoning policy

Handshake should not use:

- undocumented VS Code CLI hacks as the main runtime
- terminal buffers as authority
- generic agent frameworks as the workflow governor
- Codex model aliases as governed model selections

## Posterity Note

This bridge is intentionally small. If it ever starts choosing work, mutating packets, inferring merge authority, or replacing repo law with extension-local behavior, it has drifted out of bounds and should be reduced back to a transport layer.
