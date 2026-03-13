# Governed Session Control Architecture

Date: 2026-03-13
Status: Active governance note for the ACP-first control lane, with the VS Code bridge reduced to launch and viewport support
Scope: Governance-only session orchestration under `.GOV/**`, `.github/**`, `justfile`, `AGENTS.md`, and repo workflow docs. No product-code changes are part of this architecture.

## Quick Answer

### Does Codex itself need changes?

Usually no.

This workflow does not require a forked Codex client or product-code integration. The governance layer wraps the existing `codex` CLI and uses its JSON/threaded execution surface.

What must already work locally:

- `codex exec --json`
- `codex exec resume <thread_id> --json`

What changed is the repo governance around Codex:

- governed wrapper commands in `.GOV/scripts`
- a persistent ACP-style broker in `.GOV/tools/handshake-acp-bridge/`
- append-only request/result/output ledgers
- an operator viewport that reads those ledgers

So the answer is:

- no new Codex product patch is required for the current Handshake workflow
- yes, the local `codex` CLI must support the JSON + resume features the governance broker uses

### New Workflow: Get Started

From the Orchestrator worktree:

1. Start or confirm the role sessions:
   - `just start-coder-session WP-{ID}`
   - `just start-wp-validator-session WP-{ID}`
2. Inspect governed state:
   - `just session-registry-status WP-{ID}`
   - `just operator-monitor`
3. Steer work through governed prompts:
   - `just steer-coder-session WP-{ID} "<prompt>"`
   - `just steer-wp-validator-session WP-{ID} "<prompt>"`
4. Cancel a governed run when needed:
   - `just cancel-coder-session WP-{ID}`
   - `just cancel-wp-validator-session WP-{ID}`
5. Use packet communications for human/role collaboration:
   - `just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]`

Expected operator model:

- launch/bootstrap may still involve the VS Code bridge
- steering is ACP-backed and repo-governed
- the TUI is the viewport, not the control authority

### Old Workflow vs New Workflow

Old workflow:

- Orchestrator queued launch text for the VS Code terminal bridge
- bridge created a terminal and injected the command
- follow-up confidence depended on registry updates plus WP communications
- steering was indirect and weak

New workflow:

- Orchestrator starts a governed session once and gets a stable thread identity
- later steering resumes that same governed session through the ACP-style broker
- every governed command writes request/result/output artifacts
- cancel is first-class and auditable
- operator monitoring sees canonical board state plus governed control activity

Short version:

- old = launch-oriented, terminal-dispatch-first
- new = session-oriented, governed-control-first

## Purpose

This note records the updated direction for repo-governed multi-session work in Handshake.

The previous bridge design solved terminal dispatch. It did not solve real session control. The live smoke test exposed the gap:

- terminal dispatch was observable
- role-session liveness was not reliably proven
- follow-up steering depended on repo scripts, not a session protocol
- the operator TUI could see packet/runtime surfaces, but not the governed control stream clearly

The new target is ACP-first session control with repo-governed projections. The bridge and the TUI stay non-authoritative. Packets, traceability, task board, and WP communications remain authoritative.

## Research Basis

This design is based on the current shape of public ACP and multi-agent implementations:

- ACP official docs: stateful client-agent session protocol, explicit session methods, streamed updates
- `cola-io/codex-acp`: ACP adapter for Codex with `session/new`, `session/load`, `session/prompt`, `session/cancel`, and streamed updates
- `zed-industries/claude-agent-acp`: ACP agent with background terminals, tool calls, slash commands, and ACP-compatible client support
- `glittercowboy/get-shit-done`: thin orchestrator plus parallel worker waves, but not a session-control plane
- `fstandhartinger/ralph-wiggum`: iterative opaque worker loop with disk state and retry, but not live steerable sessions

Conclusion:

- Ralph and GSD are useful worker patterns.
- They do not replace a steerable session-control plane.
- Handshake needs ACP-style session identity, prompt steering, updates, and cancellation semantics, while keeping repo governance authoritative.

## Core Decision

Handshake adopts an ACP-first control model inside governance.

That means:

- session control is no longer defined as "send text into a terminal"
- the primary control transport becomes a governed ACP adapter under `.GOV/tools`
- repo scripts remain the authority layer that decides who may start, steer, cancel, or inspect a session
- the TUI remains a viewport only

This is not a `.bat` file design. The deliverable is a governance-owned tool package plus repo scripts, ledgers, and viewport updates.

## Authority Boundaries

The following artifacts remain authoritative:

- active packet or stub: scope, lifecycle, acceptance, and final packet truth
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`: active artifact mapping per base WP
- `.GOV/roles_shared/TASK_BOARD.md` on canonical branch `main`: canonical portfolio board
- packet-declared `.GOV/roles_shared/WP_COMMUNICATIONS/WP-{ID}/`
  - `THREAD.md`
  - `RUNTIME_STATUS.json`
  - `RECEIPTS.jsonl`

The following artifacts are projections, not work-scope authority:

- `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- `.GOV/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/`
- ACP adapter runtime metadata under `.GOV/tools/handshake-acp-bridge/`

If packet truth and session-control artifacts disagree, the packet wins.

## High-Level Architecture

### 1. Governance Layer

Lives in `.GOV/scripts`.

Responsibilities:

- enforce Orchestrator-only start authority
- enforce packet/worktree/branch/model policy
- write append-only governed requests and results
- project current state into the session registry
- decide fallback/escalation rules
- expose stable `just` commands

Representative files:

- `.GOV/scripts/session-policy.mjs`
- `.GOV/scripts/session-registry-lib.mjs`
- `.GOV/scripts/session-control-lib.mjs`
- `.GOV/scripts/session-control-command.mjs`

### 2. ACP Adapter Layer

Lives in a new sibling tool package under:

- `.GOV/tools/handshake-acp-bridge/`

Responsibilities:

- implement an ACP-compatible session surface
- bridge governed session requests to Codex session execution
- preserve stable session identity
- stream session updates back to the caller
- expose cancellation and inspection semantics
- run as a persistent broker so active runs, timeouts, and cancels are owned by one governed process instead of per-call wrappers
- require a repo-scoped auth token plus `ORCHESTRATOR` / `role_orchestrator` initialization claims before any non-bootstrap ACP method is accepted
- publish broker build/auth identity so governed clients can reject stale broker instances after governance changes

This layer is transport and runtime control, not policy.

### 3. Launch / Notification Layer

Lives in:

- `.GOV/tools/vscode-session-bridge/`

Responsibilities:

- bootstrap or notify
- optionally host ACP clients in editor workflows
- surface file-watch notices
- never become policy or scope authority

The VS Code bridge is demoted from "primary runtime control plane" to "launch/bootstrap plus notification helper."

### 4. Operator Viewport

Lives in:

- `.GOV/scripts/operator-monitor-tui.mjs`

Responsibilities:

- render the board, packets, session projections, receipts, thread entries, and ACP-derived projections
- stay read-only except for existing governed thread append behavior
- never talk directly to ACP as an authoritative channel

The TUI reads repo projections. It does not own orchestration.

## Primary Session Surfaces

### Canonical Governance Surfaces

- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/task_packets/`
- `.GOV/task_packets/stubs/`
- packet-declared `WP_COMMUNICATIONS`

### Governed Session-Control Surfaces

- `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- `.GOV/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/`

These remain the repo audit trail for ACP-backed control.

### Legacy Launch Surface

- `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`

This becomes compatibility and bootstrap only. It is no longer the preferred steering surface.

## ACP Method Model

The governed ACP adapter should expose, at minimum:

- `initialize`
- `session/new`
- `session/load`
- `session/prompt`
- `session/cancel`

Handshake-specific expectations:

- `session/new` creates or binds a governed role/WP session and yields stable session identity
- `session/load` resolves an existing governed role/WP session from repo projections
- `session/prompt` resumes the governed role session and streams updates
- `session/cancel` is a governed broker control verb; the canceled run's own settled result remains the durable repo audit row
- session mode stays fixed by repo law, and model selection remains governed through the wrapper commands rather than ad hoc ACP mutation verbs

The adapter may expose more ACP-compatible methods later, but these are the required baseline for Handshake.

## Deterministic Session Identity

Per governed session:

- one role
- one WP
- one assigned worktree
- one session key
- one stable ACP session identity
- one governed model choice

The registry remains the projection of that identity. ACP session ids do not replace the repo session key. They map to it.

## Repo Projection Rules

ACP must not become a second authority. Therefore:

- every governed start or prompt still writes a row to `SESSION_CONTROL_REQUESTS.jsonl`
- every governed cancel writes a row to `SESSION_CONTROL_REQUESTS.jsonl` and settles with its own result row
- every governed completion or failure still writes a row to `SESSION_CONTROL_RESULTS.jsonl`
- streamed updates still land in `SESSION_CONTROL_OUTPUTS/`
- `ROLE_SESSION_REGISTRY.json` remains the current-state projection
- broker liveness and active-run projection live in `.GOV/roles_shared/SESSION_CONTROL_BROKER_STATE.json`

ACP runtime state is valid only when mirrored into repo-governed artifacts.

Additional governance requirement:

- `gov-check` must enforce request/result parity, duplicate-command detection, missing output-log detection, and stale-running detection across the request ledger, result ledger, registry projection, and broker-state projection
- one governed role/WP session may have at most one active ACP broker-owned run at a time

Trust boundary note:

- this is a repo-governed local control plane, not a hostile-local-process security boundary
- authority comes from repo law plus the governed wrapper and broker handshake, not from OS-level isolation

## TUI Requirements

The operator monitor remains a viewport. It should not directly control ACP sessions.

The TUI should read:

- task board
- traceability registry
- active packet or stub
- packet-declared WP communications
- session registry
- session control requests/results
- merged governed control output logs for the selected WP
- git topology registry when needed to identify canonical `main` worktree and board path

Recommended viewport additions:

- `SESSIONS`: governed ACP sessions first, packet runtime sessions second
- `CONTROL`: recent governed ACP commands and outcomes for the selected WP
- `EVENTS`: merged tail of governed ACP output events for the selected WP across its governed role sessions
- `TIMELINE`: merged timestamp-ordered view of thread entries, receipts, governed control requests/results, and ACP events
- `ARTIFACT`: preview of the active packet or stub
- `BOARD_SOURCE`: current board source vs canonical `main` board path

Ordering and scope rules:

- `EVENTS` is WP-scoped, not single-session-scoped, unless a future per-session selector is added
- `TIMELINE` is sorted by the repo timestamp carried by each source artifact
- `CONTROL` remains the compact command/result ledger view; `TIMELINE` is the merged incident/debugging view

This gives the operator visibility into:

- stubs
- ready-for-dev packets
- superseded packets
- session steering history
- live role communication
- canonical vs mirrored board context

## Canonical Task Board Note

During a live WP, the most canonical portfolio board is still the task board on `main`.

The operator monitor may run from another worktree such as `wt-orchestrator`. The viewport must therefore:

- use the canonical `main` board for counts, filter buckets, and WP list selection whenever that canonical board is available
- use the current worktree board as the mirror comparison surface
- show drift between the selected canonical entry and the current worktree entry when they differ

The viewport must therefore surface:
- current board source worktree and branch
- canonical `main` worktree path
- whether the currently displayed board is canonical or mirrored

This is visibility, not authority transfer.

## Wake-Up Model

Wake semantics remain separate from steering:

- heartbeat = liveness only
- `validator_trigger` = validator wake hint only
- ACP prompt/resume = steering

No single field should carry all three meanings.

## Migration Intent

### Keep And Evolve

- `.GOV/scripts/session-policy.mjs`
- `.GOV/scripts/session-registry-lib.mjs`
- `.GOV/scripts/session-control-lib.mjs`
- `.GOV/scripts/session-control-command.mjs`
- `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- `.GOV/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/`
- `.GOV/scripts/operator-monitor-tui.mjs`

### Add

- `.GOV/tools/handshake-acp-bridge/`
- ACP helper docs and runtime checks
- TUI control/event visibility over repo projections

### Demote

- `.GOV/scripts/launch-cli-session.mjs`
- `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- terminal injection as the primary meaning of "session started"

The VS Code bridge still matters for local workflow convenience, but it is not the durable control plane.

## Operational Goal

The target end state is:

- Orchestrator-managed role sessions are steerable
- session control is deterministic and inspectable
- the operator TUI is a trustworthy viewport over repo-governed truth
- all work remains inside governance and does not touch product code

## Immediate Implementation Plan

1. Add a governance-owned ACP adapter package under `.GOV/tools/handshake-acp-bridge/`.
2. Route governed session start and steer commands through ACP, while preserving repo request/result ledgers.
3. Update the session registry projection to describe ACP-backed transport explicitly.
4. Extend `operator-monitor-tui.mjs` with control/event visibility and canonical board-source visibility.
5. Keep `just gov-check` as the minimum verification gate for all changes in this lane.
