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
   - `just operator-viewport`
3. Steer work through governed prompts:
   - `just steer-coder-session WP-{ID} "<prompt>"`
   - `just steer-wp-validator-session WP-{ID} "<prompt>"`
4. Cancel a governed run when needed:
   - `just cancel-coder-session WP-{ID}`
   - `just cancel-wp-validator-session WP-{ID}`
5. Close a governed session when you want the thread registration cleared:
   - `just close-coder-session WP-{ID}`
   - `just close-wp-validator-session WP-{ID}`
6. Stop the ACP broker when no governed runs remain:
   - `just handshake-acp-broker-status`
   - `just handshake-acp-broker-stop`
7. Use packet communications for human/role collaboration:
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
- close is first-class and auditable
- operator monitoring sees canonical board state plus governed control activity and broker/session lifecycle state

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

## Live Smoke Test Findings (2026-03-13)

This architecture was proven in a live end-to-end run of `WP-1-Structured-Collaboration-Artifact-Family-v1`.

Validated workflow outcome:

- implementation source head validated by WP Validator: `84d37247e9e8e6ff6350fb109e5faaea821af9b9`
- feature branch closeout head after integration-validator governance records: `f9e89285a17dc2f7a2a19f08c09fd57c89e89d2b`
- merged to canonical `main` at: `5367d86e960888ff1ccd04308bbc847e87442d7a`

Governance and ACP defects found and fixed during the live run:

- broker run timeout was too short for real WP implementation work
  - original governed run ceiling: `900s`
  - patched governed run ceiling: `5400s`
- broker build identity had to be bumped when timeout semantics changed so stale in-memory brokers were rejected and restarted
- historical settled result rows were being invalidated by broker build changes
  - fixed by treating `broker_build_id` on settled rows as historical audit metadata, while live broker identity remains exact-match enforced by the client/broker handshake
- topology registry validation produced a false positive in clean Windows checkouts because it compared raw file bytes instead of normalized line endings
  - fixed by normalizing `CRLF/LF` in `topology-registry-check`
- advisory/final validator worktrees created from `main` are expected to start with stale packet/runtime mirrors for an active WP
  - proven operating rule: validate the explicit feat-branch/WP-worktree handoff state, not the initial main-based local mirror

Operational conclusions proven by the smoke test:

- the governed ACP lane is sufficient to run `Coder -> WP Validator -> Integration Validator -> merge to main`
- the VS Code bridge is not needed for ongoing steering once the governed ACP thread exists
- repo projections remain authoritative enough for audit, but the Orchestrator must still steer validators toward the correct feat-branch source of truth when their bootstrap worktrees begin from `main`

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
- the main TUI remains governance-backed and non-authoritative as a rich viewport only
- any lifecycle actions such as `close session` or `broker stop` belong to a separate explicit admin mode, not the default operator viewport

This is not a `.bat` file design. The deliverable is a governance-owned tool package plus repo scripts, ledgers, and viewport updates.

## Authority Boundaries

The following artifacts remain authoritative:

- active packet or stub: scope, lifecycle, acceptance, and final packet truth
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`: active artifact mapping per base WP
- `.GOV/roles_shared/records/TASK_BOARD.md` on canonical branch `main`: canonical portfolio board
- packet-declared external repo-governance `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/`
  - `THREAD.md`
  - `RUNTIME_STATUS.json`
  - `RECEIPTS.jsonl`

The following artifacts are projections, not work-scope authority:

- `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
- `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/`
- ACP adapter runtime metadata under `.GOV/tools/handshake-acp-bridge/`

If packet truth and session-control artifacts disagree, the packet wins.

Operational note:

- session runtime ledgers may contain operator-facing file links, output-log paths, and launch/bootstrap command text
- those runtime artifacts are audited by their dedicated runtime/schema checks
- they should not be treated as the canonical drive-agnostic governance text surface

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

- `.GOV/roles_shared/scripts/session/session-policy.mjs`
- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-command.mjs`

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
- require a repo-scoped auth token plus `ORCHESTRATOR` / `gov_kernel` initialization claims before any non-bootstrap ACP method is accepted
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

- `.GOV/operator/scripts/operator-viewport-tui.mjs`

Responsibilities:

- render the board, packets, session projections, receipts, thread entries, and ACP-derived projections
- stay read-only except for existing governed thread append behavior
- never talk directly to ACP as an authoritative channel

The TUI reads repo projections. It does not own orchestration.

## Primary Session Surfaces

### Canonical Governance Surfaces

- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/task_packets/`
- `.GOV/task_packets/stubs/`
- packet-declared `WP_COMMUNICATIONS`

### Governed Session-Control Surfaces

- `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
- `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/`

These remain the repo audit trail for ACP-backed control.

### Legacy Launch Surface

- `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`

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

- every governed start or prompt still writes a row to the external repo-governance `SESSION_CONTROL_REQUESTS.jsonl`
- every governed cancel writes a row to the external repo-governance `SESSION_CONTROL_REQUESTS.jsonl` and settles with its own result row
- every governed completion or failure still writes a row to the external repo-governance `SESSION_CONTROL_RESULTS.jsonl`
- streamed updates still land in the external repo-governance `SESSION_CONTROL_OUTPUTS/`
- the external repo-governance `ROLE_SESSION_REGISTRY.json` remains the current-state projection
- broker liveness and active-run projection live in the external repo-governance `SESSION_CONTROL_BROKER_STATE.json`

ACP runtime state is valid only when mirrored into repo-governed artifacts.

Additional governance requirement:

- `gov-check` must enforce request/result parity, duplicate-command detection, missing output-log detection, and stale-running detection across the request ledger, result ledger, registry projection, and broker-state projection
- one governed role/WP session may have at most one active ACP broker-owned run at a time
- final PASS commit clearance for orchestrator-managed validation must use committed handoff evidence from `just validator-handoff-check WP-{ID}`, which runs against the PREPARE worktree source of truth rather than a possibly dirty validator mirror
- when external/classical validation is used, the Orchestrator should require `just validator-startup` followed immediately by `just external-validator-brief WP-{ID}` so the validator gets one canonical code target, one governance target, the committed handoff command, and the legal split-verdict contract (`VALIDATION_CONTEXT`, `CODE_VERDICT`, `GOVERNANCE_VERDICT`, `ENVIRONMENT_VERDICT`, `DISPOSITION`, `LEGAL_VERDICT`)

Trust boundary note:

- this is a repo-governed local control plane, not a hostile-local-process security boundary
- authority comes from repo law plus the governed wrapper and broker handshake, not from OS-level isolation

## TUI Requirements

The default operator viewport remains a viewport. It must stay read-only and must not directly control ACP sessions.

The read model should combine:

- canonical task board from `main`
- current worktree board as the mirror comparison surface
- traceability registry
- active packet
- related refinement
- active stub when one exists
- packet-declared WP communications
- session registry
- session control requests/results
- merged governed control output logs for the selected WP
- git topology registry when needed to identify canonical `main` worktree and board path
- validator gate and audit artifacts when present

### Design Direction

The target interaction model is closer to `k9s` and `lazygit` than to a board editor:

- left rail: WP list grouped by canonical workflow state
- right header: selected WP summary, authority badges, drift, owner, next actor, validator posture, session health
- right body: tabbed inspection area
- bottom status strip: filter, focus, follow state, refresh, broker state, key hints

The primary unit is the `WP`, not the governed session. Sessions are a child surface of a WP.

### Read-Only Operator Views

Recommended Phase 1 views:

- `OVERVIEW`: canonical board state, current mirror state, drift, owner, lane, blocker, next actor, validator and ACP posture
- `DOCS`: full read-only viewer for packet, refinement, stub, runtime status, thread, receipts, validator gate, and relevant audit files
- `COMMS`: packet-scoped communication surfaces with drill-down for `THREAD.md`, `RECEIPTS.jsonl`, and `RUNTIME_STATUS.json`
- `SESSIONS`: governed ACP session registry state, broker state, last command/result, thread registration, and active-run posture
- `TIMELINE`: merged timestamp-ordered view of thread entries, receipts, governed control requests/results, and ACP events

Ordering and scope rules:

- the WP list is sourced from canonical `main` whenever available
- `DOCS` is the authoritative file-inspection surface; it should show actual file content, not only summaries
- `TIMELINE` is WP-scoped by default and sorted by repo timestamps carried by each source artifact
- `SESSIONS` is diagnostic context for the selected WP, not the primary browsing surface
- status summaries should stay compact; full text belongs in the file/timeline viewers

### Navigation Model

Recommended interaction model:

- `tab` / `shift-tab`: switch pane focus
- `j/k` or arrows: move within the focused pane
- numeric tabs for top-level views
- `/`: search/filter WPs
- `enter`: expand or switch the selected artifact/viewer
- follow/tail mode only on timeline-style views
- no mutating action keys in the default monitor

The operator should always be able to answer three questions quickly:

- what needs attention now
- what is the authoritative state of this WP
- what actually happened and what is actually written

### Admin Mode Separation

Admin functionality must be separate from the default viewport.

- default `operator-viewport` = read-only viewport (`operator-monitor` remains a compatibility alias)
- explicit admin mode or separate admin entrypoint = lifecycle actions such as `close session`, `broker stop`, or other governed controls
- admin mode must call the same governed wrappers as the normal Orchestrator workflow
- admin mode must remain auditable through the same request/result/output ledgers

This separation keeps the monitor from becoming a worksurface while still allowing a future governed admin console.

### What The Operator Must Be Able To Inspect

The TUI should give direct read-only visibility into:

- stubs
- ready-for-dev packets
- superseded packets
- refinements
- live role communication
- governed session steering history
- canonical vs mirrored board context
- the actual contents of the selected authoritative files, not only status summaries

## Canonical Task Board Note

During a live WP, the most canonical portfolio board is still the task board on `main`.

The operator viewport may run from another worktree such as `wt-gov-kernel`. The viewport must therefore:

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

- `.GOV/roles_shared/scripts/session/session-policy.mjs`
- `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
- external repo-governance `roles_shared/ROLE_SESSION_REGISTRY.json`
- external repo-governance `roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- external repo-governance `roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- external repo-governance `roles_shared/SESSION_CONTROL_OUTPUTS/`
- `.GOV/operator/scripts/operator-viewport-tui.mjs`

### Add

- `.GOV/tools/handshake-acp-bridge/`
- ACP helper docs and runtime checks
- TUI control/event visibility over repo projections

### Demote

- `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
- external repo-governance `roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
- terminal injection as the primary meaning of "session started"

The VS Code bridge still matters for local workflow convenience, but it is not the durable control plane.

## Operational Goal

The target end state is:

- Orchestrator-managed role sessions are steerable
- session control is deterministic and inspectable
- the operator viewport TUI is a trustworthy viewport over repo-governed truth
- all work remains inside governance and does not touch product code

## Immediate Implementation Plan

1. Add a governance-owned ACP adapter package under `.GOV/tools/handshake-acp-bridge/`.
2. Route governed session start and steer commands through ACP, while preserving repo request/result ledgers.
3. Update the session registry projection to describe ACP-backed transport explicitly.
4. Extend `operator-viewport-tui.mjs` with control/event visibility and canonical board-source visibility.
5. Keep `just gov-check` as the minimum verification gate for all changes in this lane.
