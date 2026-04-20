# Typed Runtime Resume and Approval Working Notes

Temporary working file for the second-wave runtime pass focused on typed state, restart-safe resume, deferred tool execution, and human approval objects.
This is not a final architecture document. It is a mechanism-level extraction to answer the next Handshake governance design questions with code-backed evidence.

## Purpose

- inspect concrete runtime object shapes instead of broad agent-framework summaries
- identify what each framework actually persists, restores, or branches
- separate real approval-as-data mechanisms from prompt-time or schema-only "human in loop" claims
- decide which patterns should be `copy`, `adapt`, `observe`, or `reject` for Handshake governance

## Working Questions

1. what is the actual stored object for run, team, or workflow state
2. how is resume or restore keyed and carried forward
3. what is the actual object for approval, deferred execution, or user input stop
4. which mechanisms survive restart, replay, or branch creation without depending on transcript reconstruction

## Scope

- code inspected from local clones under `%TEMP%\handshake-harness-inspect`
- targeted frameworks:
  - `PydanticAI`
  - `Semantic Kernel`
  - `AutoGen`
  - `CrewAI`

## Quick Conclusion

- `PydanticAI` is the strongest direct reference for approval-as-data and deferred tool execution. It has explicit typed request and result objects, explicit approval and denial results, and a clean resume path that reuses tool call ids.
- `Semantic Kernel` is stronger on state persistence contracts, step-state migration, and durable process state than on generic approval objects. Its workflow `HITLMode` is mostly schema, and its concrete tool approval path is provider-specific.
- `AutoGen` is strong on versioned saved state for agents, teams, and orchestrators, plus typed handoff and tool-execution events. It is weaker on durable approval semantics.
- `CrewAI` is strong on branchable checkpoint lineage and resumable snapshots, and has a concrete persisted pending-feedback path for flows. Its `human_input` model is still too coarse to use as Handshake governance authority.

## Extraction Matrix

| Harness | Stored Runtime Object | Resume / Restore Path | Approval / Human Stop Object | Code Path | Why It Matters For Handshake | Adoption Read |
| --- | --- | --- | --- | --- | --- | --- |
| PydanticAI | `GraphAgentState`, `GraphAgentDeps`, `DeferredToolRequests`, `DeferredToolResults` | same message history, same `tool_call_id`, explicit `resumed_request`, optional serialized `TemporalRunContext` | `ApprovalRequired`, `CallDeferred`, `ToolApproved`, `ToolDenied`, UI approval chunks | `pydantic_ai_slim/pydantic_ai/_agent_graph.py`, `.../tools.py`, `.../exceptions.py`, `.../agent/__init__.py`, `.../durable_exec/temporal/*` | strongest generic reference for durable approval boundaries and deferred side effects | adapt |
| Semantic Kernel | JSON-serializable agent state, typed process step state, process state metadata, Dapr process or step info | runtime `save/load` contract, step-state restore, state sanitization and migration by name or version | workflow `HITLMode` enum, plus provider-specific Azure AI `ToolApproval` objects for MCP calls | `dotnet/src/Agents/Runtime/Abstractions/IAgentRuntime.cs`, `python/semantic_kernel/agents/runtime/*`, `dotnet/src/Experimental/Process.*`, `python/samples/demos/process_with_dapr/*`, `python/semantic_kernel/agents/azure_ai/agent_thread_actions.py` | strong reference for state migration and durable process restore, weaker for generic approval semantics | adapt state, observe approval |
| AutoGen | versioned `BaseState` family for agents, teams, managers, and orchestrators | per-agent and per-team `save_state` or `load_state`, round-tripped through typed state models | typed `HandoffMessage`, `ToolCallExecutionEvent`, `UserInputRequestedEvent`, plus runtime intervention hooks | `python/packages/autogen-agentchat/src/autogen_agentchat/state/_states.py`, `.../messages.py`, `.../teams/_group_chat/*`, `.../agents/*`, `python/packages/autogen-core/src/autogen_core/_intervention.py` | useful for versioned team-state envelopes and handoff or event taxonomy | adapt state, observe approval |
| CrewAI | `RuntimeState` full snapshot of active entities, checkpoint lineage fields, event record, flow state, pending feedback context | `from_checkpoint`, `fork`, replay from stored task outputs, automatic checkpoint writes on event bus | task-level `human_input` flag, persisted `PendingFeedbackContext` for paused flow feedback | `lib/crewai/src/crewai/state/runtime.py`, `.../state/checkpoint_listener.py`, `.../flow/persistence/sqlite.py`, `.../crew.py`, `.../cli/checkpoint_*` | strongest second-wave reference for checkpoint lineage, branch recording, and resume tooling | adapt checkpointing, reject approval model |

## PydanticAI

### Concrete Mechanisms Seen

- `GraphAgentState` is the core per-run state object and carries `message_history`, `usage`, retry counters, `run_step`, `run_id`, metadata, and last request parameters.
- `GraphAgentDeps` carries `resumed_request` alongside prompt, tool manager, model settings, and output schema dependencies.
- deferred and approval-required tool calls are not modeled as plain text. They are represented as typed runtime objects:
  - `DeferredToolRequests`
  - `DeferredToolResults`
  - `ToolApproved`
  - `ToolDenied`
- approval-required or deferred tools are surfaced by raising `ApprovalRequired` or `CallDeferred`, both of which can carry metadata keyed by `tool_call_id`.
- `UserPromptNode` can resume a run by injecting `DeferredToolResults` back into the graph, reconstructing per-tool results, and skipping already-executed tool calls.
- approval-required tools are statically gated: the agent must allow `DeferredToolRequests` in its output schema or tool registration fails.
- UI surfaces also preserve approval as a typed stream event via `ToolApprovalRequestChunk`.
- the Temporal durable-exec adapter serializes approval-required and deferred outcomes into explicit discriminated result objects, and the serialized run context keeps `run_id`, `tool_call_approved`, `tool_call_metadata`, retry counters, and usage.

### Code Anchors

- `pydantic_ai_slim/pydantic_ai/exceptions.py`
- `pydantic_ai_slim/pydantic_ai/tools.py`
- `pydantic_ai_slim/pydantic_ai/_agent_graph.py`
- `pydantic_ai_slim/pydantic_ai/agent/__init__.py`
- `pydantic_ai_slim/pydantic_ai/toolsets/approval_required.py`
- `pydantic_ai_slim/pydantic_ai/ui/_adapter.py`
- `pydantic_ai_slim/pydantic_ai/ui/vercel_ai/response_types.py`
- `pydantic_ai_slim/pydantic_ai/durable_exec/temporal/_toolset.py`
- `pydantic_ai_slim/pydantic_ai/durable_exec/temporal/_run_context.py`

### Why This Matters

- this is the cleanest concrete answer to the Handshake question "what is the durable object for a governed stop"
- the object split is right:
  - request object for deferred or approval-needed calls
  - result object for approval, denial, or externally executed return
  - stable ids and metadata so the run can continue without transcript guessing
- Handshake should not import the library, but it should strongly consider copying this shape:
  - explicit deferred work request envelope
  - explicit approval result envelope
  - stable per-call ids
  - resume path that consumes prior unresolved calls as data

### Adoption Read

- `copy` the object split and lifecycle semantics
- `adapt` the state shape to Handshake workflow and capability boundaries
- do not copy the graph runtime as-is

## Semantic Kernel

### Concrete Mechanisms Seen

- the runtime contract is explicit: `IAgentRuntime` exposes send, publish, subscribe, save agent state, load agent state, and get agent metadata.
- the Python runtime mirrors this with `Agent.save_state`, `Agent.load_state`, runtime-wide `save_state`, and per-agent `agent_save_state` or `agent_load_state`.
- process and step state are explicit typed objects, and the Dapr process example makes the persistence boundary clear:
  - state in a step state model is persisted
  - ephemeral references such as a live agent or thread object are not persisted if they stay outside the state model
- `KernelProcessStateMetadataExtension` sanitizes and migrates saved step state by matching current step names or aliases and checking version compatibility.
- the serialized workflow model includes workflow variables, state update operations, and `HITLMode`, but that human-in-the-loop surface is mostly declarative.
- there is one concrete approval path in the Azure AI integration:
  - provider run surfaces `SubmitToolApprovalAction`
  - SK constructs `ToolApproval` objects
  - current code auto-approves MCP calls rather than running a generic kernel-level approval decision

### Code Anchors

- `dotnet/src/Agents/Runtime/Abstractions/IAgentRuntime.cs`
- `python/semantic_kernel/agents/runtime/core/agent.py`
- `python/semantic_kernel/agents/runtime/in_process/in_process_runtime.py`
- `dotnet/src/Experimental/Process.Abstractions/Serialization/Model/Workflow.cs`
- `dotnet/src/Experimental/Process.Core/Internal/KernelProcessStateMetadataExtension.cs`
- `python/samples/demos/process_with_dapr/process/steps.py`
- `python/samples/demos/process_with_dapr/process/process.py`
- `python/samples/demos/process_with_dapr/README.md`
- `python/semantic_kernel/agents/azure_ai/agent_thread_actions.py`

### Why This Matters

- Semantic Kernel is valuable for Handshake where state evolution and restore compatibility matter more than chat niceties.
- the strongest reusable ideas are:
  - save or load state as explicit runtime contract
  - separate persisted typed step state from ephemeral execution handles
  - sanitize and migrate saved state by current workflow shape and version
- the weakest part for Handshake is approval:
  - workflow `HITLMode` is not enough by itself
  - provider-specific approval objects are useful evidence, but they are not a product-wide approval runtime

### Adoption Read

- `adapt` state metadata, restore boundaries, and version migration patterns
- `observe` provider approval passthrough
- `reject` `HITLMode` enum alone as sufficient governance design

## AutoGen

### Concrete Mechanisms Seen

- `BaseState` gives every saved state object a `type` and `version`.
- agent, team, and orchestrator state are explicit typed models:
  - `AssistantAgentState`
  - `TeamState`
  - `RoundRobinManagerState`
  - `SwarmManagerState`
  - `MagenticOneOrchestratorState`
  - `SocietyOfMindAgentState`
- save or load is implemented in concrete agents and managers by serializing these state models and reconstructing message threads from dumped message objects.
- handoff and execution are typed runtime messages, not only informal conventions:
  - `HandoffMessage` carries `target` and `context`
  - `ToolCallExecutionEvent` carries structured tool execution results
  - `UserInputRequestedEvent` represents a user-input stop as an explicit event
- intervention is also explicit, but weaker as a durable control-plane primitive:
  - `InterventionHandler` can modify, log, or drop messages
  - only the single-threaded runtime supports it
- tests confirm save/load round-trip for society-of-mind agents and task-runner tools.

### Code Anchors

- `python/packages/autogen-agentchat/src/autogen_agentchat/state/_states.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/messages.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/base/_handoff.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/agents/_assistant_agent.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/agents/_society_of_mind_agent.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/teams/_group_chat/_round_robin_group_chat.py`
- `python/packages/autogen-agentchat/src/autogen_agentchat/teams/_group_chat/_magentic_one/_magentic_one_orchestrator.py`
- `python/packages/autogen-core/src/autogen_core/_intervention.py`
- `python/packages/autogen-agentchat/tests/test_task_runner_tool.py`
- `python/packages/autogen-agentchat/tests/test_society_of_mind_agent.py`

### Why This Matters

- AutoGen provides a clean pattern for versioned saved state across increasingly complex team or orchestrator types.
- its message taxonomy is also useful because handoff, tool execution, and user-input stops are first-class typed events.
- the missing piece is durable approval semantics:
  - handoff is strong
  - state is strong
  - intervention and user-input events are real, but they are not the same as a durable approval request or result object with restart-safe matching ids

### Adoption Read

- `adapt` versioned state envelopes and typed event taxonomy
- `observe` intervention hooks as a policy surface
- `reject` AutoGen as the primary model for durable approval objects

## CrewAI

### Concrete Mechanisms Seen

- `RuntimeState` is a full snapshot root model over active entities and serializes:
  - `crewai_version`
  - checkpoint lineage parent id
  - branch label
  - serialized entities
  - event record
- checkpoints are not just raw snapshots. They also maintain lineage with `_checkpoint_id`, `_parent_id`, and `branch`, and support explicit fork creation.
- checkpoint writes can be triggered automatically from the event bus via `checkpoint_listener.py`.
- `Crew.from_checkpoint` restores a crew into the runtime state on the event bus and calls `_restore_runtime`.
- `Crew.fork` restores first, then branches the underlying runtime state.
- replay is supported separately from checkpoint restore by reading stored task outputs and resuming from a specific task.
- the flow SQLite persistence layer is more interesting than the top-level `human_input` flag:
  - it stores flow state
  - it stores `PendingFeedbackContext`
  - it can later reload both state and feedback context for resume after a paused human-feedback wait
- the checkpoint CLI and TUI are strong operator surfaces for inspecting, pruning, resuming, and forking checkpoints.
- by contrast, `human_input` in task or config surfaces is still just a coarse flag, not a typed approval request or result object.

### Code Anchors

- `lib/crewai/src/crewai/state/runtime.py`
- `lib/crewai/src/crewai/state/checkpoint_listener.py`
- `lib/crewai/src/crewai/flow/persistence/sqlite.py`
- `lib/crewai/src/crewai/crew.py`
- `lib/crewai/src/crewai/cli/checkpoint_cli.py`
- `lib/crewai/src/crewai/cli/checkpoint_tui.py`
- `lib/crewai/src/crewai/project/crew_base.py`

### Why This Matters

- CrewAI is a strong reference for checkpoint lineage and branchable resume, especially for operator-visible recovery tooling.
- the pending-feedback persistence is the part worth stealing for Handshake, not the coarse human-input flag.
- Handshake should treat this as:
  - good for checkpoint lineage and operator restore surfaces
  - good for pause/resume where feedback context must be rehydrated
  - weak for authority semantics and approval identity

### Adoption Read

- `adapt` checkpoint lineage, branch recording, and persisted feedback-context resume
- `observe` replay tooling and operator-facing checkpoint UX
- `reject` task-level `human_input` as a Handshake governance primitive

## Cross-Framework Read

### Strongest Answers By Question

#### What is the strongest direct model for approval-as-data

- `PydanticAI`

Reason:
- explicit request and result envelopes
- explicit approve or deny results
- explicit per-call metadata and ids
- explicit resume path that consumes those results

#### What is the strongest direct model for versioned runtime state

- `AutoGen` for compact typed state models
- `Semantic Kernel` for state migration and restore discipline
- `CrewAI` for full snapshot lineage and branching

#### What is the strongest direct model for restart-safe branchable execution

- `CrewAI` from this batch

Reason:
- explicit checkpoint ids
- explicit parent lineage
- explicit branch labels
- explicit `from_checkpoint` and `fork`

#### What is the strongest direct model for paused human feedback resume

- `CrewAI` flow persistence for saved feedback context
- `PydanticAI` for approval-result matching by `tool_call_id`

## Product Translation Constraints

`HANDSHAKE_PRODUCT_REFERENCE.md` is useful as a map, but it is reference-only.
The actual authority for translation is the Master Spec plus the product-governance boundary work already captured in the product snapshot.

The runtime findings above do not translate 1:1 into Handshake for these reasons:

- runtime governance state is product-owned and defaults to `.handshake/gov/`; product runtime must not treat repo `/.GOV/` or `docs/**` as authoritative runtime state
- the authoritative execution units are product primitives such as `WorkflowRun`, `WorkflowNodeExecution`, and `ModelSession`, not a framework's native run, thread, team, or graph object
- Dev Command Center is the canonical control-plane projection for workflow runs, approval routing, capability posture, and session recovery, but projection does not equal authority
- the capability system remains authoritative for policy and denial semantics, so approval or deferred-action objects must resolve through governed capability checks rather than framework-local callbacks alone
- Flight Recorder is a required evidence plane, so approval, deferred execution, resume, retry, fork, and export transitions need typed recorder visibility instead of transcript-only reconstruction
- Task Board, Role Mailbox, packet mirrors, and readable Markdown artifacts are derived planning or collaboration surfaces and must not become a second execution authority

## Translation Layer For Runtime Research

The strongest framework mechanisms should be translated into product-owned Handshake surfaces like this:

- `PydanticAI` deferred-request and approval-result objects map best to governed action envelopes attached to workflow node execution and gated tool or engine calls
- `Semantic Kernel` save or load contracts and state migration patterns map to product runtime-state restore across workflow definitions, session state, and checkpoint compatibility
- `AutoGen` versioned state envelopes and typed event vocabulary map to internal state families and recorder or projection taxonomy, not to product authority by themselves
- `CrewAI` checkpoint lineage, fork semantics, and paused-feedback persistence map to workflow checkpoint lineage plus operator-visible restore or fork tooling in Dev Command Center

## Handshake Implications

Handshake governance should not choose one of these frameworks as the model.
The higher-value move is to synthesize a narrower product-owned runtime beneath Handshake's workflow, capability, projection, and recorder surfaces with these borrowed pieces:

- from `PydanticAI`:
  - deferred action request object
  - approval result object
  - stable action ids
  - explicit deny or approve semantics
- from `Semantic Kernel`:
  - save or load contract
  - typed step state boundary
  - state migration and sanitization
- from `AutoGen`:
  - versioned agent or team state envelopes
  - typed handoff and execution event vocabulary
- from `CrewAI`:
  - checkpoint lineage
  - branch or fork semantics
  - persisted feedback context for paused workflows
  - operator-visible restore tooling

## Current Recommendation

If the next document is a Handshake target architecture, the runtime section should be built around these design choices:

1. one product-owned runtime state family centered on `WorkflowRun`, `WorkflowNodeExecution`, and `ModelSession`, with versioning and lineage
2. one explicit governed action envelope for side effects that cross capability or approval boundaries
3. one explicit action result envelope for approve, deny, external execute, retry, or skip
4. one migration boundary between stored runtime state and the current workflow definition, without transcript-only recovery
5. one paused-feedback resume carrier that can be projected in Dev Command Center and recorded in Flight Recorder, not a boolean `human_input` flag
6. one strict mirror rule: Task Board, Role Mailbox, and readable governance files stay derived surfaces, not execution authority

## Remaining Open Questions

- whether `PydanticAI` has any hidden coupling between deferred tool output handling and graph-native execution assumptions that would make direct shape-copy awkward
- whether `Semantic Kernel` has a fuller generic approval engine beyond workflow schema and Azure AI provider passthrough
- whether `AutoGen` has a stronger restart-safe approval or intervention path outside the single-threaded runtime
- whether `CrewAI` pending-feedback persistence is safe enough under concurrent resume or multi-writer conditions to use as a direct design reference
