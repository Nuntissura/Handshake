# Research: Swarm Harness Stalls, Handoffs, and Recovery

## METADATA
- RESEARCH_ID: RESEARCH-20260411-SWARM-HARNESS-STALLS-HANDOFFS-RECOVERY
- DATE_UTC: 2026-04-11
- AUTHOR: Codex acting as ORCHESTRATOR
- PURPOSE: Survey current open-source and official agentic harnesses to understand how they detect stalls, avoid unnecessary intervention, preserve workflow state, and recover from partial failure.
- PRIMARY_QUESTION: How should Handshake mechanically detect a genuinely stalled role without disturbing a still-working coder or validator lane?
- RELATED_GOVERNANCE:
  - RGF-162
  - RGF-163
  - RGF-166
  - RGF-167
  - workflow dossier / relay watchdog / ACP observability work

---

## 1. Executive Summary

The strongest current agentic harnesses do not treat "stall handling" as a single feature. They split it into five separate concerns:

1. Visibility: capture enough runtime activity to distinguish "quiet but healthy" from "actually stuck".
2. Workflow state: persist task ownership, pending work, and partial progress so recovery does not restart from zero.
3. Bounded autonomy: allow retries, wakes, and resumptions, but only within explicit iteration, timeout, or budget limits.
4. Human checkpoints: pause cleanly for approval or intervention when the system cannot continue safely.
5. Intervention hierarchy: prefer observe -> manager wake -> route retry -> bounded restart -> human escalation, in that order.

Across the surveyed systems, the common design move is not "poke every worker more aggressively." It is "improve manager- and runtime-level observability so intervention can stay conservative."

That supports the same conclusion already emerging inside Handshake: the default poke target should be the workflow authority or route manager, not the active coder/validator lane itself, unless the evidence shows the lane is both inactive and unrecoverable by route-level repair.

The clearest external patterns are:

- Claude Code Agent Teams: shared task list, direct mailbox, idle notifications, and hook-based quality gates.
- Overstory: SQLite mail, isolated worktrees, tiered watchdogs, explicit nudge surface, checkpoints, and handoff orchestration.
- Agent Swarm: lead/worker model over SQLite + API, real-time dashboard, pause/resume lifecycle, DAG workflow engine, per-step retry, and HITL pause nodes.
- LangGraph: durable execution, interrupts, replay/time travel, and explicit idempotency guidance for retries.
- AutoGen: explicit termination conditions, handoff termination, external termination, and group-chat manager routing.
- CrewAI: manager-led hierarchical execution, max-iteration controls, checkpointing, SQLite persistence, and resumable human-feedback pauses.
- OpenHands: event-stream architecture, isolated runtime, parallel delegation, and strong OTEL-style step/tool tracing.
- GitHub Agentic Workflows: read-only-by-default execution, safe outputs, sandboxing, explicit timeout and max-turn controls, plus status/health/audit commands.

The practical implication for Handshake is straightforward:

- Keep the current "do not disturb active worker by default" rule.
- Strengthen ACP/activity evidence until the Orchestrator can classify lanes into:
  - ACTIVE_HEALTHY
  - QUIET_BUT_PROGRESSING
  - ROUTE_STALE_NO_ACTIVE_RUN
  - ACTIVE_RUN_STALLED_RECOVERABLE
  - ACTIVE_RUN_STALLED_ESCALATE
- Only the last three classes should trigger intervention.

---

## 2. Selection Criteria

This survey prioritizes systems that expose at least one of:

- multi-agent or multi-session task decomposition
- explicit stall, termination, pause, or retry controls
- workflow state persistence or checkpointing
- observability beyond final output
- official or primary-source documentation on orchestration mechanics

The goal is not to rank general agent frameworks. The goal is to identify the specific mechanics that reduce downtime and false-positive "stall" judgments.

---

## 3. Project Findings

### 3.1 Claude Code Agent Teams

Primary sources:

- https://code.claude.com/docs/en/agent-teams
- https://code.claude.com/docs/en/sub-agents
- https://docs.anthropic.com/en/docs/claude-code/hooks

Key mechanics:

- Agent Teams use a team lead plus separate teammate sessions, each with its own context window.
- Teams coordinate through a shared task list plus mailbox messaging, not only through the lead.
- Teammates can message each other directly.
- The system emits idle notifications when a teammate finishes and stops.
- Team state is stored locally in generated team config and task-list files.
- Hooks provide deterministic lifecycle interception around tool use, stop/subagent-stop, session start, and session end.
- `Stop` and `SubagentStop` hooks can block stopping and force continuation.
- `Notification` hooks fire when Claude is waiting for input or permission.

What this means for stall handling:

- Claude splits "worker is done" from "worker is idle" via explicit notifications instead of forcing the lead to infer from silence.
- The hook model allows hard policy checks at the moment of tool use or stop, rather than after a lane has drifted.
- The task list externalizes work ownership, so the lead does not need to reconstruct who should act next from conversation history alone.

Relevant limitation:

- Agent Teams are still marked experimental and the docs explicitly note known limitations around session resumption, task coordination, and shutdown behavior.

Handshake takeaway:

- Shared task-list truth plus mailbox is stronger than inference from packet thread text alone.
- Idle/stop notification is a different signal from "no progress".
- Hook surfaces are a better place to emit liveness and state transitions than periodic polling alone.

### 3.2 Overstory

Primary source:

- https://github.com/ChatMason/overstory

Key mechanics:

- Single coordinator session spawns specialized workers in isolated git worktrees via tmux.
- Coordination runs through a custom SQLite mail system with typed message protocols.
- Merge flow is handled by a FIFO queue with tiered conflict resolution.
- Health monitoring is explicitly tiered:
  - Tier 0 mechanical daemon for tmux/PID liveness
  - Tier 1 AI-assisted failure triage
  - Tier 2 monitor agent for continuous patrol
- There is an explicit `nudge` command for stalled agents.
- Session checkpoint save/restore and handoff orchestration are first-class architecture elements.
- Tool enforcement hooks mechanically block unsafe writes or git operations.
- Transcript parsing and metrics extraction are built into the system.

What this means for stall handling:

- Overstory does not rely on a single signal.
- It separates pure runtime-liveness detection from semantic failure analysis.
- It gives the operator a bounded, explicit "nudge" surface instead of ad hoc manual prompting.
- It uses durable, queryable mail/storage instead of append-only text as the main coordination backbone.

Handshake takeaway:

- The strongest directly relevant pattern is tiered intervention.
- Handshake should distinguish:
  - process alive
  - workflow route healthy
  - semantic progress present
  - bounded wake needed
- SQLite as the control-plane spine is materially better than JSONL once query and suppression logic become non-trivial.

### 3.3 Agent Swarm

Primary source:

- https://github.com/desplega-ai/agent-swarm

Key mechanics:

- Lead agent plans and delegates to worker agents running in isolated Docker environments.
- The control plane is lead agent <-> MCP API server <-> SQLite DB.
- Task lifecycle includes priority queues, dependencies, and pause/resume across deployments.
- Real-time updates are exposed in the dashboard, chat threads, or API.
- Workflow engine is DAG-based and includes checkpoint durability, per-step retry, structured I/O, fan-out/convergence, configurable failure handling, and version history.
- Human-in-the-loop workflow nodes can pause for approval/input and are resumed through the dashboard.
- Agents accumulate compounding memory from session summaries, failed tasks, and file-based notes.
- Identity and profile changes are synced in real time via hooks.

What this means for stall handling:

- Agent Swarm treats stall recovery as part of a larger workflow engine, not as an afterthought.
- Pause/resume is explicit and durable.
- Dependencies are first-class, so "blocked waiting on upstream" is distinguishable from "stalled".
- Failures become future learning material, reducing repeated dead ends.

Handshake takeaway:

- Handshake should record "blocked by dependency" separately from "stalled".
- Route repair should be tied to a durable task/dependency model, not only open notifications and inferred next actor.
- A workflow engine style intervention ladder is more robust than one-off watchdog heuristics.

### 3.4 LangGraph

Primary sources:

- https://docs.langchain.com/oss/python/langgraph/interrupts
- https://docs.langchain.com/oss/javascript/langgraph/durable-execution
- https://docs.langchain.com/oss/javascript/langgraph/persistence
- https://docs.langchain.com/oss/python/langchain/multi-agent/handoffs
- https://docs.langchain.com/oss/python/langchain/supervisor

Key mechanics:

- Durable execution persists graph state so workflows can resume after failures.
- Interrupts pause execution and wait indefinitely until resumed with external input.
- Persistence supports replay, checkpoint history, and state forking/time travel.
- Completed work is preserved; on resume, successful prior steps do not need to rerun.
- Documentation explicitly requires idempotent side effects for retry safety.
- Handoff patterns are state-driven and require explicit context engineering so the receiving agent sees valid history.
- Supervisor pattern is centralized orchestration with specialized workers.

What this means for stall handling:

- LangGraph treats pause/resume as a first-class control flow, not a crash path.
- Recovery depends on deterministic state plus idempotent effects.
- Debugging uses persisted checkpoints, not human memory.

Handshake takeaway:

- The Workflow Dossier should be treated partly as a checkpoint-navigation surface, not only an audit artifact.
- Restart is only safe when side effects are idempotent or already recorded in a recoverable checkpoint.
- Handoff repair must validate context integrity, not only next-actor projection.

### 3.5 AutoGen

Primary sources:

- https://microsoft.github.io/autogen/dev/user-guide/agentchat-user-guide/tutorial/termination.html
- https://microsoft.github.io/autogen/dev/user-guide/agentchat-user-guide/selector-group-chat.html
- https://microsoft.github.io/autogen/stable/user-guide/core-user-guide/design-patterns/group-chat.html

Key mechanics:

- Group chat teams are driven by explicit termination conditions checked after each agent response.
- Built-in termination conditions include:
  - max message count
  - text mention
  - token usage
  - timeout
  - handoff termination
  - external termination
  - source match
- `SelectorGroupChat` chooses the next speaker dynamically based on context and agent descriptions.
- Group-chat manager emits explicit `RequestToSpeak` signals to the next participant.

What this means for stall handling:

- AutoGen refuses to leave termination implicit.
- Both budget and timeout become first-class stop conditions.
- Handoff to a specific target can itself be a termination/pause point.

Handshake takeaway:

- Handshake needs more explicit lane-level termination and pause classes.
- "Stop because waiting for human" and "stop because token/turn budget exhausted" should not be conflated.
- A `RequestToAct`-style explicit route signal may be easier to debug than pure next-actor inference from aggregate runtime state.

### 3.6 CrewAI

Primary sources:

- https://docs.crewai.com/en/learn/hierarchical-process
- https://docs.crewai.com/en/concepts/checkpointing
- https://docs.crewai.com/en/concepts/flows
- https://docs.crewai.com/en/learn/human-feedback-in-flows

Key mechanics:

- Hierarchical process gives a manager agent responsibility for delegation and result validation.
- Delegation is disabled by default for explicit control.
- Max iterations and max requests-per-minute are explicit manager-level controls.
- Checkpointing persists execution state for crews, flows, and agents.
- Checkpoints can be stored in JSON or SQLite; SQLite is recommended for higher-frequency checkpointing.
- Resuming from a checkpoint skips already-completed tasks.
- Human feedback pauses a flow through `HumanFeedbackPending`, automatically persists state, and resumes via `resume()` / `resume_async()`.
- Best-practice guidance explicitly calls for idempotent resume handlers.
- HITL learning can feed reviewer corrections back into memory.

What this means for stall handling:

- CrewAI turns manager validation, max-iteration caps, and HITL pause/resume into ordinary workflow features.
- Recovery is not ad hoc; it is part of the execution contract.
- Manager-led orchestration plus explicit iteration caps reduce runaway or silent loops.

Handshake takeaway:

- Add lane- or MT-level max revision / wake / restart ceilings that are mechanically surfaced.
- Prefer durable pause objects over free-form "waiting for operator" narration.
- Use SQLite when checkpoint volume or query complexity rises.

### 3.7 OpenHands

Primary sources:

- https://github.com/All-Hands-AI/OpenHands/
- https://docs.openhands.dev/sdk/guides/agent-delegation
- https://docs.openhands.dev/sdk/guides/observability
- ICLR 2025 OpenHands paper: https://proceedings.iclr.cc/paper_files/paper/2025/file/a4b6ad6b48850c0c331d1259fc66a69c-Paper-Conference.pdf

Key mechanics:

- Core architecture is agent abstraction + event stream + runtime.
- Every action execution yields an observation and both are tracked in the event stream.
- Per-task sessions run in isolated Docker sandboxes with mounted workspace.
- Sub-agent delegation supports parallel tasks using threads.
- Sub-agents inherit the same LLM configuration, work in the same workspace, keep independent conversation context, and the delegation call blocks until all sub-agents finish.
- Built-in observability traces:
  - conversation
  - conversation.run
  - agent.step
  - llm.completion
  - tool.execute
- Conversation lifecycle and tool execution are traceable with session IDs.

What this means for stall handling:

- OpenHands is strongest on event-level observability.
- Because sub-agent delegation blocks until all workers finish, it is better for parallel execution than for long-lived asynchronous role lanes.
- Event-stream architecture makes replay, debugging, and runtime instrumentation much clearer than chat-only history.

Handshake takeaway:

- ACP should keep moving toward explicit event-sequence semantics rather than only result summaries.
- Tool-call and lane-step events should be first-class inputs into the watchdog and dossier.
- The "same workspace, separate context" model is powerful but increases collision risk; Handshake's worktree isolation remains the safer choice for governed coding lanes.

### 3.8 OpenClaw + Lobster

Primary sources:

- https://github.com/openclaw/openclaw
- https://github.com/openclaw/lobster

Key mechanics:

- OpenClaw injects `AGENTS.md`, `SOUL.md`, and `TOOLS.md` into agent workspaces.
- Lobster is a local-first typed workflow shell.
- Lobster workflow files mix deterministic shell steps, native pipeline stages, and explicit approval gates.
- Approval is represented as a workflow primitive, not an informal convention.

What this means for stall handling:

- OpenClaw/Lobster is closer to a macro/workflow engine than a coding swarm manager.
- Its strongest contribution is making approval and deterministic sequencing declarative.

Handshake takeaway:

- Approval gates belong in the workflow contract, not only in role behavior text.
- Handshake's phase gates already move in this direction; the next step is making more pause/resume transitions machine-native rather than narration-native.

### 3.9 GitHub Agentic Workflows

Primary sources:

- https://github.github.com/gh-aw/
- https://github.github.com/gh-aw/setup/cli/
- https://github.github.com/gh-aw/troubleshooting/common-issues/
- https://github.github.com/gh-aw/reference/sandbox/

Key mechanics:

- Workflows run in GitHub Actions with sandboxed execution and safe-output processing.
- Read-only tokens are the default; writes require explicit safe-output pathways.
- Status, health, logs, and audit are first-class CLI surfaces (`gh aw status`, `gh aw health`, `gh aw logs`, `gh aw audit`).
- Failure guidance is explicit:
  - default job timeout of 20 minutes unless configured otherwise
  - if max turns are reached, increase `max-turns` or decompose into smaller workflows
  - if workflows time out repeatedly, narrow scope rather than endlessly retry
- Workflow health metrics are surfaced as an ordinary operator-facing command.

What this means for stall handling:

- GitHub Agentic Workflows treats runaway execution as a budgeting and decomposition problem, not just a runtime problem.
- It gives the operator explicit health and audit commands before manual repair.

Handshake takeaway:

- Add a true operator-facing "lane health" summary that includes:
  - timeout class
  - turn/budget class
  - route health
  - intervention recommendation
- If a lane repeatedly exceeds a bounded cycle, the next action should be decomposition or scoping repair, not just another wake.

---

## 4. Cross-System Pattern Map

| Concern | Claude Teams | Overstory | Agent Swarm | LangGraph | AutoGen | CrewAI | OpenHands | GitHub AW | Handshake Today |
|---|---|---|---|---|---|---|---|---|---|
| Shared work ownership | Shared task list | Beads + mail | Task lifecycle + DAG | Graph state | Team manager + messages | Crew/tasks/flows | Conversation + delegation | Workflow files | Packet + runtime + notifications |
| Direct peer communication | Yes | Yes | Yes | Possible via graph routing | Yes | Manager-centric | Limited by delegation model | No, mostly workflow/job oriented | Coder/validator receipts |
| Durable resume | Limited / experimental | Checkpoints | Checkpoint durability | Core feature | Resume patterns | Checkpoints + pending flow state | Event stream + runtime | Workflow reruns / audits | Partial via ledgers and runtime projections |
| Explicit HITL pause | Hooks / plan approval | Nudge + operator | HITL nodes | Interrupts | External termination / handoff | HumanFeedbackPending | UI driven, not core lane model | Safe outputs / review | Packet/thread/operator checkpoints |
| Stall/health observability | Notifications + hooks | Tiered watchdog + metrics | Dashboard + API | Checkpoints + state | Termination conditions | Event bus + tracing | OTEL traces + event stream | status/health/audit | Watchdog + dossier + ACP JSONL |
| Bounded retries / stop | Hooks, team limits | Nudge + patrol tiers | Per-step retry | Durable replay + explicit tasks | Timeout/token/max message | Max iterations / retry / resume | Delegation threads + traces | timeout-minutes / max-turns | Relay cycles + restart guards |

Common outcome:

- The best systems combine explicit work state, explicit pause/stop semantics, and strong observability.
- No strong system relies on "human remembers what was happening" as the primary recovery path.

---

## 5. What Handshake Should Adopt

### 5.1 Immediate Direction

The safest near-term path is:

1. Keep intervention orchestrator-centric.
2. Improve evidence collection until "observe-only" is good enough to trust.
3. Only restart a worker lane when the route manager cannot repair it and the activity evidence is stale.

This aligns with both the external systems and Handshake's governed role boundaries.

### 5.2 Recommended Intervention Ladder

Handshake should formalize this ladder:

1. Observe only
   - Collect ACP activity, runtime projection, control rows, receipts, and dossier idle metrics.
   - Output a single recommendation class, but do not mutate anything.

2. Route-level wake
   - If no active run exists and next-actor projection is stale, re-wake the route through the Orchestrator.
   - Do not message coder or validator directly first.

3. Active-run verification
   - If an active run exists, inspect ACP output activity by type:
     - command execution
     - file change
     - web search
     - todo/task planning
     - agent message
   - If recent progress exists, suppress intervention.

4. Bounded restart
   - Only if:
     - route is stale
     - active run is present
     - ACP output is stale
     - timeout window is exceeded
     - side effects are recoverable or idempotent enough
   - Restart count must be bounded and surfaced.

5. Human escalation
   - If repair budget is exhausted or evidence is contradictory, mark attention-required and stop automatic wakeups.

### 5.3 Specific Design Changes

#### A. Add a first-class "observe-only stall verdict" artifact

The current watchdog now has an observe-only mode, which is the correct start. Extend this into a stable verdict object:

- `ACTIVE_HEALTHY`
- `QUIET_BUT_PROGRESSING`
- `ROUTE_STALE_NO_ACTIVE_RUN`
- `ACTIVE_RUN_STALLED_RECOVERABLE`
- `ACTIVE_RUN_STALLED_BUDGET_EXHAUSTED`
- `WAITING_ON_HUMAN`
- `BLOCKED_ON_DEPENDENCY`

Why:

- Most systems separate healthy waiting from unhealthy waiting.
- The current Handshake surfaces still make the operator infer too much.

#### B. Promote ACP activity classes into the control plane

The watchdog and dossier should not only know "output file changed." They should know what changed:

- file edits
- searches
- tool loops
- repeated errors
- repeated retries
- explicit task-plan changes

Why:

- OpenHands and OTEL-style systems show that step- and tool-level traces are what make debugging tractable.
- Overstory's transcript-aware watchdog tiers also point here.

#### C. Introduce route-state reasons beyond "attention_required"

Current recovery can still collapse multiple meanings into one attention bit. Split it into typed causes:

- `WAITING_ON_VALIDATOR`
- `WAITING_ON_CODER`
- `WAITING_ON_ORCHESTRATOR_CHECKPOINT`
- `WAITING_ON_HUMAN_APPROVAL`
- `WAITING_ON_DEPENDENCY`
- `STALL_NO_PROGRESS`
- `STALL_RETRY_LOOP`
- `STALL_REPEATED_ERROR`
- `STALL_COMMAND_LOOP`
- `RELAY_BUDGET_EXHAUSTED`

Why:

- AutoGen, CrewAI, LangGraph, and GitHub AW all externalize stop/termination/pause reasons.

#### D. Add MT- and lane-scoped retry budgets

Track separately:

- validator-to-coder remediation cycles per MT
- orchestrator route wakes per lane
- cancel-and-resteer count per lane
- same-error recurrence count

Why:

- CrewAI max iterations and GitHub AW max-turns/timeout guidance both show that bounded retries are required to keep repair honest.

#### E. Make blocked-vs-stalled a first-class distinction

If a lane is waiting on:

- another MT
- another role
- approval
- a missing dependency

that is not a stall.

Why:

- Agent Swarm's DAG/dependency model and Claude Teams' shared task-list unblocking both demonstrate this clearly.

#### F. Move toward a SQLite control-plane spine

Strong candidate future migration:

- session-control requests/results
- notification queue
- receipt ledger
- route/watchdog counters
- lane activity summaries

in one typed SQLite database, with append-only semantics where needed.

Why:

- Overstory, Agent Swarm, and CrewAI all rely on SQLite where local concurrency and querying matter.
- Handshake has reached the point where suppression, joining, and drift diagnosis are more complex than append-only files handle comfortably.

### 5.4 What Handshake Should Not Copy

- Do not copy OpenHands' same-workspace sub-agent concurrency for governed coder/validator lanes. Worktree isolation remains safer for Handshake's audit model.
- Do not let the route manager disappear into fully self-claiming autonomy yet. Claude Teams can rely on shared task list plus unified runtime assumptions; Handshake still depends on governed packet truth and role boundaries.
- Do not add restarts before stronger checkpoint semantics exist. LangGraph and CrewAI make restart safe because recovery state is explicit and resume-aware.

---

## 6. Recommended Handshake Roadmap

### Phase A: Better evidence, same authority model

- keep Orchestrator as the only automatic wake authority
- keep worker lanes non-self-starting
- improve observe-only lane verdicts
- enrich dossier sync with ACP activity classes
- distinguish blocked vs stalled vs waiting-on-human

### Phase B: Typed recovery and bounded retries

- add typed route-state reasons
- add MT and lane retry budgets
- add explicit pause states for:
  - human approval
  - route stale
  - dependency blocked
  - budget exhausted

### Phase C: Storage and replay hardening

- move WP communications + control-plane summaries to SQLite
- preserve append-only audit exports
- add queryable checkpoint/replay support for governed session recovery

### Phase D: Optional shared-task projection

- add a generated MT task board with:
  - declared
  - claimed
  - completed
  - failed
  - blocked
- keep packet authority primary, but project task ownership more explicitly

---

## 7. Handshake Repo-Governance Operating Model

This repo-governance layer is the current testbed for the product-governance operating model. It is intentionally more compartmentalized, more artifact-heavy, and more audited than the likely end-state product UX because the immediate goal is to make autonomous agentic work safe enough to inspect, replay, and repair.

### 7.1 Repo Workflow

The governed repo workflow is packet-centric and file-first:

1. Stub or follow-on need is identified in the backlog / traceability surfaces.
2. Refinement is authored or repaired, with approved spec-enrichment only when the refinement proves it is required.
3. The Orchestrator records signature, workflow lane, execution owner, and role-model profiles.
4. The packet is hydrated, microtasks are populated when declared, worktrees are prepared, and the live Workflow Dossier is seeded.
5. For `ORCHESTRATOR_MANAGED` lanes, Activation Manager completes readiness before governed coder / validator launch.
6. Coder and `WP_VALIDATOR` run the normal implementation + per-MT review loop through governed receipts, notifications, and steering.
7. `INTEGRATION_VALIDATOR` owns final verdict and closeout sync, including packet / runtime / task-board truth promotion.
8. Audits and post-mortem artifacts remain attached to the run instead of being reconstructed from chat alone.

This means the true swarm control problem in Handshake is not "how do multiple models reason in parallel?" It is "how do we keep packet truth, route truth, runtime truth, and audit truth aligned while multiple governed sessions are active?"

### 7.2 Governance Artifacts

The main repo-governance artifacts already form a partial control plane:

- `.GOV/roles_shared/records/TASK_BOARD.md`
  - human-readable project execution board
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - governance-only refactor sequencing board
- `.GOV/roles_shared/records/BUILD_ORDER.md`
  - dependency and sequencing projection
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - base WP to active packet mapping
- `.GOV/task_packets/**`
  - executable WP contracts and active packet family truth
- `.GOV/refinements/**`
  - signed technical refinements that justify packet shape and scope
- packet-declared `WP_COMMUNICATION_DIR`
  - thread, receipt, and notification truth for governed role-to-role flow
- `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - external governed runtime ledgers for ACP launch / steer / cancel truth
- `.GOV/Audits/smoketest/**`
  - live Workflow Dossiers, smoketest reviews, and post-run audit artifacts

Together these surfaces already record much more than a typical swarm harness, but the recording still leans too heavily on inferred silence, broad status bits, and retrospective audit writing.

### 7.3 Roles

The governed role set is deliberately split:

- `ORCHESTRATOR`
  - workflow authority; owns launch, steering, packet/routing truth, and status progression
- `ACTIVATION_MANAGER`
  - bounded pre-launch worker for refinement, packet hydration, microtask setup, and readiness
- `CODER`
  - scoped implementation worker bound by packet scope and phase gates
- `WP_VALIDATOR`
  - WP-scoped technical reviewer and steering validator during execution
- `INTEGRATION_VALIDATOR`
  - final technical verdict and closeout authority
- `MEMORY_MANAGER`
  - bounded governance-memory hygiene and pattern capture

This is already a manager-worker swarm, but with stronger role boundaries and more externalized audit surfaces than typical open-source agent swarms.

### 7.4 Work Packet Lifecycle

The WP lifecycle is currently:

1. Stub backlog or follow-on discovery
2. Refinement / enrichment review
3. Signature + workflow-lane selection
4. Packet creation / update
5. Worktree and runtime preparation
6. Activation readiness
7. `STARTUP` phase gate
8. Implementation and microtask execution
9. Direct `CODER` <-> `WP_VALIDATOR` review exchanges during the run
10. `HANDOFF` phase gate
11. `VERDICT` / `CLOSEOUT` final-lane checks and truth sync
12. Task-board, runtime, dossier, and audit closure

For autonomous swarm work, the expensive part is not the individual steps. The expensive part is the waiting between them when the system cannot tell whether a lane is:

- actively working
- blocked on another role
- blocked on a dependency
- waiting on human input
- stale but recoverable
- trapped in a retry or command loop

### 7.5 What Must Be Added So The Workflow Is Recorded Reliably

Some groundwork is already in place: the watchdog now has an observe-only mode, and Workflow Dossier sync can already summarize ACP activity classes. The next additions should make the control loop legible enough to manage real autonomous swarm work without constant operator babysitting.

Add these recording layers:

- a first-class per-lane verdict artifact
  - machine-readable lane status rather than only log text
- typed activity-class capture from ACP/session output
  - not just "file changed", but what kind of work occurred
- typed wait and stall reasons
  - `WAITING_ON_REVIEW`, `WAITING_ON_DEPENDENCY`, `WAITING_ON_HUMAN_APPROVAL`, `STALL_RETRY_LOOP`, and similar states
- MT- and lane-scoped retry / wake budgets
  - how many wakes, reteers, cancel-and-restart attempts, and repeated-error cycles occurred
- wall-clock attribution
  - active build time vs validator wait vs route wait vs human wait vs repair overhead
- manager-first wake policy evidence
  - whether the system woke the route authority only, or actually interrupted a worker lane

These additions are what allow the repo-governance testbed to become a usable autonomous swarm harness rather than a high-audit but slow multi-role ritual.

### 7.6 Governance Work Items To Materialize This

The immediate governance follow-ons for this research are:

- `RGF-177`
  - typed lane verdict artifact and stall taxonomy
- `RGF-178`
  - ACP activity-class projection into runtime and Workflow Dossier truth
- `RGF-179`
  - blocked-vs-stalled route reason codes
- `RGF-180`
  - wall-clock downtime attribution and queue-pressure ledger
- `RGF-181`
  - manager-first wake policy and explicit worker-interrupt budget

These are not product-code WPs. They are governance control-plane improvements that should make the repo testbed a better predictor for the eventual product-governance swarm runtime.

---

## 8. Final Assessment

If the question is "should Handshake mechanically poke roles when they appear stalled?", the research answer is:

- yes, but only after better classification
- yes, but primarily at the route-manager/orchestrator layer
- yes, but with explicit budgets and typed reasons
- no, not by indiscriminately interrupting active coder or validator runs

The most mature external systems all converge on the same discipline:

- observe first
- retain state
- route explicitly
- retry conservatively
- escalate cleanly

Handshake is already closer to that model than to naive swarm prompting. The main gap is not conceptual. The gap is that too much of the current intervention logic still depends on inferred silence rather than typed workflow state plus richer ACP activity evidence.

---

## 9. Source Index

### Official docs and repos

- Claude Code Agent Teams: https://code.claude.com/docs/en/agent-teams
- Claude Code Subagents: https://code.claude.com/docs/en/sub-agents
- Anthropic Claude Code Hooks: https://docs.anthropic.com/en/docs/claude-code/hooks
- Overstory: https://github.com/ChatMason/overstory
- Agent Swarm: https://github.com/desplega-ai/agent-swarm
- LangGraph Interrupts: https://docs.langchain.com/oss/python/langgraph/interrupts
- LangGraph Durable Execution: https://docs.langchain.com/oss/javascript/langgraph/durable-execution
- LangGraph Persistence: https://docs.langchain.com/oss/javascript/langgraph/persistence
- LangChain Multi-Agent Handoffs: https://docs.langchain.com/oss/python/langchain/multi-agent/handoffs
- LangChain Supervisor Pattern: https://docs.langchain.com/oss/python/langchain/supervisor
- AutoGen Termination: https://microsoft.github.io/autogen/dev/user-guide/agentchat-user-guide/tutorial/termination.html
- AutoGen Selector Group Chat: https://microsoft.github.io/autogen/dev/user-guide/agentchat-user-guide/selector-group-chat.html
- AutoGen Group Chat Pattern: https://microsoft.github.io/autogen/stable/user-guide/core-user-guide/design-patterns/group-chat.html
- CrewAI Hierarchical Process: https://docs.crewai.com/en/learn/hierarchical-process
- CrewAI Checkpointing: https://docs.crewai.com/en/concepts/checkpointing
- CrewAI Flows: https://docs.crewai.com/en/concepts/flows
- CrewAI Human Feedback in Flows: https://docs.crewai.com/en/learn/human-feedback-in-flows
- OpenHands SDK Agent Delegation: https://docs.openhands.dev/sdk/guides/agent-delegation
- OpenHands SDK Observability: https://docs.openhands.dev/sdk/guides/observability
- OpenHands paper (ICLR 2025): https://proceedings.iclr.cc/paper_files/paper/2025/file/a4b6ad6b48850c0c331d1259fc66a69c-Paper-Conference.pdf
- OpenClaw: https://github.com/openclaw/openclaw
- Lobster: https://github.com/openclaw/lobster
- GitHub Agentic Workflows: https://github.github.com/gh-aw/
- GitHub Agentic Workflows CLI: https://github.github.com/gh-aw/setup/cli/
- GitHub Agentic Workflows Common Issues: https://github.github.com/gh-aw/troubleshooting/common-issues/
- GitHub Agentic Workflows Sandbox: https://github.github.com/gh-aw/reference/sandbox/
