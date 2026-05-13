# Handshake V2 Kernel Reset Brief

STATUS: OPERATOR_LOCAL_DRAFT
CREATED: 2026-05-09
OWNER: OPERATOR
PURPOSE: Capture the Operator's strategic reset intent and preliminary research proposals before deciding whether to rebuild Handshake around a product-native deterministic kernel.
AUTHORITY: This file is an operator-local planning brief. It is not a Work Packet, Master Spec amendment, validator verdict, or implementation order until explicitly promoted.

## 0. No-Context Handoff

This brief must be readable by a capable model with no prior chat context. The expected reader is a future cloud model, local model, coder, validator, or planning agent that needs to understand why the Operator is considering a Handshake rebuild and what the first technical implementation direction should be.

Compressed history:

- The Operator first created The Prompt Diaries, a large prompt/governance system with about 1200 rule bundles across 14 RIDs. It proved that prose-only LLM control was too brittle.
- Handshake was created from the insight that LLMs should be governed by code, typed state, deterministic tools, and validation gates instead of huge natural-language rule prompts.
- The first Handshake build produced useful spine: strict validation, microtasks, role separation, machine-readable artifacts, receipts, and tool-mediated execution.
- The failure was build order. The external repo-governance harness became a mirror of the future product and started consuming the project. Governance drift, documentation sync, role paperwork, and repair loops burned too much model budget.
- The Operator now wants to pause the current build direction and consider restarting around a product-native Handshake kernel. Handshake itself should become the governance/runtime surface, while repo governance shrinks to a protective shell.

Core decision under consideration:

- Do not throw away the vision or Master Spec.
- Do change build order.
- Build the deterministic product kernel first.
- Add creative modules after the kernel can govern work, models, memory, validation, artifacts, and operator control.

Terms used in this brief:

- `Operator`: the human owner controlling Handshake.
- `Operator surface`: the app UI where the Operator controls sessions, models, tasks, artifacts, validation, and logs.
- `Session broker`: the product service that routes model sessions, tool calls, sandbox work, receipts, and state updates.
- `Model lane`: one execution path for a model, for example local inference, official CLI bridge, BYOK API, or manual handoff.
- `Authority state`: durable truth stored in Postgres/event records.
- `CRDT state`: live collaborative workspace state for parallel operator/model editing.
- `Projection`: generated UI, Markdown, reports, or summaries derived from authority state.
- `Promotion gate`: the deterministic step that converts sandbox output or CRDT edits into authoritative product state.
- `Visible reasoning`: planning text, thinking summaries, CLI-visible assessment text, or provider-exposed reasoning output. It is diagnostic evidence only, not complete internal model computation and not authority.

If a future model reads only this file, it should understand that the first implementation target is not "make an AI IDE." The first target is a local-first deterministic runtime where models, tools, memory, sandboxes, CRDT collaboration, logs, tests, and operator decisions are all converted into typed state that Handshake can inspect, replay, validate, and improve.

## 1. Plain Decision

The current Handshake build should pause. The project may restart around a smaller product-native kernel instead of continuing to expand the external repo-governance harness.

The existing Master Spec remains valuable as a vision map and technology reference, but it should no longer control build order as one giant atomic execution file. The build order should start with the product core: deterministic runtime, local models, machine-readable state, sandboxed execution, memory, diagnostics, and operator-facing control surfaces.

## 2. Operator Intent

Handshake was born from the pain of The Prompt Diaries: a large rule-governed creative system where LLMs were boxed in by prompts and natural-language rules. That approach was too large and brittle for models in 2025. The core realization was that LLM behavior should not be controlled mainly by prose. It should be controlled by code, typed state, deterministic tools, and mechanical gates.

The first Handshake effort correctly pushed for rigor:

- no corner-cutting
- strict validation
- microtasks
- deterministic checks
- machine-readable artifacts
- LLMs acting through tools instead of directly mutating truth
- raw/derived/display separation
- a strong spine for creative workflows

The mistake was that the repo-governance harness became a mirror of the product, then consumed too much work itself. Governance documentation drift, packet paperwork, role protocols, repair loops, and sync surfaces cost too much token budget and slowed product progress. Coding was not the main problem; the external governance system became the pain point.

The new intent is to stop iterating on governance outside the product and instead build Handshake as the governance runtime from day one.

## 3. New Product Thesis

Handshake should be a deterministic local-first harness for creative and software work. It should coordinate operator actions, cloud models, local models, tools, memory, validation, and creative modules through machine-readable state.

The product should be able to help build itself because the product kernel owns the same primitives currently simulated by repo governance:

- actors
- sessions
- work packets
- microtasks
- typed receipts
- runtime state
- validation gates
- artifact promotion
- memory
- diagnostic traces
- operator-facing projections

The long-term edge is not just "an AI wrapper." The edge is a repeatable mechanical runtime where LLMs do not have to remember the workflow. The machine holds the workflow.

## 4. Day-One Rules

- Everything important is machine-readable.
- Every durable entity has a stable unique ID.
- Markdown is a projection, not authority, when a typed contract exists.
- The operator-facing UI is the primary control surface, not VS Code terminals.
- The operator surface is the audit boundary for model work.
- The operator surface must behave as a control room: task queue, live session trace, model output, tool calls, diffs, tests, approvals, memory, artifacts, and validation in one place.
- Local models are native from day one.
- Cloud models are optional execution engines, not the kernel.
- Cursor/Copilot/Antigravity-style bundled cloud model subscriptions are not a day-one Handshake path.
- Unofficial consumer-web automation is not a Handshake kernel dependency.
- Official CLIs may be used as backend transports when allowed, but Handshake must still own the operator surface, state, logs, and promotion gates.
- CRDT is day-one for live parallel operator/model workspace state.
- Postgres/event state is day-one for authoritative truth.
- SQLite is not a Kernel V1 technology. Do not use SQLite for authority, cache, offline mode, compatibility mode, tests, local fallback, or bootstrap convenience going forward.
- Sandboxed execution is day-one for model-written code.
- Deterministic checks run before LLM review.
- Memory is typed product state, not loose prose logs.
- Visual debugging is part of normal validation, not a late feature.
- Creative modules attach to the kernel after the kernel can govern work.

## 4.1 Build Reset Operating Mode

The reset is a build-order shift, not a rejection of the existing product implementation. The current Handshake product codebase remains the target foundation to build upon. Existing implemented product code should be treated as a good implementation of the Master Spec unless local evidence proves a specific defect.

During the Kernel V1 build, assume the external ACP/repo-governance workflow may be broken or overgrown. Do not spend product-build time patching repo governance unless the issue blocks safe product edits, restartability, Task Board/Build Order/WP/microtask updates, or validator handoff.

The working role for this mode is `KERNEL_BUILDER`: a temporary hybrid of Orchestrator and Coder. It may create broad Kernel V1 WPs and edit product code, but it does not validate, merge, or issue PASS/FAIL. Validation remains a separate Classic Validator or Operator-designated validator responsibility.

Refinement and spec enrichment should be kept to the minimum needed for speed and clarity. Large WPs are acceptable, but each WP and microtask must contain enough detail for a capable model with no chat context to implement the work and for a validator to review it.

Storage direction is non-negotiable for the reset: Handshake is a harness for parallel swarm agents and the Operator working at the same time. PostgreSQL plus CRDT are the product direction. Existing SQLite-backed code, tests, or storage helpers are migration/removal targets, not future architecture, fallback paths, or acceptable Kernel V1 scaffolding. Any SQLite usage in the current external repo-governance harness is legacy harness debt and must not be copied, defended, or carried forward as a Handshake product pattern.

## 5. Corrected State Model

CRDT should be part of the day-one architecture because Handshake must support parallel work inside the app:

- the Operator edits while models work
- multiple model sessions work on related project surfaces
- creative boards, plans, documents, and task views update live
- conflicts are merged without manual file conflict pain

However, CRDT should not be the sole authority for final workflow truth. The recommended split is:

```text
CRDT Layer
- live collaborative workspace state
- editable project surfaces
- canvases, notes, plans, editor buffers, task-board views, creative maps
- optimized for parallel editing and presence

Postgres/Event Ledger
- authority, audit, verdicts, receipts, IDs
- work packet accepted
- microtask complete
- test passed
- validator verdict recorded
- patch promoted
- model output approved

Projection Layer
- UI views
- generated Markdown
- reports
- operator summaries

Promotion Gate
- converts live workspace edits or sandbox results into authoritative events
```

This keeps collaboration fluid without making live editable state the final source of truth.

There is no SQLite layer in this state model. Kernel V1 must not introduce or preserve SQLite as a local cache, offline replica, convenience database, test-only authority, or compatibility storage mode. If a future implementation step encounters SQLite in existing product code, the correct posture is to plan its removal or containment while moving the kernel toward PostgreSQL-backed authority and CRDT-backed collaboration.

## 6. Proposed Kernel MVP

### 6.0 Current Industry Pattern Scan

This section records the updated 2026 research stance. It should be treated as implementation guidance for the rebuild decision, not as a binding dependency list.

The major industry pattern is not "one magic agent protocol." The pattern is a split between:

```text
Durable controller
- owns workflow state, sessions, approvals, checkpoints, retries, events, and audit

LLM worker
- reasons, proposes, calls allowed tools, returns artifacts, or hands off to another worker

Tool/resource boundary
- exposes external capabilities through schemas, gates, permissions, and logs
```

Examples:

- OpenAI Agents SDK: the app/server owns orchestration, state, tools, approvals, sandboxing, handoffs, and tracing. Heuristic to copy: the product owns the run; models are workers inside product-owned state.
- Claude Code / Claude Agent SDK: subagents run in separate context windows with focused prompts and tool permissions; hooks observe or block workflow steps. Heuristic to copy: isolate context by role/task, restrict tools per role, and expose observable lifecycle events.
- Google ADK: deterministic workflow agents provide sequential, parallel, and loop execution around LLM agents; sessions have events and state; A2A defines external agent tasks/messages/artifacts. Heuristic to copy: deterministic orchestration outside the LLM, with explicit session state and external-agent task artifacts.
- LangGraph: graph execution persists checkpoints by thread, supports human-in-the-loop, time travel, and state history. Heuristic to copy: every agent/workflow step should be resumable, inspectable, and replayable from checkpoints.
- AutoGen: multi-agent teams communicate through group-chat/team runtimes, with round-robin, selector, swarm/handoff, termination conditions, and observable team behavior. Heuristic to copy: agent collaboration needs explicit routing policy and stop conditions; do not let agents free-chat forever.
- CrewAI: controlled Flows own state and execution, while autonomous Crews do flexible work inside flow steps. Heuristic to copy: keep the deterministic workflow as the frame and allow autonomy only inside bounded work cells.
- Dapr Agents / durable workflow systems: long-running agent work is backed by workflow state, child workflows, routing, observability, and restart recovery. Heuristic to copy: treat model calls and tool calls as durable workflow activities, not ephemeral chat messages.
- A2A: external agents advertise capabilities through Agent Cards and exchange tasks, messages, artifacts, status updates, streaming updates, and cancellation. Heuristic to copy: if Handshake talks to outside agents later, use task/artifact protocols instead of exposing raw internal state.
- MCP: tools, resources, prompts, sampling, elicitation, roots, and structured tool results are useful at the integration boundary. Heuristic to copy: tool schemas, output schemas, human confirmation, and audit logs.

What Handshake should copy heuristically:

- Durable `SessionRun` records instead of transient terminal/chat sessions.
- Explicit `Task`, `Message`, `Artifact`, `ToolCall`, `Checkpoint`, `Approval`, and `Cancellation` objects.
- Parallel execution through a scheduler with leases/backpressure, not through uncontrolled parallel chats.
- Role/task-specific context windows instead of one giant global prompt.
- Handoffs as typed events and artifacts, not prose-only summaries.
- Human-in-loop approval points represented as state transitions.
- Checkpoint and resume after every meaningful step.
- Trace-first debugging: every prompt bundle, tool request, result, diff, test, validation, and promotion is recorded.
- Tool permission profiles per role/session/task.
- Stop conditions, budget limits, loop counters, and stale-session recovery as mechanical state.
- External interoperability through adapters: MCP for tools/resources, A2A-like contracts for external agents, official CLIs/APIs for model execution.

What Handshake should not copy directly:

- Framework-first architecture where LangGraph, AutoGen, CrewAI, ADK, Dapr, or ACP becomes the product kernel.
- Freeform group chat as authority.
- Provider-hosted state as the only source of truth.
- Agent autonomy without typed leases, cancellation, budgets, and validation.
- Tool calls that can mutate project truth without Handshake promotion.

Updated conclusion:

Handshake should build its own control plane, but the industry confirms the shape: deterministic controller outside the LLM, durable workflow state, isolated agents/subagents, typed messages/artifacts, tool gates, traces, checkpoints, human approvals, and resumable execution.

The differentiator is still mechanical determinism. Others increasingly provide harnesses; Handshake should make the harness inspectable, replayable, local-first, CRDT-aware, and promotion-gated.

### 6.1 Deterministic Runtime

Implementation proposal:

- Build a product-native runtime with typed events for sessions, actors, work packets, microtasks, artifacts, receipts, validation, and memory.
- Store authoritative state in PostgreSQL.
- Do not add SQLite-backed runtime paths. Do not preserve SQLite as a fallback, cache, test target, local-only authority, or migration bridge for Kernel V1.
- Treat generated Markdown and UI summaries as projections over machine state.
- Keep a compact event schema from day one so models can consume small slices instead of huge context dumps.
- Add a scheduler that can run parallel model sessions with leases, backpressure, cancellation, stale-session recovery, and deterministic stop conditions.
- Represent handoffs as typed state transitions with input/output artifacts, not as loose chat messages.

Investigation:

- Event sourcing versus normalized workflow tables.
- How much state should be append-only versus mutable current-state projection.
- Whether existing governance receipt/runtime schemas can be simplified and reused as seed designs.
- Whether a graph runtime is needed immediately or whether simple typed workflow tables plus a scheduler are enough for the first kernel.
- How much to borrow from LangGraph-style checkpoints, ADK-style deterministic workflow agents, CrewAI-style flow/crew separation, and Dapr-style durable activities without adopting those frameworks as the kernel.

### 6.2 CRDT Collaboration Layer

Implementation proposal:

- Use CRDT for live editable project surfaces.
- Start with Yjs or Automerge as the main investigation candidates.
- Keep CRDT document updates separate from authoritative workflow receipts.
- Add promotion actions that convert CRDT edits into durable workflow events.

Investigation:

- Yjs for web/editor ecosystem, CodeMirror/Monaco integrations, and high-performance shared types.
- Automerge for Rust/JS portability and local-first file-style documents.
- Persistence model: CRDT updates persist through PostgreSQL-backed product state. Large artifacts may use product-managed artifact storage, but workflow/collaboration truth still resolves through PostgreSQL plus CRDT contracts.
- How to expose CRDT state to LLMs without dumping entire documents into context.

### 6.3 Local Model Runtime

Implementation proposal:

- Do not make Handshake only a wrapper around Ollama or LM Studio.
- Build a `ModelRuntime` abstraction owned by Handshake.
- Use adapters underneath it so engines can be swapped.
- Start with local inference as a first-class path.

Preliminary engine candidates:

- `llama.cpp` / GGML: best baseline for embedded local inference, broad hardware support, GGUF ecosystem, and direct low-level control.
- `vLLM`: strong for high-throughput GPU serving, PagedAttention, batching, LoRA serving, OpenAI-compatible server mode.
- `SGLang`: strong for structured generation, RadixAttention/prefix caching, high-throughput serving, agentic program execution patterns.
- `MLC LLM`: possible candidate for native deployment and compiler-based acceleration.

Investigation:

- Which engine can be embedded cleanly into a Rust/Tauri product.
- How to expose KV cache policy, LoRA hot-swap, structured decoding, speculative decoding, quantization, and routing as Handshake-native controls.
- Whether Handshake should ship one embedded baseline and treat high-throughput engines as optional installed backends.

### 6.4 Inference Lab

Implementation proposal:

- Treat inference control as a product pillar, not an implementation detail.
- Add an internal interface for inference experiments:
  - prompt/program input
  - model and adapter choice
  - sampling parameters
  - structured output schema
  - KV cache settings
  - context reuse strategy
  - benchmark result

Investigation:

- Sparse attention techniques.
- Subquadratic inference options.
- KV cache quantization and reuse.
- Speculative decoding.
- Multi-LoRA routing.
- Local adapter training and evaluation.
- Whether a small Handshake-native helper model is realistic later.

### 6.5 Sandboxed Execution

Implementation proposal:

- Model-written code should not directly touch the operator's real project files.
- A coder works inside an isolated sandbox.
- The sandbox produces a patch, artifact bundle, tests, and logs.
- Handshake validates and promotes the patch into product truth only after checks pass.

Candidates:

- Docker or Podman first for practical local sandboxing.
- SWE-ReX as a directly relevant agent sandbox interface.
- Dagger for repeatable build/test workflows as code.
- gVisor for stronger container isolation later.
- Firecracker for microVM-grade isolation later.

Investigation:

- Windows host constraints.
- GPU access inside sandbox for local models.
- File promotion model: patch copy, git apply, or content-addressed artifact promotion.
- Network policy per task.
- Secret isolation.

### 6.6 Operator Surface

Implementation proposal:

- The operator should control Handshake through Handshake, not through external terminals.
- Build the operator surface early:
  - session monitor
  - model/task queue
  - built-in terminal
  - artifact viewer
  - validation panel
  - log viewer
  - patch inspector
  - browser/debug panel
  - memory inspector
  - work packet/microtask board

Expected behavior:

- The operator surface is not a chat window bolted onto a repo. It is a state-first control room.
- The Operator can create or select a project, choose a work packet or task, choose one or more model lanes, inspect the model context capsule before launch, approve launch, watch progress, interrupt, resume, compare outputs, approve or deny tool calls, inspect diffs, run validation, and promote accepted work.
- The Operator sees model work as structured state first and transcript second.
- The UI must show what the model was asked to do, what context it received, what it claimed, what tools it used, what files it read, what files it attempted to change, what commands ran, what failed, what passed, and what still needs human judgment.
- The UI must avoid stealing keyboard focus, opening uncontrolled windows, or making terminals the main workspace.
- A built-in terminal/debug panel is still required, but it is a diagnostic surface inside Handshake.
- Parallel work should feel natural: the Operator can continue editing CRDT-backed project surfaces while models work in sandboxes or model lanes. Handshake tracks presence, conflicts, pending promotions, and validation state.
- The Operator can replay a session later from the event log without relying on memory or chat history.
- The Operator can ask "why did this happen?" and Handshake can answer from events, receipts, tool calls, diffs, tests, and visible reasoning evidence.

Day-one audit boundary requirements:

- Every model lane must pass through the Handshake session broker before it reaches project state.
- Handshake should mechanically log operator messages, model-visible context bundles, selected model, reasoning mode/effort, visible model responses, exposed planning/reasoning summaries, tool calls, tool results, file reads, proposed diffs, sandbox commands, test results, validation results, approvals, denials, and final claims.
- Exposed "thinking", planning text, chain summaries, or CLI-visible assessment text should be stored as diagnostic evidence only. It is useful for the Operator to judge whether a model is making the right assessments, but it is not the model's full private computation and must not become workflow authority.
- Tool calls, diffs, receipts, tests, validation results, and promotion events are the authoritative evidence.
- CLI-backed models should still appear inside the operator surface. The official CLI can be a backend transport, but Handshake owns the session record, event normalization, log display, diff capture, validation, and promotion gate.
- The operator must be able to inspect a task as a structured trace, not only as a transcript wall.
- The audit log must be machine-readable from day one so future diagnostics, memory, replay, and self-improvement loops can consume it without parsing prose.

Investigation:

- How to run headless sessions without stealing keyboard focus.
- How to show model work as typed state rather than transcript walls.
- How to let the operator intervene without breaking deterministic routing.
- Whether browser automation should use Playwright internally.
- Which official CLIs expose structured events versus plain stdout/stderr, and how to normalize both into one Handshake event stream.
- How to capture visible reasoning/planning text without implying that it is complete internal model reasoning.

### 6.7 Visual Debugging

Implementation proposal:

- Visual checks should be part of the default validation loop.
- The LLM should be able to inspect what it built through screenshots, traces, DOM state, layout metrics, and console/network logs.

Tools and checks:

- Playwright screenshots and trace viewer artifacts.
- Accessibility checks with axe-core.
- Layout overlap detection.
- Visual regression diffing.
- Console error capture.
- Network failure capture.
- UI state snapshot IDs.

Investigation:

- How to turn screenshots and DOM metadata into model-consumable compact evidence.
- How to detect bad layout automatically before LLM visual review.
- How to preserve UI debug artifacts as persistent project evidence.

### 6.8 FEMS / Memory

Implementation proposal:

- Repomem should not be copied into the product as-is.
- FEMS should be product-native typed memory:
  - episodic memory: what happened
  - semantic memory: durable facts and relationships
  - procedural memory: what failed and what fixed it
  - artifact graph: links between code, spec, tests, screenshots, model outputs, validations
  - retrieval policy: why a memory is injected
  - memory eval: whether retrieval improved the next action

Investigation candidates:

- Letta/MemGPT for memory hierarchy and agent-managed memory concepts.
- Mem0 for multi-level memory patterns.
- Zep for temporal knowledge graph ideas.
- Existing graph/vector/hybrid memory benchmarks.

Design stance:

- Memory must not be a prompt dump.
- Memory must be inspectable, suppressible, source-linked, and testable.
- A model should receive a small memory capsule tied to the current task, not a giant memory transcript.

### 6.9 Karpathy-Style Self-Improvement Loop

Implementation proposal:

- Use the autoresearch pattern as a controlled improvement loop:
  1. choose a measurable target
  2. isolate editable surface
  3. run model in sandbox
  4. execute test/eval
  5. accept or reject change
  6. record result as memory/training data
  7. repeat within budget

Good first targets:

- reduce failing tests
- improve benchmark score
- improve UI layout score
- improve local-model routing accuracy
- improve memory retrieval quality
- improve static-analysis findings
- reduce build time
- simplify code without losing tests

Investigation:

- Karpathy `autoresearch`.
- Generalized autoresearch loops for non-ML metrics.
- How to prevent overfitting to weak metrics.
- How to require human/operator review before promotion.

### 6.10 Deterministic Code Checking

Implementation proposal:

- Use deterministic checks before model review.
- Cloud-model validation should inspect spec intent, architecture, interconnectivity, and subtle risks, not replace basic tooling.

Candidates:

- Rust: `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`.
- TypeScript/frontend: typecheck, lint, unit tests, Playwright.
- Security/static analysis: Semgrep, CodeQL.
- Structural search/rewrite: ast-grep.
- Dependency/supply chain: cargo-deny, npm/pnpm audit where appropriate.
- UI/accessibility: Playwright, axe-core.

Investigation:

- Which checks should be blocking per microtask versus per work packet.
- How check results become typed evidence.
- How to map static findings to code ownership and work-packet rows.

### 6.11 Cloud Model Constraints

Implementation proposal:

- Handshake must not depend on unofficial automation of consumer chat UIs.
- Local models should be the native default.
- API keys are optional adapters.
- Subscription tools can remain useful through official paths, official CLIs, and handoff flows.
- A Cursor/Copilot/Antigravity-style cloud model picker backed by Handshake's own app subscription is not a day-one option.

Important constraints:

- ChatGPT subscription billing and OpenAI API billing are separate.
- Claude Code can use Pro/Max subscription through official Claude Code login/CLI paths.
- Arbitrary embedding of Claude/GPT into Handshake should not rely on browser scraping, reused cookies, OAuth-token capture, hidden browser control, or pretending to be an official client.
- Commercial AI IDEs that expose multiple cloud models through one app login generally have their own provider agreements, billing arrangements, request meters, abuse controls, compliance posture, and backend routing. Handshake does not have those day one.
- The Operator's personal ChatGPT, Claude, Google, Cursor, or Copilot subscriptions do not automatically grant Handshake the right or technical ability to embed those same models through Handshake's own UI unless an official provider path supports it.

Why the commercial IDE approach is not the day-one Handshake approach:

- It requires provider contracts or aggregator agreements.
- It requires a billing/metering system and budget controls.
- It creates financial exposure if a model lane loops or overuses expensive reasoning models.
- It requires provider-specific safety, data-handling, abuse-prevention, and account controls.
- It would shift early work away from the real product edge: deterministic local-first governance, model orchestration, sandboxes, CRDT collaboration, memory, validation, and auditability.
- It would make Handshake depend on external commercial access before its own kernel is useful.

What does work for Handshake:

- Native local inference lane:
  - Handshake owns the local runtime abstraction.
  - Engines such as llama.cpp, vLLM, SGLang, or MLC LLM can sit under adapters.
  - This supports local models, LoRA experiments, inference controls, model routing, and later self-training research.
- Official CLI bridge lane:
  - Handshake launches an official CLI such as Claude Code, Codex, or Gemini CLI as a controlled backend worker when that tool's terms and auth path allow it.
  - The Operator works inside Handshake, not the terminal.
  - Handshake captures visible output, structured events when available, stdout/stderr when not, diffs, command results, and session receipts.
  - The CLI is transport; Handshake is the product surface and authority.
- BYOK API lane:
  - The Operator provides an API key later if budget allows.
  - Handshake calls the provider through documented APIs.
  - This gives stronger automation and structured tool-calling, but pay-per-token cost is explicit.
- Manual handoff/import lane:
  - Handshake prepares a task capsule.
  - The Operator runs it in an external official tool.
  - Handshake imports the result as an artifact and still handles validation and promotion.
- Future Handshake-managed cloud lane:
  - Only later, if the project has provider agreements, billing, quota, abuse controls, and data-policy infrastructure.

Safe MVP lanes:

- Native local model execution inside Handshake.
- Optional official OpenAI/Anthropic API-key adapters.
- Official CLI bridge adapters where the provider exposes an allowed CLI/subscription path:
  - Handshake launches the CLI as a controlled backend worker.
  - The Operator interacts through Handshake, not the terminal.
  - Handshake captures stdout/stderr or structured events, normalizes them into the audit log, and displays the session in the operator surface.
- External assistant handoff:
  - Handshake prepares a task capsule.
  - Operator runs it in Claude Code, ChatGPT, Codex, or another official tool.
  - Handshake imports the result as an artifact.
  - Validation and promotion still happen inside Handshake.

Investigation:

- Official CLI integration surfaces.
- What can be automated under each provider's allowed interface.
- How to keep subscription-based work useful without making it a kernel dependency.
- Whether an OpenRouter-style aggregator is acceptable later as an optional paid adapter, without making it a core dependency.

### 6.12 Minimal Technical Contracts

The first kernel should define contracts before building large features. The exact schema can change, but a no-context model should preserve the following shape.

Core services:

```text
Operator Surface
  -> Session Broker
    -> Model Adapter
    -> Tool Adapter
    -> Sandbox Runner
    -> Validation Runner
    -> Memory Service
    -> Event Ledger
    -> CRDT Workspace
    -> Projection Builder
```

Model adapter interface:

```json
{
  "adapter_id": "claude-code-cli",
  "lane_type": "OFFICIAL_CLI",
  "model_family": "claude",
  "auth_path": "provider_official_cli_login",
  "supports_streaming": true,
  "supports_structured_tool_events": "unknown",
  "supports_visible_reasoning": "provider_dependent",
  "cost_mode": "operator_subscription_or_provider_limit",
  "kernel_dependency": false
}
```

Model invocation event:

```json
{
  "event_type": "MODEL_INVOCATION_STARTED",
  "event_id": "evt_...",
  "session_id": "ses_...",
  "actor_id": "actor_model_...",
  "task_id": "task_...",
  "adapter_id": "claude-code-cli",
  "model_label": "provider_selected_or_operator_selected",
  "reasoning_mode": "visible_if_known",
  "context_bundle_id": "ctx_...",
  "input_artifact_ids": ["art_..."],
  "created_at": "timestamp"
}
```

Tool call event:

```json
{
  "event_type": "TOOL_CALL_REQUESTED",
  "event_id": "evt_...",
  "session_id": "ses_...",
  "tool_call_id": "tool_...",
  "tool_name": "read_file",
  "arguments_hash": "sha256_...",
  "arguments_projection": "safe_operator_readable_summary",
  "approval_required": true,
  "status": "requested"
}
```

Audit evidence event:

```json
{
  "event_type": "MODEL_VISIBLE_REASONING_CAPTURED",
  "event_id": "evt_...",
  "session_id": "ses_...",
  "source": "cli_stdout_or_structured_event",
  "evidence_class": "diagnostic_not_authority",
  "text_artifact_id": "art_...",
  "redaction_state": "none_or_applied"
}
```

Promotion event:

```json
{
  "event_type": "ARTIFACT_PROMOTED",
  "event_id": "evt_...",
  "source": "sandbox_patch_or_crdt_change",
  "artifact_id": "art_...",
  "validation_report_id": "val_...",
  "operator_approval_id": "approval_...",
  "target_state": "authority"
}
```

Hard contract rules:

- Every event has a stable ID, timestamp, actor, session, source, and schema version.
- Human-readable text is stored as an artifact linked to an event, not as hidden authority.
- The event ledger can replay what happened without relying on provider chat history.
- CRDT edits are not final until a promotion event accepts them into authority state.
- Sandbox outputs are not final until validation and promotion events accept them.
- Visible reasoning is always marked diagnostic, never authoritative.
- A model may propose; Handshake records, checks, and promotes.

## 7. Proposed First Build Slice

### 7.0 Build First Proposal

First build target:

```text
HSK-KERNEL-001: Event Ledger + Session Broker Proof
```

Build this before local-model depth, CRDT depth, sandbox depth, memory depth, creative modules, or a full operator UI.

The first proof is not "can a model code inside Handshake?" The first proof is "can Handshake govern a model worker through product-owned state without relying on terminal history, chat memory, provider state, or prose-only workflow rules?"

The first vertical slice should run this full path:

```text
Operator creates a task
-> Handshake creates durable IDs and a SessionRun
-> Session Broker dispatches to a dummy model adapter or trivial local echo adapter
-> adapter receives a stored context bundle
-> adapter emits a visible response, optional tool request, and artifact proposal
-> Tool Gate records allow/deny
-> Artifact Store records output, logs, and evidence
-> Validation Runner records a pass/fail result
-> Operator approves or rejects
-> Promotion Gate records final authority transition
-> Operator trace view replays the whole session from durable state
```

Why this comes first:

- It proves Handshake, not a provider chat UI or terminal buffer, owns the work.
- It establishes the product authority model before any model is trusted.
- It forces the first implementation to create durable IDs, event schemas, artifacts, approvals, and promotion rules.
- It gives CRDT, sandboxing, memory, local inference, validation, and creative modules a stable control plane to attach to.
- It reduces the risk of rebuilding another external governance harness because the first useful thing is already inside the product kernel.
- It allows model integration to start with a dummy adapter, so architecture can be validated before spending effort on inference engines or cloud-provider edge cases.

What it should achieve:

- A durable append-only event ledger exists.
- A session can be started, observed, closed, and replayed from product state.
- A task has stable actor/session/artifact/context/validation/approval IDs.
- A model adapter cannot directly mutate authority state.
- A tool request is represented as a typed event with approval status.
- A model output is stored as an artifact, not hidden transcript text.
- A validation result is stored as machine-readable evidence.
- A promotion event is the only path from proposed output to authority state.
- A minimal operator trace panel can answer:
  - what task was launched
  - which actor/model lane ran
  - what context bundle it received
  - what it returned
  - what tool request happened
  - what artifact was proposed
  - what validation ran
  - who approved or rejected
  - what became authoritative

What it does not need yet:

- Real local model quality.
- Multi-agent parallelism.
- Full CRDT collaboration.
- Full sandbox patch application.
- Full memory retrieval.
- Full creative module support.
- Provider-specific cloud-model embedding.
- Perfect UI.

Minimum components:

- `EventLedger`: Postgres-backed append-only events with schema version, event ID, actor ID, session ID, source, timestamp, and artifact links.
- `SessionBroker`: starts, dispatches, pauses/cancels if possible, closes, and records session state transitions.
- `ContextBundle`: stored input package for the model adapter, with hash and source references.
- `ModelAdapter`: dummy or trivial local adapter first; real adapters later.
- `ArtifactStore`: stores model outputs, context bundles, logs, validation reports, and operator-visible evidence.
- `ToolGate`: records tool requests and allow/deny decisions, even if the first tool is fake or read-only.
- `ValidationRunner`: records one deterministic validation result.
- `PromotionGate`: records approved transition from proposal to authority state.
- `TraceProjection`: minimal UI or generated projection that replays one session from the ledger.

Acceptance criteria:

- The entire first task can be reconstructed from the ledger after restarting the app/backend.
- No authority state changes occur without a promotion event.
- No SQLite database, SQLite-backed cache, SQLite offline mode, SQLite test fixture, or SQLite compatibility path is introduced or accepted for Kernel V1.
- The model adapter can be replaced without changing event authority semantics.
- The trace projection does not depend on provider chat history, terminal scrollback, or model memory.
- A no-context model can inspect the stored events and understand what happened.
- The same event contracts can later support local models, official CLI bridges, BYOK APIs, CRDT promotion, sandbox promotion, memory capture, and self-improvement loops.

### Week 1: Product Kernel State

- Define product-native IDs.
- Define actor/session/work-packet/microtask/artifact schemas.
- Implement Postgres-backed event ledger.
- Implement generated projections.
- Add minimal UI to inspect state.
- Add the audit event contract before any model lane is allowed to mutate project state.

### Week 2: CRDT Workspace + Promotion

- Add CRDT-backed live project document/workspace.
- Add operator/model parallel edit proof.
- Add promotion gate from CRDT workspace change to authority event.
- Persist CRDT updates.

### Week 3: Sandbox Execution

- Add sandbox runner.
- Add patch/artifact/log capture.
- Add validation gate.
- Add patch promotion into authority state.

### Week 4: Local Model + Memory V0

- Add `ModelRuntime` adapter interface.
- Integrate one local backend.
- Add typed memory records.
- Add retrieval capsule for a microtask.
- Run one self-improvement loop against a measurable check.

## 8. What To Freeze From Current Governance

Freeze external repo governance to:

- protect the repo
- create minimal packets if needed
- run validation
- preserve traceability
- avoid destructive operations

Stop expanding:

- role protocol complexity
- external packet paperwork
- duplicated Markdown/JSON sidecars
- governance-only repair loops
- broad documentation sync work not needed for product kernel work

The new rule should be: if a governance improvement belongs in Handshake, build it in Handshake unless the external shell is blocking safe work.

## 9. Research Anchors

- llama.cpp: https://github.com/ggml-org/llama.cpp
- vLLM: https://docs.vllm.ai/en/latest/
- SGLang: https://docs.sglang.io/
- MLC LLM: https://llm.mlc.ai/docs/
- Yjs: https://docs.yjs.dev/
- Automerge: https://automerge.org/
- Letta/MemGPT: https://docs.letta.com/guides/agents/architectures/memgpt
- Mem0: https://docs.mem0.ai/
- Karpathy autoresearch: https://github.com/karpathy/autoresearch
- SWE-ReX: https://github.com/SWE-agent/swe-rex
- Dagger: https://docs.dagger.io/
- gVisor: https://gvisor.dev/docs/
- Firecracker: https://github.com/firecracker-microvm/firecracker
- Semgrep: https://semgrep.dev/docs/
- CodeQL: https://codeql.github.com/docs/
- ast-grep: https://ast-grep.github.io/
- Playwright trace viewer: https://playwright.dev/docs/trace-viewer
- axe-core: https://www.deque.com/axe/axe-core/
- OpenAI billing separation: https://help.openai.com/en/articles/9039756
- Claude Code Pro/Max usage: https://support.claude.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan
- OpenAI Agents SDK: https://developers.openai.com/api/docs/guides/agents
- Claude Code subagents: https://code.claude.com/docs/en/sub-agents
- Claude Code hooks: https://docs.anthropic.com/en/docs/claude-code/hooks
- Google ADK workflow agents: https://adk.dev/agents/workflow-agents/
- Google ADK session state: https://adk.dev/sessions/state/
- A2A protocol specification: https://a2a-protocol.org/latest/specification/
- LangGraph persistence/checkpoints: https://docs.langchain.com/oss/python/langgraph/persistence
- Microsoft AutoGen teams: https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/tutorial/teams.html
- CrewAI introduction: https://docs.crewai.com/en/introduction
- Dapr Agents core concepts: https://docs.dapr.io/developing-ai/dapr-agents/dapr-agents-core-concepts/
- MCP tools specification: https://modelcontextprotocol.io/specification/2025-06-18/server/tools
- MCP prompts specification: https://modelcontextprotocol.io/specification/2025-06-18/server/prompts
- MCP sampling specification: https://modelcontextprotocol.io/specification/draft/client/sampling

## 10. Open Questions Before Rebuild

1. Should Handshake V2 be a clean product branch, a new repo, or a new kernel crate/app inside the current repo?
2. Which state is authoritative in Postgres, and which state is live-collaborative in CRDT?
3. Which local model backend is the first embedded baseline?
4. What is the first self-improvement metric?
5. What is the first creative module to attach after the kernel is usable?
6. What operator surface is the minimum viable control room?
7. How much of current code is salvageable versus reference-only?
8. How should the Master Spec be split into indexed machine-readable contracts?
9. What is the smallest validation path that preserves current rigor without recreating external governance bloat?

## 11. Recommended Decision

Restart the build order, not the vision.

Use the existing project as source material. Keep the useful spine, strict validation, and product dreams. Stop treating the external governance harness as the product. Build the product-native deterministic kernel first, then let Handshake govern its own future work.

## 12. No-Context Acceptance Criteria

A model with no chat history should be able to read this brief and correctly infer:

- The Operator is not abandoning Handshake; the Operator is changing build order.
- The Master Spec remains reference material, not the day-one execution shape.
- The first target is a deterministic product kernel, not a creative module and not another external governance harness.
- CRDT is required day one because the Operator and models must work inside the app in parallel.
- Postgres/event state is required day one because live collaboration state is not final authority.
- SQLite is explicitly rejected going forward. Future models must not propose SQLite authority, fallback, cache, offline, test-fixture, or compatibility paths for Kernel V1.
- The operator surface is the primary control room and audit boundary.
- The terminal may exist as a backend transport or debug panel, but it is not the Operator's main workspace.
- A commercial AI IDE model-subscription approach is not available to Handshake day one because it requires provider agreements, billing, quota, and abuse-control infrastructure.
- Handshake can still use cloud-capable model work through official CLI bridges, BYOK APIs, manual handoff/import, and future managed cloud lanes.
- Visible model reasoning/planning output is valuable diagnostic evidence but not complete internal thinking and not workflow authority.
- Tool calls, diffs, test results, validation results, receipts, approvals, and promotion events are authority evidence.
- Model-written code must start in a sandbox and only move into authority state through validation and promotion.
- Memory must be typed, source-linked, inspectable, suppressible, and evaluated.
- Self-improvement loops must be metric-bound, sandboxed, and promotion-gated.
- Current industry patterns support the reset: durable controllers, checkpoints, handoffs, isolated subagents, tool gates, traces, and human approvals are now mainstream.
- Handshake should copy those heuristics, but not adopt any single external framework as the kernel.

If a future model proposes work from this brief, it should start with a small kernel implementation plan around event contracts, operator-surface audit logging, CRDT workspace proof, sandbox runner, model adapter interface, and validation promotion gates.
