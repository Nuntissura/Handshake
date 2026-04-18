# Harness Adoption Extraction Working Notes

Temporary working file for mechanism-level extraction from external harness codebases. This is not a stable research artifact yet. The goal is to capture exact reusable runtime, coordination, and governance mechanisms before they are normalized into the main repo-governance research docs.

## Purpose

- move from broad harness awareness to code-backed mechanism extraction
- focus on external harnesses that look most relevant to Handshake repo-governance failure modes
- record exact code paths, state carriers, control surfaces, and adoption hypotheses

## Working Rules

- prefer exact mechanisms over impressions
- record file paths for every concrete claim
- distinguish `copy`, `adapt`, `observe`, and `reject`
- keep unresolved questions visible instead of smoothing them over

## Relevance Lens

The mechanisms below are being evaluated against current repo-governance pain points:

- workflow truth drift
- session lifecycle and recovery
- validator and closeout convergence
- parallel coordination and work transfer
- artifact hygiene and durable auditability
- token/time overhead from control-plane churn

## Extraction Matrix

| Harness | Capability / Problem | Exact Mechanism | Code Path | Why It Matters For Handshake Repo Governance | Adoption Read |
| --- | --- | --- | --- | --- | --- |
| LangGraph | durable execution state | checkpoint object stores channel values, versions, versions seen, updated channels, timestamp, id | `langgraph/libs/checkpoint/langgraph/checkpoint/base/__init__.py` | strong reference for durable workflow truth and replayable execution state | adapt |
| LangGraph | interrupt and resume control | explicit `Interrupt`, `StateSnapshot`, `Command`, and `Send` types expose resume, goto, update, and fan-out semantics | `langgraph/libs/langgraph/langgraph/types.py` | useful for swarm-safe pause/resume and controlled parallel fan-out | adapt |
| Open SWE | thread-scoped repo execution | thread metadata carries sandbox and repo execution identity; runtime reconnects by thread | `open-swe/agent/server.py`, `open-swe/agent/utils/sandbox_state.py` | useful model for stable execution identity without spreading truth across many ledgers | adapt |
| Open SWE | asynchronous follow-up while run is busy | queued messages stored under thread namespace and injected by middleware before model step | `open-swe/agent/middleware/check_message_queue.py` | useful for steering, announce-back, and operator relay without corrupting active run context | adapt |
| Open SWE | deterministic workflow keying | issue, PR, Slack, and Linear ingress paths derive stable `thread_id` values from source identifiers | `open-swe/agent/webapp.py`, `open-swe/agent/utils/github_comments.py` | useful for replacing ad hoc lane/session identity rules with deterministic workflow keys | adapt |
| Open SWE | terminal safety net | explicit commit/PR tool is backed by fallback PR middleware and anti-silent-finish middleware | `open-swe/agent/tools/commit_and_open_pr.py`, `open-swe/agent/middleware/open_pr.py`, `open-swe/agent/middleware/ensure_no_empty_msg.py` | useful for making completion robust even when the model misses the preferred terminal action | adapt |
| Letta | governance policy as data | typed tool-rule objects encode child/parent/conditional/approval semantics | `letta/letta/schemas/tool_rule.py` | useful for making governance restrictions explicit and machine-readable | adapt |
| Letta | capability-attached approval policy | attaching a tool can auto-attach a `RequiresApprovalToolRule` | `letta/letta/services/agent_manager.py` | useful for binding authority policy automatically when a role or tool is granted | adapt |
| Letta | approval as durable control-plane stop | approval-required tool calls emit durable approval requests and stop reasons, then require matching approval/denial to resume | `letta/letta/agents/helpers.py`, `letta/letta/interfaces/*_streaming_interface.py` | useful reference for explicit approval boundaries in governed workflows | adapt |
| Letta | copy-on-execute with selective commit-back | side-effecting tool execution deep-copies agent state, strips capabilities, runs in sandbox, and only commits validated memory deltas | `letta/letta/services/tool_executor/sandbox_tool_executor.py`, `letta/letta/agent.py` | useful pattern for safe tool execution against durable governance state | adapt |
| Letta | split run and step telemetry | run manager persists runs and metrics separately from step-level timing and tool-use records | `letta/letta/services/run_manager.py`, `letta/letta/services/step_manager.py` | useful for cleaner audit and cost visibility than deriving everything from logs | adapt |
| LangGraph | thread lifecycle and conflict handling | thread API supports explicit ids, duplicate handling, metadata merge, TTL, search, and superstep-based copy-on-create | `langgraph/libs/sdk-py/langgraph_sdk/_sync/threads.py` | useful for replacing ad hoc runtime ledgers with one durable thread object and explicit lifecycle controls | adapt |
| LangGraph | resumable runs and multitask control | run API exposes resumable streams, interrupt before/after, durability mode, disconnect behavior, future scheduling, and multitask strategies | `langgraph/libs/sdk-py/langgraph_sdk/_sync/runs.py` | useful for governed pause, resume, enqueue, interrupt, and async continuation semantics | adapt |
| OpenHands | PR-only artifact hygiene | repo instructions formalize `.pr/` for temporary design, analysis, and log artifacts with notification and cleanup flow | `openhands/AGENTS.md` | useful as a lightweight hygiene pattern for non-canonical review artifacts | copy |
| OpenHands | resolver output as structured artifact | resolver persists issue, base commit, patch, history, metrics, success, and explanations in a structured output record | `openhands/openhands/resolver/resolver_output.py` | useful reference for keeping a single durable run artifact per issue resolution attempt | observe |
| smolagents | tool admission and reproducibility | tool validator rejects non-self-contained tools and non-default init arguments to preserve rebuildability | `smolagents/src/smolagents/tool_validation.py` | useful for preventing hidden state from sneaking into reusable governance tools or role packs | adapt |
| smolagents | executor boundary and safe transport | local vs remote executor split with safe serializer by default, pickle opt-in, and explicit remote executor families | `smolagents/src/smolagents/agents.py`, `smolagents/src/smolagents/remote_executors.py`, `smolagents/src/smolagents/serialization.py` | useful for code-execution isolation and explicit trust boundaries | adapt |
| smolagents | exportable replay surface | agent can save tools, managed agents, prompts, and config to a portable folder layout | `smolagents/src/smolagents/agents.py` | useful as a lower-overhead model for exportable, inspectable runtime packaging | observe |
| LangGraph | barriered step progression | runtime loop advances through tick/runner/after-tick stages and persists checkpoints according to explicit durability mode | `langgraph/libs/langgraph/langgraph/pregel/main.py`, `langgraph/libs/langgraph/langgraph/pregel/_loop.py` | useful reference for making progression and checkpoint timing explicit instead of hidden in orchestration glue | adapt |
| LangGraph | lineage and history retrieval | thread state and history APIs expose checkpoint lineage, pending writes, and parent config | `langgraph/libs/langgraph/langgraph/pregel/main.py`, `langgraph/libs/sdk-py/langgraph_sdk/_sync/threads.py` | useful for audit reconstruction and rollback-oriented workflow views | adapt |
| LangGraph | saver capability contract | base saver defines list/get/put/put_writes/delete/copy/prune contract with conformance specs | `langgraph/libs/checkpoint/langgraph/checkpoint/base/__init__.py`, `langgraph/libs/checkpoint-conformance/...` | useful for designing a governance store against an explicit capability interface | adapt |
| Cline | semantic approval gating | approvals are keyed by command prefix, tool name, server name, or browser action and update pending tool state live | `cli/src/agent/permissionHandler.ts`, `cli/src/agent/ClineAgent.ts`, `cli/src/agent/messageTranslator.ts` | useful for durable approval policy that matches operation identity instead of transient request ids | adapt |
| Cline | checkpointed rollback plus history reconstruction | shadow git checkpoints are tied back to task messages and task history can be reconstructed from per-task artifacts | `src/integrations/checkpoints/*`, `src/core/commands/reconstructTaskHistory.ts`, `src/core/storage/StateManager.ts` | useful for restore and audit recovery when visible history or cache state drifts | adapt |
| Cline | file-backed hook policy surface | task lifecycle and tool lifecycle hooks can modify context or veto execution and are managed as workspace/global files | `src/core/hooks/*`, `src/core/task/tools/utils/ToolHookUtils.ts` | useful reference for operator-manageable policy surfaces outside prompt text | adapt |
| Roo Code | mode-bound delegation and command routing | command files declare `mode:` frontmatter and `new_task` is isolated as a structured delegation boundary | `.roomodes`, `.roo/commands/*`, `src/core/tools/NewTaskTool.ts` | useful for specialized lanes and structured handoff instead of one generic worker flow | adapt |
| Roo Code | defense in depth for tool use | prompt-time tool filtering is backed by execution-time validation including regex file restrictions and patch-path checks | `src/core/prompts/tools/filter-tools-for-mode.ts`, `src/core/tools/validateToolUse.ts` | useful for preventing tool drift and file-scope violations | copy |
| Roo Code | durable per-task history and resume | per-task `history_item.json` files are authoritative, `_index.json` is a cache, and resume avoids chaining from stale responses | `src/core/task-persistence/TaskHistoryStore.ts`, `src/core/task/Task.ts` | useful for session durability and parent/child resume safety | adapt |
| Roo Code | delegation return path | child task completion writes back to the parent and resumes it through ordered delegation events | `src/core/webview/ClineProvider.ts`, `src/extension/api.ts`, `src/core/tools/AttemptCompletionTool.ts` | useful for controlled work transfer without losing parent workflow state | adapt |
| SWE-agent | replayable trajectory artifact | every step persists a `.traj` bundle with history, trajectory, info, replay config, and environment | `sweagent/agent/agents.py`, `sweagent/run/run_replay.py` | useful for deterministic replay, postmortem analysis, and resumable batch work | copy |
| SWE-agent | structured post-action state capture | tool bundles can define `state_command` hooks whose JSON output is captured as first-class step state | `sweagent/tools/bundle.py`, `sweagent/tools/tools.py`, `sweagent/types.py` | useful for environment observability beyond stdout scraping | copy |
| SWE-agent | resumable batch orchestration | batch runs skip validated complete trajectories, purge corrupt ones, and track machine-readable exit status across runs | `sweagent/run/run_batch.py`, `sweagent/run/_progress.py` | useful for restart-safe large-scale governed execution | adapt |
| AgentScope | message hub fan-out | `MsgHub` subscribes agents for scoped fan-out and strips internal reasoning before rebroadcast | `src/agentscope/pipeline/_msghub.py`, `src/agentscope/agent/_agent_base.py` | useful for explicit coordination channels instead of ad hoc observe calls | copy |
| AgentScope | state modules with pluggable session backends | nested state modules auto-register and persist through JSON, Redis, or TableStore sessions | `src/agentscope/module/_state_module.py`, `src/agentscope/session/*` | useful for a typed runtime-state layer backed by interchangeable stores | adapt |
| AgentScope | toolkit and middleware governance | equipped tools are treated as absolute state and middleware wraps tool execution as an onion | `src/agentscope/tool/_toolkit.py` | useful for central tool governance and policy injection | adapt |
| BeeAI Framework | policy compiler over tool use | requirement rules compile into `allowedTools`, `hiddenTools`, `toolChoice`, and `canStop`, with cycle-breaking injected at runtime | `python/beeai_framework/agents/requirement/*`, `typescript/src/agents/requirement/*` | useful for deterministic tool governance over nondeterministic model behavior | copy |
| BeeAI Framework | inline permission requirement | permission checks are enforced by event interception before tool execution and can remember decisions | `python/beeai_framework/agents/requirement/requirements/ask_permission.py` | useful for durable approval gates without a separate preflight subsystem | copy |
| BeeAI Framework | protocol adapter registry | one server/factory model exposes agents and tools over MCP, A2A, and ACP surfaces | `python/beeai_framework/serve/server.py`, `python/beeai_framework/adapters/*/serve/server.py`, `typescript/src/adapters/*` | useful for separating internal runtime from external protocol surfaces | adapt |
| BeeAI Framework | session-aware handoff memory | cloned or read-only memory is attached per session and handoff strips system messages before delegation | `python/beeai_framework/serve/utils.py`, `python/beeai_framework/tools/handoff.py`, `python/beeai_framework/memory/readonly_memory.py` | useful for session isolation and safer work transfer | adapt |
| TaskWeaver | round and post lifecycle projection | session emits round/post start, end, error, status, routing, message, and attachment updates through an event-emitter proxy model | `taskweaver/taskweaver/module/event_emitter.py`, `taskweaver/taskweaver/session/session.py` | useful reference for step-level UX and audit projection without transcript parsing | observe |
| TaskWeaver | workspace-local prompt and artifact capture | session writes per-round JSON logs and per-post prompt logs, while code execution saves artifacts under the session workspace | `taskweaver/taskweaver/session/session.py`, `taskweaver/taskweaver/code_interpreter/code_executor.py` | useful for prompt/evidence capture and artifact materialization, though not as the primary workflow authority | adapt |
| TaskWeaver | verification feedback as typed attachments | code interpreter records verification and execution status in explicit attachment types and auto-enables verification in local mode | `taskweaver/taskweaver/code_interpreter/code_interpreter/code_interpreter.py`, `taskweaver/taskweaver/memory/attachment.py` | useful for separating verifier/executor feedback from freeform text | adapt |
| OpenAI Swarm | minimal handoff runtime | tool-call results can switch the active agent and merge context variables into the running loop without a separate orchestrator layer | `swarm/swarm/core.py`, `swarm/swarm/types.py` | useful as a lower-bound reference for handoff semantics and for testing what can be shrunk out of a control plane | observe |
| OpenAI Swarm | side-channel context injection | context variables are hidden from the model-visible tool schema and only passed to functions that declare them | `swarm/swarm/core.py` | useful as a minimal runtime-only context transport pattern, but too weak on auditability by itself | observe |
| PocketFlow | action-routed runtime with built-in retry/fallback | nodes split `prep/exec/post`, retry `exec` with wait/fallback, and flows follow action-labeled successors | `pocketflow/pocketflow/__init__.py` | useful as a tiny lower-bound orchestrator pattern with explicit control flow | adapt |
| PocketFlow | shared-store contract plus bounded parallelism | runtime separates shared store from immutable params and supports batch, async, and parallel variants while leaving persistence to the caller | `pocketflow/pocketflow/__init__.py`, `pocketflow/docs/core_abstraction/communication.md`, `pocketflow/docs/core_abstraction/parallel.md` | useful for explicit data-vs-compute separation and bounded fan-out, but not as a governance state model on its own | adapt |

## LangGraph

### Concrete Mechanisms Seen

- `StateGraph` compiles shared state plus reducers into an executable graph.
- checkpoint persistence is explicitly thread-scoped and version-aware.
- checkpoint metadata records provenance fields including source, step, parents, and run id.
- runtime control has explicit first-class types for interrupt, resume, goto, update, and task fan-out.
- thread management supports explicit ids, metadata merge, TTL, and copy-oriented `supersteps`.
- run management exposes durability modes, resumable streams, disconnect policy, future scheduling, and multitask strategies like reject, interrupt, rollback, and enqueue.
- runtime progression is barriered: prepare tasks, run tasks, apply writes, save checkpoint, then evaluate post-step interrupts.
- thread history and lineage carry `parent_config` and pending writes rather than flattening everything into the latest visible state.
- fork/copy is represented explicitly through checkpoint metadata source `fork` and copy-oriented update paths.
- saver copy/prune are part of the storage contract, even when concrete implementations are still uneven across saver backends.

### Code Anchors

- `langgraph/libs/langgraph/langgraph/graph/state.py`
- `langgraph/libs/langgraph/langgraph/pregel/_checkpoint.py`
- `langgraph/libs/checkpoint/langgraph/checkpoint/base/__init__.py`
- `langgraph/libs/langgraph/langgraph/types.py`
- `langgraph/libs/sdk-py/langgraph_sdk/_sync/threads.py`
- `langgraph/libs/sdk-py/langgraph_sdk/_sync/runs.py`
- `langgraph/libs/langgraph/langgraph/pregel/main.py`
- `langgraph/libs/langgraph/langgraph/pregel/_loop.py`
- `langgraph/libs/langgraph/langgraph/pregel/_algo.py`
- `langgraph/libs/langgraph/langgraph/pregel/_io.py`
- `langgraph/libs/langgraph/langgraph/callbacks.py`

### Why This Matters

- Handshake currently pays a high cost reconciling workflow truth across packet, task-board, runtime, and session surfaces.
- LangGraph’s checkpoint model is a cleaner example of durable execution truth with explicit provenance and replay seams.
- The `Interrupt` and `Command` types are especially relevant for governed pause, steer, resume, and re-route semantics.
- The thread and run client surfaces show how lifecycle policy can be explicit instead of being spread across bespoke orchestration helpers.
- The saver capability contract is also valuable: it suggests Handshake should define the storage interface first, then test implementations against it instead of letting storage semantics emerge accidentally.

### Open Questions

- how much of LangGraph’s runtime truth model depends on graph-native programming assumptions that do not map well onto repo-governance lanes
- whether thread-scoped checkpoint persistence alone is enough for audit-grade finalization or needs an external evidence layer
- how much of the barriered BSP execution model is useful directly versus only as inspiration for workflow checkpoint timing
- whether `checkpoint_ns` should map to lane/subflow/worktree concepts in Handshake or whether that would import too much graph-specific ancestry structure

## Open SWE

### Concrete Mechanisms Seen

- runtime identity is centered on deterministic `thread_id` values derived from source systems.
- sandbox backend and sandbox id are recovered through thread metadata, not only in-memory caches.
- queued follow-up messages are stored separately and injected before the next model step.
- PR creation exists both as an explicit terminal tool and as an after-agent middleware safety net.
- busy-state handling happens at ingress by checking whether the thread is already active and queueing work instead of colliding with the active run.
- restart/recovery uses a cache-first, probe-first model: reconnect by sandbox id, test liveness cheaply, recreate if stale, and write recovered identity back to thread metadata.
- terminal safety also includes a no-silent-finish guard that injects a minimal completion action when the model reaches the end of a run without a visible outcome.

### Code Anchors

- `open-swe/agent/server.py`
- `open-swe/agent/webapp.py`
- `open-swe/agent/utils/sandbox_state.py`
- `open-swe/agent/utils/github_comments.py`
- `open-swe/agent/middleware/check_message_queue.py`
- `open-swe/agent/middleware/open_pr.py`
- `open-swe/agent/middleware/ensure_no_empty_msg.py`
- `open-swe/agent/tools/commit_and_open_pr.py`

### Why This Matters

- Handshake currently burns time on stale runtime/session surfaces and on steering runs that are already mid-flight.
- Open SWE shows a practical thread-bound execution model that is closer to real repo work than abstract agent orchestration libraries.
- The message queue middleware is a direct candidate pattern for governed steer-next and operator relay without mutating active conversational truth in place.
- It also shows a credible pattern for completion safety: explicit terminal action plus fallback plus visible non-empty finish behavior.

### Open Questions

- whether relying on mutable thread metadata and `busy` thread status is strong enough for stricter governance-grade admission control
- how robust their queue semantics are under true multi-writer concurrency, because the current pattern still assumes a shared durable store and early delete discipline
- whether branch and repo identity are authoritative enough for audit reconstruction or still rely on surrounding platform state
- how much hidden operational burden is carried by their server/platform layer rather than the agent code alone

## Letta

### Concrete Mechanisms Seen

- tool policy is expressed as typed rule objects, not only prompts or ad hoc middleware.
- attaching a tool can automatically attach approval requirements.
- run management is persistent and filterable by status, project, agent, duration, and tool use.
- agent state and tool execution appear to use a deep-copy and commit-on-diff pattern rather than mutating primary state blindly.
- approval-required tools become explicit runtime stops with durable approval requests and matching response validation before execution can continue.
- persisted agent state includes operational fields like tool rules, message ids, last stop reason, and last run metrics instead of leaving them purely in runtime memory.
- monitoring and telemetry are first-class and separate from plain logs.

### Code Anchors

- `letta/letta/schemas/tool_rule.py`
- `letta/letta/services/run_manager.py`
- `letta/letta/services/agent_manager.py`
- `letta/letta/agent.py`
- `letta/letta/agents/helpers.py`
- `letta/letta/services/tool_executor/sandbox_tool_executor.py`
- `letta/letta/services/step_manager.py`
- `letta/letta/orm/agent.py`

### Why This Matters

- Handshake governance currently embeds a lot of authority and approval behavior in procedural choreography.
- Letta is a strong example of encoding governance policy and approval logic as explicit stateful objects.
- The separation between persistent agent records, run records, and tool rules is useful for swarm-safe coordination and post-run analysis.
- The approval-request plus idempotent resume flow is one of the clearest external patterns for governed interruption that should survive retries and reconnects.
- The copy-on-execute pattern is especially relevant for any side-effecting governance tool or validator action.

### Open Questions

- how far Letta’s rule model can express workflow authority, role boundaries, and closeout semantics versus mostly tool-level restrictions
- whether Handshake needs durable approval requests only for high-risk mutations or as a more general workflow stop primitive
- whether their persistence and metric layers are reusable conceptually without adopting their full agent platform

## Cline

### Concrete Mechanisms Seen

- approval is keyed to semantic operation identity such as command prefix, tool name, MCP server name, or browser ask, not only a transient request id
- tool admission has both a central allowed-tool universe and runtime coordination for dynamic namespaces like MCP tools and subagent tools
- hooks are first-class lifecycle objects for task start, resume, cancel, complete, pre-tool, post-tool, user prompt submit, notification, and pre-compact
- pre-tool hooks can veto execution explicitly rather than only decorate context
- session-local pending tool state is split from task-local recovery state so async UI and tool streams can be reconciled safely
- task persistence separates API history, UI messages, task metadata, and global task history, then reconstructs history from per-task artifacts if the global index drifts
- checkpoints use a shadow git repo per workspace hash and anchor restore points back into task messages and history
- cancellation is treated as a controlled state transition with cleanup, revert, and optional resume instead of a hard kill

### Code Anchors

- `cli/src/agent/permissionHandler.ts`
- `cli/src/agent/ClineAgent.ts`
- `cli/src/agent/messageTranslator.ts`
- `cli/src/agent/public-types.ts`
- `src/shared/tools.ts`
- `src/core/task/tools/ToolExecutorCoordinator.ts`
- `src/core/hooks/hook-factory.ts`
- `src/core/hooks/hook-executor.ts`
- `src/core/task/tools/utils/ToolHookUtils.ts`
- `src/core/storage/StateManager.ts`
- `src/core/commands/reconstructTaskHistory.ts`
- `src/integrations/checkpoints/CheckpointTracker.ts`
- `src/core/controller/checkpoints/checkpointRestore.ts`

### Why This Matters

- Cline is one of the stronger references for making approvals, hooks, checkpoints, and task-history recovery into explicit control-plane primitives instead of hidden side effects.
- The pairing of checkpoint hashes with message history is especially relevant to Handshake because it ties restorable repo state back to visible workflow state.
- Its hook surface is also a strong external reference for operator-manageable policy injection without having to hard-code every governance branch into the orchestrator runtime.

### Open Questions

- how durable the approval state really is across host restart, because much of the always-allow memory appears to be process-local
- whether its split between UI history and API history is a useful governance pattern or too extension-specific for Handshake
- how much of its checkpoint approach depends on editor-host assumptions that would not carry cleanly into a repo-native control plane

## Roo Code

### Concrete Mechanisms Seen

- mode files and command frontmatter create explicit specialized entrypoints rather than one monolithic agent surface
- built-in and custom modes merge through a constrained composition layer with regex-based file restriction errors
- prompt-time tool filtering is backed by execution-time validation, including path extraction from `apply_patch` payloads
- approval controls are granular by action category, workspace sensitivity, and command allow or deny list
- API/profile locking across modes prevents mode switches or history restores from silently rewriting execution configuration
- task history is stored per task as the authoritative record while an index file acts only as a cache
- resume logic avoids chaining from the wrong prior response and refreshes environment details on resume
- delegation is explicit: parent metadata is persisted before child start, the child writes back a result or note, and ordered events resume the parent task
- `new_task` is isolated as a single-tool boundary with runtime enforcement to prevent mixed tool execution around delegation
- rollback checkpoints use a separate per-task repo service with sanitized git environment and protected-workspace checks

### Code Anchors

- `.roomodes`
- `.roo/commands/*`
- `src/shared/modes.ts`
- `src/core/prompts/tools/filter-tools-for-mode.ts`
- `src/core/tools/validateToolUse.ts`
- `src/core/auto-approval/index.ts`
- `src/core/task-persistence/TaskHistoryStore.ts`
- `src/core/task/Task.ts`
- `src/core/tools/NewTaskTool.ts`
- `src/services/checkpoints/ShadowCheckpointService.ts`

### Why This Matters

- Roo Code is the closest direct external reference so far for specialized modes, structured delegation, and durable parent-child task resumption.
- It also has one of the cleanest defense-in-depth stories for tool use: prompt filtering, runtime validation, file restriction checks, and action-category approvals.
- The `new_task` isolation rule is especially relevant to Handshake if work transfer needs to become a strict control-plane transition rather than another ordinary tool call.

### Open Questions

- whether the mode layer is too UI-product-centric to reuse directly in a repo-governance runtime
- how much complexity is introduced by keeping mode config, tool filters, approvals, and profile locks all in separate settings surfaces
- whether the per-task history model scales cleanly to multi-actor workflow ledgers or mainly to extension task history

## SWE-agent

### Concrete Mechanisms Seen

- the main loop saves trajectory data after every step, making the `.traj` bundle the durable replay surface rather than plain logs
- error handling and retry control are explicit in the agent loop, with internal retry and exit tokens that steer control flow after format, timeout, cost, and environment errors
- retry agents and reviewers can compare attempts under explicit budget limits and select the best candidate
- environment reset is strict: deployments are restarted, repos are recopied or hard reset, and dirty local repos are rejected
- tool bundles can declare `state_command` hooks whose JSON output becomes first-class step state
- batch runs skip complete trajectories, purge corrupt ones, and track exit statuses in machine-readable reports
- replay reconstructs a run from stored replay config and trajectory actions so failures can be re-executed deterministically
- human step-in and step-out can transfer control between AI and a human model mid-run
- task-source normalization turns different workload sources into one batch instance shape

### Code Anchors

- `sweagent/agent/agents.py`
- `sweagent/agent/reviewer.py`
- `sweagent/environment/swe_env.py`
- `sweagent/environment/repo.py`
- `sweagent/tools/bundle.py`
- `sweagent/tools/tools.py`
- `sweagent/run/run_batch.py`
- `sweagent/run/_progress.py`
- `sweagent/run/run_replay.py`
- `sweagent/run/run_traj_to_demo.py`
- `sweagent/run/run_shell.py`

### Why This Matters

- SWE-agent is one of the clearest references for turning every run into a durable replay artifact instead of relying on mixed runtime ledgers and later reconstruction.
- Its post-action `state_command` hook is especially strong: it gives you structured environment truth after each action without teaching the model to narrate that truth itself.
- The strict environment reset and resumable batch logic are also relevant to Handshake if swarm execution needs to survive interrupted runs without contaminating later work.

### Open Questions

- whether the benchmark lineage of the project makes some of its run and reporting surfaces too evaluation-centric for daily governed product work
- how well the `.traj` artifact model maps onto work-transfer and validator-handshake flows, not just agent replay
- whether the internal control tokens are a robust design pattern or a project-local workaround that should not be imported directly

## AgentScope

### Concrete Mechanisms Seen

- `MsgHub` creates a scoped fan-out channel so agents can broadcast to subscribed peers without manually wiring every `observe()` edge
- reply rebroadcast strips internal reasoning before propagation
- nested `StateModule` attributes auto-register and persist through interchangeable JSON, Redis, or TableStore session backends
- tool governance is centralized in `Toolkit`, which treats equipped tools as absolute state and wraps execution in middleware layers
- MCP clients preserve state across calls and normalize MCP responses into AgentScope blocks
- A2A integration preserves observed messages, task status, and artifacts while resolving agents from well-known cards or Nacos
- tracing covers agent, tool, model, formatter, and generic functions, with optional Studio streaming and file logging
- evaluation storage skips already completed work and persists run artifacts keyed by task and repeat ids

### Code Anchors

- `src/agentscope/pipeline/_msghub.py`
- `src/agentscope/agent/_agent_base.py`
- `src/agentscope/module/_state_module.py`
- `src/agentscope/session/*`
- `src/agentscope/tool/_toolkit.py`
- `src/agentscope/mcp/*`
- `src/agentscope/a2a/*`
- `src/agentscope/tracing/*`
- `src/agentscope/evaluate/*`

### Why This Matters

- AgentScope is a strong reference for explicit coordination channels and typed runtime state with interchangeable backends.
- The `MsgHub` model is directly relevant to swarm coordination because it turns agent-to-agent propagation into a scoped control-plane primitive.
- Its state-module layer is also useful as a reminder that runtime state can be typed and backend-neutral instead of becoming a loose pile of sidecar JSON files.

### Open Questions

- how much of the session backend model is production-hardened versus tutorial-friendly abstraction
- whether `MsgHub` is sufficient for governed work transfer or mostly useful for in-process collaboration patterns
- how much of the observability surface is governance-relevant versus mostly evaluation and Studio integration support

## BeeAI Framework

### Concrete Mechanisms Seen

- requirement rules are compiled into `allowedTools`, `hiddenTools`, `toolChoice`, and `canStop` before execution
- runtime injects cycle-breaking requirements when tool usage gets stuck
- `AskPermissionRequirement` enforces inline approval by intercepting tool-start events and can remember decisions or always allow selected tools
- trajectory middleware records nested execution trees while stream middleware emits partially formed tool calls as structured updates
- memory attaches per session through clone and reset semantics, with read-only wrappers available for delegated agents
- handoff strips system messages and forwards only the safe conversation prefix into delegated agents
- workflows advance through explicit navigation tokens like `START`, `SELF`, `NEXT`, `PREV`, and `END`
- one factory registry exposes agents and tools over MCP, A2A, and ACP server surfaces
- the TypeScript MCP HTTP server keeps resumable transports keyed by session id while the Python side reference-counts session wrappers for cleanup

### Code Anchors

- `python/beeai_framework/agents/requirement/requirements/requirement.py`
- `python/beeai_framework/agents/requirement/requirements/conditional.py`
- `python/beeai_framework/agents/requirement/_runner.py`
- `python/beeai_framework/agents/requirement/requirements/ask_permission.py`
- `python/beeai_framework/middleware/trajectory.py`
- `python/beeai_framework/middleware/stream_tool_call.py`
- `python/beeai_framework/serve/utils.py`
- `python/beeai_framework/tools/handoff.py`
- `python/beeai_framework/workflows/workflow.py`
- `python/beeai_framework/serve/server.py`
- `python/beeai_framework/adapters/*/serve/server.py`
- `typescript/src/adapters/mcp/serve/http_server.ts`

### Why This Matters

- BeeAI is one of the strongest code references so far for deterministic tool governance over an otherwise nondeterministic model loop.
- Its rule compiler and inline permission requirement are directly relevant to Handshake because they externalize execution policy instead of burying it in prompts or ad hoc orchestration logic.
- The shared server and adapter model is also relevant because it separates the internal runtime from protocol surfaces cleanly enough to support MCP, A2A, and ACP without rebuilding the core loop each time.

### Open Questions

- whether the requirement model is expressive enough for workflow authority and validator roles, not only tool governance
- how much complexity is hidden in the mirrored Python and TypeScript implementations if Handshake only wants one control-plane language
- whether its workflow navigation model is clean enough for repo-governance lanes or better treated as a simpler orchestration reference

## Preliminary Read

### Mechanisms Already Strong Enough To Carry Forward

- LangGraph checkpoint provenance and interrupt/resume control surfaces
- Open SWE thread-scoped execution identity and queued follow-up injection
- Letta typed policy rules and auto-attached approval requirements
- Letta approval-request plus idempotent resume flow
- Cline file-backed hook lifecycle and checkpoint-to-message linkage
- Roo Code isolated `new_task` delegation and defense-in-depth tool enforcement
- SWE-agent replayable `.traj` bundles and post-action `state_command` capture
- AgentScope `MsgHub` scoped fan-out and pluggable state backends
- BeeAI rule-compiled tool governance and inline permission requirements
- PocketFlow action-routed runtime plus built-in retry/fallback as a lower-bound orchestrator reference
- smolagents executor isolation and safe-default serialization boundary
- OpenHands `.pr/` artifact discipline for temporary non-canonical review material

### Mechanisms That Need Another Pass Before Adoption Judgment

- Open SWE recovery behavior under server restart, stale sandbox, or partial PR failure
- LangGraph storage implementation reality for copy/prune beyond the declared capability contract
- the smallest viable mapping from LangGraph thread/checkpoint semantics onto repo-governance objects without importing graph-specific complexity
- Letta’s rule vocabulary mapped onto Handshake governance boundaries rather than generic tool families

### Additional Follow-Up Risks

- Cline approval durability across host restart and non-editor control surfaces
- Roo Code mode complexity versus the smaller Handshake governance surface
- AgentScope session backends and `MsgHub` behavior under true multi-writer or distributed governance flows
- BeeAI requirement expressiveness for validator authority, closeout, and workflow-level stops
- TaskWeaver's richer event and artifact surfaces sit on top of an in-memory session store, so its runtime durability story is much weaker than its UX projection story
- OpenAI Swarm is intentionally too thin for governance by itself, so it is more useful as a shrink-floor than as an adoption target

## Next Inspection Candidates

- `AutoGen`
- `AG2`
- `CrewAI`
- `Semantic Kernel`
- `PydanticAI`
- `ChatDev`
- `AutoGPT`
- `OWL`
- `Letta Code`
- `Microsoft Agent Framework`
- deeper follow-up on `LangGraph`, `Open SWE`, `Letta`, `Cline`, `Roo Code`, `AgentScope`, and `BeeAI` only where a mechanism already looks adoption-worthy

## OpenHands

### Concrete Mechanisms Seen

- repository instructions explicitly reserve `.pr/` for temporary PR-only artifacts such as design notes, investigation logs, and reviewer-facing evidence.
- `.pr/` is treated as non-blocking during active work but wired into notification and post-approval cleanup.
- resolver flows persist structured resolution outputs containing issue identity, base commit, patch, history, metrics, and success explanation.
- the current resolver stack being inspected is explicitly marked legacy, which matters when judging how much weight to give its runtime design.

### Code Anchors

- `openhands/AGENTS.md`
- `openhands/openhands/resolver/README.md`
- `openhands/openhands/resolver/resolver_output.py`
- `openhands/openhands/resolver/issue_resolver.py`
- `openhands/openhands/resolver/send_pull_request.py`

### Why This Matters

- Handshake needs lighter-weight hygiene for temporary review artifacts and run evidence that should not become canonical governance truth.
- OpenHands is a useful reference for that distinction even if its current resolver runtime is not the main design to copy.

### Open Questions

- whether the newer V1 runtime preserves the same artifact discipline in a cleaner control-plane design
- how much of the resolver pipeline should count as legacy historical context versus active adoption candidate

## smolagents

### Concrete Mechanisms Seen

- tool validation checks for self-contained source, rebuildable defaults, and constrained class attributes.
- tool execution supports parallel tool calls while preserving explicit memory-step recording.
- code execution is isolated behind an executor abstraction with local and several remote backends.
- remote execution defaults to safe serialization and requires explicit opt-in for pickle fallback.
- agents can be exported as a portable folder with tools, prompts, and config, which makes replay and inspection easier.

### Code Anchors

- `smolagents/src/smolagents/tool_validation.py`
- `smolagents/src/smolagents/agents.py`
- `smolagents/src/smolagents/remote_executors.py`
- `smolagents/src/smolagents/serialization.py`

### Why This Matters

- Handshake repo governance currently has weak boundaries around what should be portable, inspectable, and safe to execute remotely.
- smolagents is a useful reference for keeping execution isolation and serialization trust explicit.
- its validator is also a strong reminder that reusable governance tools should be reconstructable from source rather than hiding critical init-time state.

### Open Questions

- whether its executor abstraction is too code-agent-centric for repo-governance workflows that need stronger durable orchestration state
- how much of its memory and save model is useful beyond packaging/export convenience

## TaskWeaver

### Concrete Mechanisms Seen

- sessions create a dedicated workspace plus execution cwd and dump both per-round JSON logs and per-post prompt logs under that session workspace.
- conversation state is stored as rounds and posts, with failed rounds optionally filtered out for later role views.
- shared state inside the conversation is carried through typed `shared_memory_entry` attachments rather than only through freeform message text.
- event emission is explicit and hierarchical: session, round, and post events include status updates, send-to changes, message deltas, attachment updates, and errors.
- code execution saves generated artifacts into the session workspace and surfaces verification and execution outcomes as typed attachments.
- experience and example loading can be redirected by shared-memory subpath entries, which is a concrete example of plan-time state steering later retrieval.
- session management is still process-local: the session store is in-memory only and the commented resume/fork path is not yet durable.

### Code Anchors

- `taskweaver/taskweaver/session/session.py`
- `taskweaver/taskweaver/memory/memory.py`
- `taskweaver/taskweaver/module/event_emitter.py`
- `taskweaver/taskweaver/app/session_store.py`
- `taskweaver/taskweaver/app/session_manager.py`
- `taskweaver/taskweaver/code_interpreter/code_executor.py`
- `taskweaver/taskweaver/code_interpreter/code_interpreter/code_interpreter.py`
- `taskweaver/taskweaver/role/role.py`

### Why This Matters

- TaskWeaver is not a strong durable-governance reference, but it is a useful reference for rich step-level projection surfaces.
- The event taxonomy and attachment model are especially relevant if Handshake wants operator-visible progress, verification, execution, and artifact updates without relying on transcript parsing.
- Its workspace-local prompt logs are also a useful reminder that prompt capture can be tied to a concrete run surface instead of being hidden inside tracing exporters or chat logs.

### Open Questions

- whether the attachment taxonomy is worth copying directly or should be normalized into Handshake-native event and evidence schemas
- whether per-post prompt-log capture is worth the storage cost once a stronger Flight Recorder exists
- how much of TaskWeaver's richer interaction surface matters once you remove the in-memory-only session model and data-analysis-specific runtime

## OpenAI Swarm

### Concrete Mechanisms Seen

- the runtime loop is intentionally tiny: prepend agent instructions, call the model, execute tool calls, update context, optionally switch the active agent, and continue until the turn ends.
- tool functions are surfaced as JSON tools, and `parallel_tool_calls` is configured per agent.
- `context_variables` are hidden from the model-visible tool schema and only passed into functions that explicitly declare them.
- tool-call results can carry a replacement `Agent` and additional context variables, which makes agent handoff a plain tool-result effect rather than a separate orchestration subsystem.
- the durable state surface is minimal: the run deep-copies message history and context variables, appends new messages, and returns them as the response.

### Code Anchors

- `swarm/swarm/core.py`
- `swarm/swarm/types.py`

### Why This Matters

- Swarm is the useful lower bound for this research: it shows the smallest handoff runtime that still has a coherent agent-switching model.
- That makes it valuable as a shrink target when evaluating whether more elaborate control-plane machinery is actually necessary.
- It is not a direct governance reference because it does not provide durable workflow state, approval stops, audit-grade replay, or lifecycle controls beyond the current message loop.

### Open Questions

- whether any part beyond handoff semantics and runtime-only context transport is worth carrying into Handshake
- whether the minimal loop remains useful once governed approvals, restart safety, validator authority, and evidence capture are introduced

## PocketFlow

### Concrete Mechanisms Seen

- the runtime enforces a clean `prep -> exec -> post` split for nodes and follows explicit action-labeled successors in flows.
- retry, wait, and fallback behavior are part of the base node runtime rather than being left to prompt instructions.
- batch, async, and async-parallel variants are first-class, which makes fan-out an explicit runtime choice instead of an ad hoc loop in user code.
- the communication model is explicit about the difference between a shared store for durable shared data and immutable params for per-iteration identity.
- the framework stays intentionally small and pushes persistence, schemas, and storage policy to the application layer.
- human-in-the-loop behavior in the cookbook is modeled as an ordinary branch-and-loop flow rather than a special runtime primitive.

### Code Anchors

- `pocketflow/pocketflow/__init__.py`
- `pocketflow/docs/core_abstraction/communication.md`
- `pocketflow/docs/core_abstraction/parallel.md`
- `pocketflow/cookbook/pocketflow-cli-hitl/README.md`

### Why This Matters

- PocketFlow is a useful lower-bound orchestration substrate for explicit control flow, retry policy, and bounded parallelism.
- The shared-store-vs-params split is also a useful design constraint for Handshake because it forces data schema and compute logic apart.
- It is not a sufficient governance runtime by itself because persistence, approvals, evidence, and durable coordination remain caller responsibilities.

### Open Questions

- whether PocketFlow's simplicity survives once workflow truth, authority policy, and replay evidence become first-class requirements
- whether the framework's deliberately externalized persistence model is a strength for Handshake or simply pushes too much governance burden back onto the product

## Batch 2 Intake

This is a compact preservation block for the second code-inspection wave. It is intentionally less normalized than the sections above so the mechanism signal is captured before the next comparison pass.

### AutoGen

- versioned Pydantic state envelopes plus runtime-wide save/load give it one of the cleaner durable state models in this batch
- resume is explicit through `on_resume`, separate from raw state load
- handoff is a first-class tool boundary, including user-proxy handoff and nested agent containers
- runtime intervention handlers can structurally drop messages before send, publish, or response processing
- code execution can require explicit sync or async approval callbacks
- selector and Magentic One orchestration add stall detection, replanning, and progress-ledger behavior
- trace propagation is strong: spans flow across runtime, agents, and tools with call ids and exception metadata
- serialization is disciplined: ad hoc tools and approval callbacks are excluded from serializable config/state surfaces

Key anchors:

- `python/packages/autogen-agentchat/src/autogen_agentchat/state/_states.py`
- `python/packages/autogen-core/src/autogen_core/_single_threaded_agent_runtime.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/base/_handoff.py`
- `python/packages/autogen-core/src/autogen_core/_intervention.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/agents/_code_executor_agent.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/teams/_group_chat/_selector_group_chat.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/teams/_group_chat/_magentic_one/_magentic_one_orchestrator.py`

### AG2

- shared `ContextVariables` are passed by reference across agents, manager, tool executor, and optional user proxy
- group chat has explicit resume and last-speaker reconstruction from persisted message state
- routing is a stateful precedence-ordered transition pipeline, not only round-robin speaker selection
- function targets bundle side effects, context updates, outbound messages, and next-target routing in one handoff result
- internal transfer tool calls can be scrubbed from the visible transcript while still driving routing
- safeguard enforcement is one of the strongest policy surfaces seen so far: it spans agent transitions, tool I/O, model I/O, and user I/O, with schema validation and event emission
- A2A support includes task-anchored recovery and artifact normalization; AG-UI streaming preserves UI-safe serialized state

Key anchors:

- `autogen/agentchat/conversable_agent.py`
- `autogen/agentchat/groupchat.py`
- `autogen/agentchat/group/group_utils.py`
- `autogen/agentchat/group/targets/function_target.py`
- `autogen/agentchat/group/safeguards/enforcer.py`
- `autogen/agentchat/group/safeguards/validator.py`
- `autogen/a2a/server.py`
- `autogen/a2a/client.py`
- `autogen/ag_ui/adapter.py`

### CrewAI

- runtime state is checkpointable with provider abstraction, lineage, branching, and migration on restore
- checkpoint saving is policy-driven through a listener on the event bus rather than being scattered across runtime code
- flow persistence separates ordinary flow state from pending human-feedback state
- crews can resume from checkpoints, fork, replay, and auto-create manager agents for hierarchical execution
- delegation is explicit via coworker tools rather than an implicit side channel
- tool execution includes hooks, usage caps, result-as-answer termination, and output guardrails with retry budgets
- A2A delegation is more coupled but technically rich: context ids, task ids, reference ids, and transport-specific completion recovery are preserved
- event DAG plus tracing listeners and telemetry gives it one of the strongest reconstructable audit surfaces in the set
- MCP discovery/execution is cached, retried, and transport-pluggable; file/path hygiene is strong with SSRF and traversal protection

Key anchors:

- `lib/crewai/src/crewai/state/runtime.py`
- `lib/crewai/src/crewai/state/checkpoint_listener.py`
- `lib/crewai/src/crewai/flow/persistence/sqlite.py`
- `lib/crewai/src/crewai/crew.py`
- `lib/crewai/src/crewai/tools/agent_tools/*`
- `lib/crewai/src/crewai/agents/crew_agent_executor.py`
- `lib/crewai/src/crewai/a2a/wrapper.py`
- `lib/crewai/src/crewai/state/event_record.py`
- `lib/crewai/src/crewai/mcp/tool_resolver.py`

### Semantic Kernel

- orchestration runs are isolated on per-invocation internal topics instead of one global conversation channel
- sequential, concurrent, handoff, and group-chat orchestrators are explicit and composable
- handoff is implemented as generated transfer tools, not only a routing table
- group chat exposes explicit manager gates for user-input requests, termination, next-agent selection, and result filtering
- Magentic orchestration tracks round, stall, reset counts, and a progress ledger, then replans or resets threads on stall
- process state is versioned and hierarchical, with step and process metadata reconstructed from live trees
- the Python sessions plugin is a useful safety boundary reference: deny-by-default upload policy, allowed directories, and canonical remote file paths
- telemetry is OpenTelemetry-native and tries to preserve message ordering while redacting payloads unless explicitly enabled

Key anchors:

- `python/semantic_kernel/agents/orchestration/orchestration_base.py`
- `python/semantic_kernel/agents/orchestration/handoffs.py`
- `python/semantic_kernel/agents/orchestration/group_chat.py`
- `python/semantic_kernel/agents/orchestration/magentic.py`
- `python/semantic_kernel/processes/kernel_process/kernel_process_state.py`
- `python/semantic_kernel/core_plugins/sessions_python_tool/sessions_python_plugin.py`
- `python/semantic_kernel/utils/telemetry/model_diagnostics/*`

### PydanticAI

- run state is mirrored across graph state and tool context, including approval metadata and step counters
- tool deferral and approval are first-class serialized control-flow states, not implicit pauses
- `ApprovalRequiredToolset` is a clean reusable pattern: approval becomes a wrapper over any toolset
- output typing can explicitly allow or forbid deferred-tool requests, which moves some governance policy into schema boundaries
- graph persistence captures node/end snapshots with explicit lifecycle statuses
- beta graph orchestration includes fork/join tracking and sibling cancellation across nested fan-out/fan-in
- durable execution integrations (Temporal, DBOS, Prefect) show a serious attempt to move model/tool work into workflow activities safely
- UI adapters preserve provider metadata, approvals, and streamed tool events across frontend protocols
- MCP caching and invalidation are explicit, with durable-execution caveats acknowledged at the wrapper layer

Key anchors:

- `pydantic_ai_slim/pydantic_ai/_run_context.py`
- `pydantic_ai_slim/pydantic_ai/_agent_graph.py`
- `pydantic_ai_slim/pydantic_ai/toolsets/approval_required.py`
- `pydantic_graph/pydantic_graph/persistence/__init__.py`
- `pydantic_graph/pydantic_graph/beta/graph.py`
- `pydantic_ai_slim/pydantic_ai/durable_exec/temporal/*`
- `pydantic_ai_slim/pydantic_ai/ui/*`
- `pydantic_ai_slim/pydantic_ai/mcp.py`

### Batch 2 Read

- `AutoGen`, `AG2`, and `CrewAI` all have stronger-than-average answers for governed work transfer and restart-safe orchestration.
- `Semantic Kernel` is especially valuable for explicit orchestration shapes, progress-ledger recovery, and safe session/file boundaries.
- `PydanticAI` is strongest on approval-as-data, durable tool deferral, and schema-shaped control flow.
- The second wave reinforces a pattern already visible in batch one: the best harnesses do not treat approval, handoff, and recovery as chat conventions. They make them typed runtime objects with explicit storage and resume semantics.

## Batch 3 Intake

### Mastra

- approvals are resumable execution states with typed resume schemas, not one-off prompts
- suspended tool calls can be auto-resumed from stored thread memory rather than process-local state
- background tasks are durable runtime objects with retry, timeout, cancel, and stale-task recovery paths
- thread handoff is lock-ordered during cloning, which is a real anti-split-brain runtime control
- governance is expressed through harness-native tools like `ask_user`, `submit_plan`, `task_write`, and `task_check`
- observability carries a strong correlation envelope across run, thread, resource, session, request, and user/org fields
- MCP transport is session-aware and approval-capable on both server and client sides
- config/version history is git-backed and reconstructable, which is unusually strong for operator-auditable persistence

Key anchors:

- `packages/core/src/loop/network/index.ts`
- `packages/core/src/background-tasks/manager.ts`
- `packages/core/src/background-tasks/create.ts`
- `packages/core/src/harness/harness.ts`
- `packages/core/src/harness/tools.ts`
- `packages/core/src/observability/context.ts`
- `packages/core/src/observability/types/core.ts`
- `packages/core/src/storage/domains/observability/tracing.ts`
- `packages/mcp/src/server/server.ts`
- `packages/mcp/src/client/client.ts`
- `packages/core/src/storage/filesystem-versioned.ts`

### MetaGPT

- team-level checkpointing and resume is real and based on serialized team/context state
- polymorphic serialization preserves subclass behavior across checkpoint boundaries
- role recovery removes the newest observed message on failure so replay can safely retry the same input
- plan state is executable and dependency-aware rather than just a generated todo list
- human review is an actual acceptance gate via `AskReview` and planner confirmation flow
- `RoleZero` uses command-policy filtering with exclusive/special command maps to constrain execution
- tool governance combines registry validation with bounded recommendation/ranking
- experience caching and long-term memory persistence are explicit reuse layers, not just longer transcripts
- structured reporters for thought/task/file/browser/terminal events create a practical audit surface

Key anchors:

- `metagpt/team.py`
- `metagpt/context.py`
- `metagpt/schema.py`
- `metagpt/roles/role.py`
- `metagpt/strategy/planner.py`
- `metagpt/actions/di/ask_review.py`
- `metagpt/roles/di/role_zero.py`
- `metagpt/tools/tool_registry.py`
- `metagpt/tools/tool_recommend.py`
- `metagpt/exp_pool/decorator.py`
- `metagpt/memory/role_zero_memory.py`
- `metagpt/utils/report.py`

### CAMEL

- agent memory is serializable and cloneable rather than being locked to a single live process
- agent and session identity is carried through async execution via context-local fields and tracing hooks
- workforce orchestration has a real lifecycle state machine with pause/resume/stop/snapshot/restore
- task handoff is formalized through task/channel state instead of prompt-only delegation
- failure handling is policy-driven with retry, replan, decompose, create-worker, and reassign strategies
- immutable events plus callbacks and JSON logging give it one of the cleaner orchestration audit surfaces
- reusable workflow memory is stored as versioned markdown artifacts with metadata
- tool execution can be risk-scored and explicitly overridden, which is a useful split between policy and exception
- HITL is first-class through human toolkit calls and reducer logic
- runtimes can be local Docker, remote HTTP, or MCP-exposed

Key anchors:

- `camel/societies/workforce/workforce.py`
- `camel/societies/workforce/task_channel.py`
- `camel/societies/workforce/events.py`
- `camel/societies/workforce/workforce_logger.py`
- `camel/societies/workforce/workflow_memory_manager.py`
- `camel/agents/chat_agent.py`
- `camel/utils/langfuse.py`
- `camel/runtimes/llm_guard_runtime.py`
- `camel/runtimes/docker_runtime.py`
- `services/agent_mcp/agent_mcp_server.py`

### Agency Swarm

- the main durable primitive is a flat message ledger with explicit persist/load hooks
- replay compatibility is guarded by stamped metadata like agent, caller, run ids, trace ids, and history protocol
- nested runs propagate parent/child lineage explicitly rather than relying on loose prompt inheritance
- delegation is a declared communication graph that becomes runtime `send_message` tools or handoff objects
- some agents are hard-bounded as receive-only, which is a concrete governance boundary for worker roles
- tool loading, schema parsing, validation, and single-call guards are handled in code rather than convention
- streaming persistence normalizes unstable IDs, deduplicates repeated items, and reconciles streamed versus final artifacts
- request-scoped logging, citation extraction, and usage tracking create a stronger audit/cost surface than most org-graph frameworks

Key anchors:

- `src/agency_swarm/utils/thread.py`
- `src/agency_swarm/messages/message_formatter.py`
- `src/agency_swarm/agent/execution.py`
- `src/agency_swarm/agent/execution_streaming.py`
- `src/agency_swarm/agent/execution_stream_persistence.py`
- `src/agency_swarm/agency/setup.py`
- `src/agency_swarm/agent/subagents.py`
- `src/agency_swarm/agent/tools.py`
- `src/agency_swarm/tools/concurrency.py`
- `src/agency_swarm/messages/message_filter.py`
- `src/agency_swarm/utils/usage_tracking.py`

### Langroid

- chat artifacts are lineage-bearing objects with parent/child identity and registry-backed deletion helpers
- tasks support an external session kill flag, which is a lightweight but real control plane
- hosted OpenAI assistant/thread ids can be cached and reused, giving it one of the few off-process resume hooks in the set
- orchestration is a responder search over agent plus subtask responders with explicit stall detection
- human participation is first-class and can be interactive or defaulted, depending on run mode
- control-flow tools are governed separately for generation versus handling, which is a strong default-deny pattern
- handoff/delegation uses dedicated orchestration tools with recipient metadata and subtask spawning
- rewind exists as history pruning plus descendant invalidation, though still process-local
- TSV plus HTML traces preserve sender/recipient/tool metadata for readable audit reconstruction

Key anchors:

- `langroid/agent/task.py`
- `langroid/agent/chat_document.py`
- `langroid/agent/base.py`
- `langroid/agent/chat_agent.py`
- `langroid/agent/tools/orchestration.py`
- `langroid/agent/tools/task_tool.py`
- `langroid/agent/tools/rewind_tool.py`
- `langroid/agent/openai_assistant.py`
- `langroid/utils/html_logger.py`
- `langroid/utils/logging.py`

### Batch 3 Read

- `Mastra` is one of the strongest answers so far for resumable approval state, durable background work, and correlation plumbing.
- `CAMEL` is more novel than expected on workforce lifecycle, recovery strategy, workflow memory artifacts, and runtime-level risk gating.
- `Agency Swarm` is especially interesting for explicit communication-graph delegation, lineage-heavy message metadata, and request-scoped override cleanup.
- `Langroid` is a strong lower-weight reference for typed orchestration tools, history rewind, and generation-versus-handling tool governance.
- `MetaGPT` is useful less as a modern distributed runtime and more as a code reference for checkpointable org/team execution, plan state, command policy, and structured reporting.
- The third wave keeps reinforcing the same design lesson: the reusable ideas are rarely the headline "multi-agent" framing. They are the concrete runtime controls around identity, approval, replay, handoff, and audit.
