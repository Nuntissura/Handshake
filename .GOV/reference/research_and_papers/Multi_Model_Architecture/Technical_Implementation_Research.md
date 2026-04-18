# Technical Implementation Research — Multi-Agent Harness Mechanics

## Purpose

Implementation-grade technical research for building the Handshake agent harness.
This is NOT a survey — the survey lives in `Multi Agent Architectures.md`.
This document answers **HOW** systems work at the code/protocol/runtime level.

Every section must produce actionable technical detail: data structures, state machines,
wire formats, failure modes, recovery mechanics. Marketing descriptions are worthless here.

### Research method

For each system:
1. Read source code (GitHub repos, not docs pages)
2. Read architecture posts / RFCs / design docs from the actual builders
3. Trace the critical path: what happens when an agent is spawned, steered, fails, recovers?
4. Identify the state model: where does state live, how is it persisted, what happens on crash?
5. Identify the contract model: how are agents constrained before they start?
6. Identify the failure model: what breaks, what's detected, what's silent?

### Relationship to other documents

| Document | Role |
|---|---|
| `Multi Agent Architectures.md` | Survey / catalog — breadth index of 60+ systems |
| `Technical_Implementation_Research.md` (this file) | Implementation depth — HOW the best systems actually work |
| `Harness_Lessons_Learned.md` | Our own ACP/governance experience — what broke and why |
| Architecture Synthesis (future) | Combined output — architecture decisions with evidence |

---

## Pain Point Map

These are the specific problems the Handshake harness is fighting right now.
Every Tier 1 deep-dive must address how the system handles (or fails to handle) each of these.

| ID | Pain point | What we need to learn |
|---|---|---|
| PP-1 | Hard loop caps + strategy escalation | When does the system stop retrying and change strategy? What triggers escalation vs abort? Is this configurable per-task? |
| PP-2 | Machine-readable MT contracts | How are agent outputs constrained before execution? Typed schemas? Validation gates? Pre-declared scope? |
| PP-3 | Heuristic-risk classification | Does the system classify tasks by complexity/risk before routing? Different treatment for deterministic vs fuzzy work? |
| PP-4 | Operator alerting outside chat | How does the operator learn about problems without watching a terminal? Push notifications? Webhooks? Dashboard polling? |
| PP-5 | Broker/session reliability under load | How does the system manage session lifecycle under host pressure? Crash recovery? Backpressure? Resource contention? |
| PP-6 | Mechanical state transitions | How does orchestration work? Explicit state machine? Graph execution? Or narrative prompt chains? |

---

## TIER 1 — Deep Technical Analysis

### 1.1 Cursor

**Why:** Closest to our isolation model. Worktrees, shadow workspace, background agents, model routing, hooks, approval surfaces.

**Source material:**
- [ ] GitHub source (where available) / extension internals
- [ ] Cursor changelog and architecture blog posts
- [ ] Steve Yegge's analysis of Cursor internals
- [ ] Community reverse-engineering (if official source is closed)

#### 1.1.1 Agent Lifecycle

- How are background agents spawned? What parameters constrain them at launch?
- What is the agent's execution boundary? (worktree? container? sandbox? process?)
- How does the system track agent state across restarts / crashes?
- What is the maximum concurrent agent count and how is it enforced?

#### 1.1.2 State Management

- Shadow workspace implementation: how is file state isolated per agent?
- Worktree management: created per-agent? per-task? pooled?
- Checkpoint mechanics: when is state persisted? what triggers a checkpoint?
- Recovery after crash: does the agent resume or restart? from what state?

#### 1.1.3 Orchestration & Routing

- Model routing: how does Cursor decide which model handles which subtask?
- Subagent delegation: what triggers spawning a subagent vs handling inline?
- Loop control: is there a retry budget? escalation path? hard abort?
- How does the orchestrator transition between phases? (explicit states vs prompt-driven?)

#### 1.1.4 Hooks & Approval Surfaces

- Hook system architecture: what events trigger hooks? synchronous or async?
- Deterministic middleware: what runs without model involvement?
- Approval gating: what actions require user approval? how is the gate implemented?
- How do hooks compose with agent autonomy settings?

#### 1.1.5 Communication & Coordination

- How do background agents report results back?
- Is there a message bus or is it direct function calls?
- Artifact handoff: how do agents share file changes, diffs, results?
- How does the UI stay in sync with background agent state?

#### 1.1.6 Pain Point Coverage

| Pain Point | How Cursor handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.1.7 Findings

_To be filled after research._

---

### 1.2 Claude Code

**Why:** We literally run on this. Hooks, subagents, checkpoints, permissions, sequential steering. Understanding its actual mechanics is non-negotiable.

**Source material:**
- [ ] Claude Code documentation and changelog
- [ ] Hook system specification
- [ ] Agent/subagent implementation details
- [ ] Permission model documentation
- [ ] MCP integration architecture

#### 1.2.1 Agent Lifecycle

- How are subagents spawned? What isolation do they get? (worktree mode, process, context)
- Agent tool: what parameters control the subagent? (prompt, model, isolation, background)
- How does the parent agent track subagent state? (blocking vs background, notification)
- What happens when a subagent fails or times out?

#### 1.2.2 State Management

- Conversation context: how is it managed as it approaches limits? (compaction)
- Worktree isolation: how does `isolation: "worktree"` actually work? (git worktree lifecycle)
- Checkpoint/memory: file-based memory system — how does it persist across conversations?
- Crash recovery: what state survives a session crash? what's lost?

#### 1.2.3 Orchestration & Routing

- Sequential execution model: no parallel in-lane work — how is this enforced?
- Model selection: how does fast mode / model override work in the agent hierarchy?
- Skill system: how do skills expand into prompts? composability?
- How does Claude Code handle tool call chains with dependencies?

#### 1.2.4 Hooks & Permission Surfaces

- Hook trigger points: what events fire hooks? (PreToolUse, PostToolUse, etc.)
- Hook execution: synchronous blocking? what happens on hook failure?
- Permission modes: how do allowlist/denylist/auto-approve compose?
- How do hooks interact with the approval prompt?

#### 1.2.5 Communication & Coordination

- Parent ↔ subagent communication: prompt in, single message result out — no streaming
- Tool results and user messages: how are system reminders injected?
- MCP integration: how do MCP servers expose tools into the agent context?
- How does the system handle context window pressure across agent hierarchies?

#### 1.2.6 Pain Point Coverage

| Pain Point | How Claude Code handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.2.7 Findings

_To be filled after research._

---

### 1.3 OpenHands

**Why:** Strong event stream architecture, sandboxed execution, runtime agent control. Most transparent implementation of session lifecycle.

**Source material:**
- [ ] GitHub: All-Hands-AI/OpenHands — full source available
- [ ] Architecture documentation and design decisions
- [ ] Event stream implementation
- [ ] Sandbox/runtime implementation

#### 1.3.1 Agent Lifecycle

- Agent runtime loop: what is the actual event loop? (observation → action cycle)
- How are agents initialized? what state do they start with?
- Agent delegation: how does one agent spawn/delegate to another?
- Timeout and termination: what kills an agent? configurable budgets?

#### 1.3.2 State Management

- Event stream: what events are recorded? what is the persistence format?
- State restoration: can an agent session be resumed from the event stream?
- Sandbox state: how is the execution environment state managed? (Docker container lifecycle)
- File state isolation: how are workspace changes tracked and committed?

#### 1.3.3 Orchestration & Routing

- Controller architecture: how does the controller decide what the agent does next?
- Micro-agent concept: specialized agents for specific tasks — how are they routed to?
- Stuck detection: how does the system detect an agent is looping or stuck?
- LLM retry logic: what happens on model API failure? backoff? fallback?

#### 1.3.4 Sandbox & Execution Isolation

- Docker sandbox: what is the container lifecycle per agent session?
- File system isolation: how are workspace files mounted and synced?
- Command execution: how are shell commands run? what limits are enforced?
- Security boundary: what can the agent NOT do? how is this enforced?

#### 1.3.5 Communication & Coordination

- Event stream as communication substrate: how do components communicate?
- Agent ↔ sandbox communication: what is the protocol?
- UI ↔ backend communication: WebSocket? SSE? polling?
- How does observation/action serialization work?

#### 1.3.6 Pain Point Coverage

| Pain Point | How OpenHands handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.3.7 Findings

_To be filled after research._

---

### 1.4 LangGraph

**Why:** Strongest implementation of mechanical state transitions. Durable state machines, checkpointing, interrupts, typed channels. Directly addresses PP-6.

**Source material:**
- [ ] GitHub: langchain-ai/langgraph — full source available
- [ ] LangGraph documentation (concepts, how-to, architecture)
- [ ] Checkpoint/persistence implementation
- [ ] LangGraph Platform / Cloud architecture docs

#### 1.4.1 Agent Lifecycle

- Graph compilation: how does a graph definition become an executable runtime?
- Node execution: how is a single node invoked? what is the execution contract?
- Subgraph composition: how do nested graphs work? state scoping?
- Thread lifecycle: what is a thread? how is it created, persisted, resumed?

#### 1.4.2 State Management

- State schema: how is graph state defined? (TypedDict, Pydantic, dataclass)
- Reducer functions: how do concurrent node outputs merge into shared state?
- Checkpointing: what triggers a checkpoint? what is persisted? (full state snapshot? delta?)
- Checkpoint backends: SQLite, PostgreSQL, custom — what are the trade-offs?
- Time-travel: how does replaying from a checkpoint actually work?

#### 1.4.3 Orchestration & Routing

- Conditional edges: how does the router function work? (state → next node mapping)
- Send API: dynamic fan-out — how does map-reduce work within a graph?
- Human-in-the-loop interrupts: `interrupt_before`, `interrupt_after` — implementation mechanics
- Retry policies: per-node retry configuration — what happens on node failure?
- Subgraph invocation: how does a parent graph call a child graph? state bridging?

#### 1.4.4 Typed Channels & Contracts

- Channel types: what channel types exist? (LastValue, Append, BinaryOperator)
- Input/output schemas: how are they enforced? compile-time? runtime?
- Validation: what happens when a node produces invalid output?
- How do channels compose with conditional routing?

#### 1.4.5 Communication & Coordination

- Message passing between nodes: via shared state channels, not direct
- Streaming: how does the runtime stream intermediate state to clients?
- Event system: what events does the runtime emit? (on_chain_start, on_chain_end, etc.)
- Multi-agent patterns: supervisor, handoff, hierarchical — how are they implemented on top of the graph primitive?

#### 1.4.6 Pain Point Coverage

| Pain Point | How LangGraph handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.4.7 Findings

_To be filled after research._

---

### 1.5 Gas Town / Beads (Steve Yegge)

**Why:** Closest to the "operator as colony manager" vision. Durable work graph, watchdogs, mailboxes. Fundamentally different mental model from IDE-native agents.

**Source material:**
- [ ] Steve Yegge blog posts and public talks
- [ ] Beads source code / documentation (if available)
- [ ] Gas Town architecture descriptions
- [ ] Community analysis and comparisons

#### 1.5.1 Agent Lifecycle

- Worker spawning: how does the mayor/manager create workers?
- Worker scope: what constrains a worker? task definition? resource limits?
- Worker monitoring: watchdog mechanics — what triggers intervention?
- Worker termination: graceful shutdown vs kill vs abandon?

#### 1.5.2 State Management

- Durable work graph: what is the data model? nodes and edges?
- Persistence: where does the work graph live? how is it persisted?
- Mailboxes: implementation — queue? database? file? how are messages durable?
- Crash recovery: what happens when the colony manager crashes? when a worker crashes?

#### 1.5.3 Orchestration & Routing

- Colony management model: how does the mayor decide what workers do?
- Task decomposition: who breaks work into subtasks? how?
- Coordination patterns: what patterns does the system use? (star? mesh? pipeline?)
- Escalation: how does a worker signal it's stuck? what happens next?

#### 1.5.4 Operator Console

- What does the operator see? real-time work graph? historical logs?
- What can the operator do? (kill workers, reassign tasks, modify the graph?)
- How does the console stay in sync with the runtime?
- Approval/intervention surfaces: how does the operator inject decisions?

#### 1.5.5 Communication & Coordination

- Mailbox protocol: what is the message format? typed? schema-validated?
- Hook system: what hooks exist? synchronous or async?
- Inter-worker communication: do workers talk to each other or only through the manager?
- Artifact handoff: how do workers share results?

#### 1.5.6 Pain Point Coverage

| Pain Point | How Gas Town/Beads handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.5.7 Findings

_To be filled after research._

---

### 1.6 PydanticAI

**Why:** Strongest typed output / validation-first design. Directly addresses PP-2 (machine-readable MT contracts).

**Source material:**
- [ ] GitHub: pydantic/pydantic-ai — full source available
- [ ] Documentation: agent definition, structured outputs, tools, dependencies
- [ ] Validation and retry mechanics

#### 1.6.1 Agent Lifecycle

- Agent definition: how is an agent defined? (system prompt, tools, result type, model)
- Agent run: what happens during `agent.run()`? the actual execution loop
- Run context: what state is available during execution? (dependencies, retry info)
- Result validation: what happens when the model output fails validation?

#### 1.6.2 Typed Output Contracts

- Result type declaration: how is the output schema defined? (Pydantic model, union types)
- Structured output enforcement: how is the model forced to produce valid output?
- Validation retries: how many retries? what feedback does the model get on failure?
- Union result types: how does the system handle multiple valid output shapes?
- Custom validators: how do application-level validators compose with schema validation?

#### 1.6.3 Tool System

- Tool declaration: function-based tools — how are parameter schemas derived?
- Tool execution context: what does a tool have access to? (RunContext, dependencies)
- Tool retry: what happens when a tool call fails?
- Tool result injection: how do tool results flow back into the conversation?

#### 1.6.4 Dependency Injection

- Deps pattern: how does the dependency injection system work?
- What can be injected? (database connections, API clients, configuration)
- How does this compose with testing? mocking?

#### 1.6.5 Pain Point Coverage

| Pain Point | How PydanticAI handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.6.6 Findings

_To be filled after research._

---

### 1.7 Microsoft Agent Framework (AutoGen successor)

**Why:** Strongest on observability, checkpointing, and time-travel debugging. Enterprise-grade telemetry. Addresses PP-4 and PP-5.

**Source material:**
- [ ] GitHub: microsoft/autogen (v0.4+ / Agent Framework) — full source available
- [ ] Architecture documentation and migration guides
- [ ] Checkpoint and state management implementation
- [ ] Telemetry / observability implementation

#### 1.7.1 Agent Lifecycle

- Agent types: what agent abstractions exist? (AssistantAgent, CodeExecutorAgent, custom)
- Agent runtime: how does the runtime manage agent instances?
- Team/group patterns: how are multi-agent teams composed?
- Termination conditions: what conditions stop execution? composable?

#### 1.7.2 State Management

- Checkpointing: what is checkpointed? agent state? conversation? tool state?
- Checkpoint storage: backends? format? frequency?
- Time-travel: how does replaying from a checkpoint work? what state is restored?
- State serialization: how are agent states serialized for persistence?

#### 1.7.3 Orchestration & Routing

- Team topologies: what built-in patterns exist? (round-robin, selector, swarm, etc.)
- Selector pattern: how does the selector decide which agent speaks next?
- Handoff mechanics: how does one agent transfer control to another?
- Nested teams: how do teams compose? state scoping between teams?

#### 1.7.4 Telemetry & Observability

- OpenTelemetry integration: what spans are created? what attributes?
- Event system: what events does the runtime emit?
- Token tracking: how is token usage tracked per agent/per-turn?
- Trace correlation: how are distributed traces correlated across agents?
- Integration with LangSmith / custom backends?

#### 1.7.5 Communication & Coordination

- Message types: what message types exist? structured?
- Routing: how are messages routed between agents in a team?
- Broadcast vs directed: can agents send to specific targets or only broadcast?
- Human-in-the-loop: how does the framework handle human intervention?

#### 1.7.6 Pain Point Coverage

| Pain Point | How MS Agent Framework handles it | Evidence / Source |
|---|---|---|
| PP-1 Loop caps | | |
| PP-2 Contracts | | |
| PP-3 Risk classification | | |
| PP-4 Operator alerting | | |
| PP-5 Session reliability | | |
| PP-6 Mechanical transitions | | |

#### 1.7.7 Findings

_To be filled after research._

---

## TIER 2 — Pattern Extraction

For each system below, extract the ONE specific mechanic it does best.
Don't do a full deep-dive — just the reusable pattern.

### 2.1 CrewAI — Role Delegation & Process Types

**Research question:** How does CrewAI define role constraints and select between sequential, hierarchical, and consensual process types?

- Role definition: what fields constrain an agent? (role, goal, backstory, tools, allow_delegation)
- Process types: sequential vs hierarchical vs consensual — how does each work internally?
- Task definition: expected_output, context, human_input — how are these enforced?
- Delegation: when an agent delegates, what happens mechanically?
- Memory: what memory types exist? how persistent?

**Pattern to extract:** Role-constrained task delegation with process type selection.

---

### 2.2 smolagents — Minimum Viable Agent Scaffold

**Research question:** What is the absolute minimum scaffold needed for a functional agent? Where is the boundary between "too little" and "enough"?

- Core loop: how small is the actual agent loop? (plan → act → observe)
- Code-first actions: how do code actions differ from tool-call actions?
- Sandboxing: how is code execution sandboxed? (E2B, Docker, local)
- Multi-agent: how does the managed agent pattern work?
- What is intentionally NOT included?

**Pattern to extract:** Lower bound of agent scaffold complexity.

---

### 2.3 A2A Protocol (Google) — Agent-to-Agent Wire Format

**Research question:** What does a standardized inter-agent communication protocol look like at the wire level?

- Agent Card: discovery mechanism — what's in the card?
- Task lifecycle: what states does a task go through? (submitted, working, input-required, completed, failed)
- Message format: structured parts — text, file, data?
- Streaming: SSE-based streaming — what events?
- Push notifications: webhook-based updates — what triggers them?
- Authentication: how is trust established?

**Pattern to extract:** Standardized agent communication protocol and task lifecycle.

---

### 2.4 OpenAI Swarm — Handoff Mechanics

**Research question:** How does the simplest possible agent handoff work? What makes it work despite being minimal?

- Handoff function: how is an agent switch implemented? (return Agent object)
- Context variables: how is state carried across handoffs?
- What state is preserved vs lost on handoff?
- Why is this explicitly "educational" and not production? What's missing?

**Pattern to extract:** Minimal handoff mechanic and what production requires beyond it.

---

### 2.5 SWE-agent — Trajectory Debugging

**Research question:** How does SWE-agent record and replay agent trajectories for debugging?

- Trajectory format: what is recorded per step?
- ACI (Agent-Computer Interface): how is the interface constrained?
- Replay: can a trajectory be replayed? how?
- Config system: how are agent behaviors parameterized?

**Pattern to extract:** Trajectory-first debugging and constrained agent interface.

---

### 2.6 CrewAI / LangGraph — Memory Implementation Patterns

**Research question:** How do production frameworks implement agent memory? What types, what persistence, what lifecycle?

- Memory types: short-term, long-term, entity, procedural — how does each work?
- Persistence: in-memory vs database vs file?
- Memory lifecycle: creation, retrieval, expiry, compaction?
- Cross-session memory: how does memory persist between runs?
- Memory and context window: how is memory injected without overflowing context?

**Pattern to extract:** Memory type taxonomy and lifecycle management.

---

### 2.7 Codex (OpenAI) — Command Center / Cloud Execution

**Research question:** How does Codex handle remote agent execution with operator oversight?

- Cloud sandbox: what is the execution environment?
- Task submission: how does the operator define work?
- Progress visibility: how does the operator see what's happening?
- Result review: how are results presented for approval?
- Multi-task management: how are concurrent tasks tracked?

**Pattern to extract:** Remote execution with asynchronous operator oversight.

---

### 2.8 Cline — Permission Surface Design

**Research question:** How does Cline implement granular human-in-the-loop approval?

- Permission categories: what actions require approval?
- Auto-approve settings: how granular is the configuration?
- Approval UX: how is the approval presented to the user?
- How do permissions compose with agent autonomy levels?

**Pattern to extract:** Granular permission gating with progressive autonomy.

---

### 2.9 AgentScope — Message Hub & Runtime

**Research question:** How does AgentScope implement its message hub pattern for multi-agent coordination?

- Message hub: how does the centralized message routing work?
- Agent runtime: distributed deployment capabilities?
- MCP + A2A integration: how is interoperability implemented?
- Monitoring: built-in observability features?

**Pattern to extract:** Centralized message hub with monitoring.

---

### 2.10 BeeAI Framework — Policy Surface

**Research question:** How does BeeAI implement agent policy/governance?

- Policy system: what policies can be defined?
- Enforcement: how are policies enforced at runtime?
- A2A/MCP serving: how does the framework expose agents as services?

**Pattern to extract:** Policy-first agent governance.

---

## TIER 2 — Cross-Cutting Pattern Synthesis

After individual pattern extraction, synthesize across systems:

### 2.A State Machine Patterns Comparison

Compare how each system implements state transitions:
- Explicit graph (LangGraph, MS Agent Framework) vs implicit prompt-chain (most others)
- What triggers transitions? (function return? state change? router decision?)
- How is the "current state" represented and persisted?

### 2.B Failure Handling Taxonomy

Compile how systems handle different failure types:
- Model API failure (timeout, rate limit, error)
- Agent loop (stuck, repeating, diverging)
- Output validation failure
- Sandbox/execution failure
- State corruption

### 2.C Contract Enforcement Spectrum

Map the spectrum from "no constraints" to "fully typed":
- No constraints: raw prompt, any output (most basic agents)
- Soft constraints: system prompt says "output JSON" (unreliable)
- Schema constraints: structured output mode (model-level enforcement)
- Typed + validated: Pydantic model + retry on validation failure
- Contract + pre-conditions: task definition with expected output + scope limits

### 2.D Session Lifecycle Patterns

Compare session management across systems:
- Spawn: how is a session created? what parameters?
- Monitor: how is progress tracked?
- Steer: how does the operator intervene?
- Recover: what happens on crash?
- Close: how is a session terminated and cleaned up?

---

## Research Execution Tracker

| System | Status | Researcher | Date started | Date completed | Key findings |
|---|---|---|---|---|---|
| 1.1 Cursor | NOT STARTED | | | | |
| 1.2 Claude Code | NOT STARTED | | | | |
| 1.3 OpenHands | NOT STARTED | | | | |
| 1.4 LangGraph | NOT STARTED | | | | |
| 1.5 Gas Town / Beads | NOT STARTED | | | | |
| 1.6 PydanticAI | NOT STARTED | | | | |
| 1.7 MS Agent Framework | NOT STARTED | | | | |
| 2.1 CrewAI | NOT STARTED | | | | |
| 2.2 smolagents | NOT STARTED | | | | |
| 2.3 A2A Protocol | NOT STARTED | | | | |
| 2.4 OpenAI Swarm | NOT STARTED | | | | |
| 2.5 SWE-agent | NOT STARTED | | | | |
| 2.6 Memory patterns | NOT STARTED | | | | |
| 2.7 Codex | NOT STARTED | | | | |
| 2.8 Cline | NOT STARTED | | | | |
| 2.9 AgentScope | NOT STARTED | | | | |
| 2.10 BeeAI | NOT STARTED | | | | |
| 2.A State machines | NOT STARTED | | | | |
| 2.B Failure handling | NOT STARTED | | | | |
| 2.C Contracts | NOT STARTED | | | | |
| 2.D Session lifecycle | NOT STARTED | | | | |
