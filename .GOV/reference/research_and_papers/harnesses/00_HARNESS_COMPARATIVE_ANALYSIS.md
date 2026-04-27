# Harness Comparative Analysis — Why Handshake Is Burning Tokens, and How Pi/Hermes/OpenClaw/Gastown Avoid It

> Synthesis of four deep-dive harness studies, written for Handshake.
> Date: 2026-04-26
> Drafts referenced (read these alongside this document):
> - `01_pi.md` — Pi (`badlogic/pi-mono`)
> - `02_hermes.md` — Hermes Agent (`NousResearch/hermes-agent`)
> - `03_openclaw.md` — OpenClaw + ACPX (`openclaw/openclaw`, `openclaw/acpx`)
> - `04_gastown.md` — Gastown (`steveyegge/gastown`)
>
> Local clones inspected at `harnesses/` (Pi, Hermes, OpenClaw, OpenClaw-ACPX, Gastown).

---

## TL;DR

Handshake's token burn is not a model problem, an ACP problem, or a microtask problem. It is **a state-in-documents problem**. The four harnesses studied differ wildly in scope (Pi is a 1-process loop, Gastown is a multi-agent town), but every single one of them refuses to put coordination state in markdown files the model authors. They all do one of two things:

1. **Put state in machine-written structured rows** (Hermes' SQLite + FTS5 sessions DB; OpenClaw-ACPX's `~/.acpx/sessions/*.json`; Gastown's Dolt/beads ledger), with named-verb messages between roles when they exist.
2. **Refuse to have inter-role state at all** and let the conversation transcript be the only artifact (Pi's single-loop architecture, Hermes' single-agent + `delegate_task` model).

Handshake currently sits in a third position — state in markdown documents *the model authors as part of the turn*, with multiple roles writing to overlapping artifacts (WP packet, dossier, receipts, RGFs). That is the source of the artifact-repair tax, the 110M-token orchestrator runs, and the inversion where smarter models fail to translate into faster work. The smartness is being spent on bookkeeping the work, not doing it.

The prescription is not "delete ACP" or "go back to operator-relay". It is **adopt a wire format that no role authors and no role validates** — typed events between roles, machine-written state, model-visible context split from audit context — and *delete* the document layer that today every role has to author and repair. Three of the four harnesses get there with about a day's worth of primitives (a queue, a typed message schema, a hook). The hard part is not building it. The hard part is letting go of the documents.

---

## 1. The four harnesses at a glance

| Axis | Pi | Hermes | OpenClaw / ACPX | Gastown |
|---|---|---|---|---|
| **Scope** | Single-process, single-loop coding agent | Single-agent autonomous agent + gateway | Personal AI bouncer across 20+ chat channels | Multi-agent town, role taxonomy, federation |
| **Roles** | None — one agent | None — `delegate_task` for sub-tasks (depth 2) | None — channel adapters route to one agent | Mayor / Witness / Refinery / Polecat / Deacon / Boot / Dog |
| **Inter-role protocol** | None | None (sub-agents return JSON) | ACP between client/agent (acpx) | Mail (named verbs) + Nudge (queue) + ACP (stream) |
| **Audit artifact** | One JSONL session file (tree-structured) | SQLite SessionDB with FTS5 | NDJSON event stream + small per-session JSON record | Dolt SQL server with git versioning |
| **What the model authors** | Tool calls + final reply | Tool calls + `<think>` + final reply | Tool calls + final reply | Tool calls + final reply (+ named-verb mail bodies) |
| **What state lives outside model context** | Tool-result `details`, session file metadata | Memory.md / Skills / Session DB rows | ACP events, queue lease files, session record | Beads (work, mail, identity, role), JSONL events |
| **Cache stability rule** | Stable-prefix caching + iterative compaction | Hard policy: no mid-conversation system-prompt mutation (`AGENTS.md:521-535`) | Stateless — agent owns its own context | Per-session — `gt prime` only fires on hooks |
| **Steering / nudging** | First-class loop primitives (`PendingMessageQueue`, `mode: "all"\|"one-at-a-time"`) | `/steer` queue drained pre-API-call | `QueueSubmitRequest` IPC; one owner per session | Nudge queue: JSON files, FIFO, rename-claim, TTL, requeue-on-failure |
| **Hook surface** | `beforeToolCall` / `afterToolCall` | `pre_tool_call` / `post_tool_call` / `transform_tool_result` | Per-channel adapters; permission policy modes | Provider-specific JSON hooks (`SessionStart` / `UserPromptSubmit` / `Stop` / `PreCompact`) |
| **Stuck/safety stance** | No max-iter; user is watchdog; extension hooks add gates | Iteration budget (default 90); depth-2 delegation; kill-switch | Sandbox by session key; DM pairing; permission modes | Three-tier watchdog (Daemon→Boot→Deacon→Witness) |
| **Most distinctive idea** | "Don't make a protocol where a function call works" — `tool result → next assistant turn` is the only handoff | Cache-stability as policy; same `<tool_call>` XML across training/runtime/storage; `execute_code` collapses N-call pipelines | One queue owner per session; typed IPC messages; SOUL.md / AGENTS.md / state separation | Named-verb mail protocol; hook-driven `gt prime` self-rehydration; forbid agent-local TODO/MEMORY |

---

## 2. Why Handshake is burning tokens — direct answers to operational questions

The five questions answered in this section came from a **parallel governance-audit track** the operator was running with another model — a smaller, narrower investigation grounded only in observed Handshake failures (110M-token orchestrator runs, malformation-every-time, post-MT branch staleness, judgment-note authoring overhead). Those questions were not generated from the harness study; the convergence between the operational diagnosis and the comparative-harness diagnosis is independent corroboration. Each question is answerable from the four harness studies.

### 2.1 "What causes the malforment every time?"

Three structural causes, not one bug:

**(a) The artifact has too many writers.** A WP packet is currently authored and edited by orchestrator, coder, and validator — sometimes within the same WP. OpenClaw-ACPX's lesson is `src/cli/queue/ipc.ts`: **exactly one queue owner per session**, every other caller sends typed `QueueSubmitRequest` / `QueueCancelRequest` / `QueueSetModeRequest` messages and gets typed responses. There is no document any other process can corrupt because there is no document. Generation numbers (`assertOwnerGeneration`) catch the rare race when an owner has been replaced (`openclaw-acpx/src/cli/queue/ipc.ts:89-101`). Handshake's WP packet violates this rule continuously: every role is a co-writer.

**(b) The artifact is trying to be both audit and model context.** Pi makes the split rigorously (`packages/agent/src/types.ts:291-302`): every tool result has `content` (model-visible) and `details` (UI/log only). The unified diff is `details`; the model only sees `"Successfully replaced N block(s) in foo.ts."` Handshake's receipts and dossiers carry both at once — they have to be human-auditable *and* model-readable on the next turn — which means they accumulate metadata the model will misformat the moment it touches them.

**(c) Templates are buried in protocols, not surfaced to the role authoring them.** When the WP packet schema lives in a 200-line protocol document the model has to read separately, the model approximates it from memory. Hermes' answer (`run_agent.py:3463-3475`) is to bake the canonical format into the system prompt template *itself*, plus a `coerce_tool_args()` function (`model_tools.py:382`) that **repairs LLM type errors against the JSON Schema before dispatch**. The model never has to remember "edits is an array, not a string" — the harness fixes it silently. Pi does the same with `prepareArguments` shims that fold legacy `oldText`/`newText` into `edits[]`, normalize Unicode in fuzzy match, and accept `edits` as a JSON string when models like Opus 4.6 / GLM-5.1 send it that way (`packages/coding-agent/src/core/tools/edit.ts:90-114`, `edit-diff.ts:34-55`). **Whenever a model is observed to malform an artifact in a known, deterministic way, the right fix is a one-function shim that absorbs it silently, not a workflow loop that repairs the document over multiple turns.**

The combined diagnosis: malformation is unavoidable when (a) many writers, (b) audit-and-context conflated, (c) templates not enforced at the wire. Fix any one of these and malformation drops; fix all three and it disappears.

### 2.2 "Is the orchestrator required to write compact judgment notes? Is it a blocker if it doesn't?"

It is **self-imposed**. None of the four harnesses requires a coordinator role to author judgment notes between turns. The audit trail is a byproduct of doing the work, not a separate document the model has to compose:

- **Pi**: the JSONL session file is the audit (`packages/coding-agent/docs/session.md`). Tree-structured, append-only, every event with `id`/`parentId`. The model never reads it back; the human navigates it via `/tree`/`/fork`.
- **Hermes**: the SessionDB SQLite + FTS5 is the audit. `session_search` is a *tool* the model can call when it needs prior context, but the agent does not have to repopulate it. `MEMORY_GUIDANCE` (`agent/prompt_builder.py:144-162`) explicitly says "Do NOT save task progress, session outcomes, completed-work logs, or temporary TODO state to memory; use session_search to recall those from past transcripts."
- **OpenClaw-ACPX**: the NDJSON ACP event stream is the audit. Every `tool_call` / `tool_call_update` / `session/update` is structured. The README is explicit: there is no role authoring receipts.
- **Gastown**: structured beads with named labels (`thread:abc123`, `from:gastown/Toast`, `cc:mayor/`) — created with one CLI call, queryable via `bd list`.

Handshake's "compact judgment notes for the WP Validator PASS, the Windows-only scheduler-probe residual, and the closeout transition" maps to three pieces of structured data the *runtime* could write automatically:

1. The WP Validator PASS verdict — already a typed verdict, just needs to be a row, not prose.
2. The Windows-only scheduler-probe residual — already a typed concern, ditto.
3. The closeout transition — already a phase change, ditto.

If those three signals lived in the event log as `{verdict: PASS, mt: 1}`, `{concern: scheduler-probe-windows-residual}`, `{phase_transition: CLOSEOUT}` rows, no model would need to author judgment notes mid-conversation. **The "compact judgment note" is the orchestrator translating structured truth back into prose for itself, which is a sign the structured truth is not yet the contract.**

Is it a blocker today? Likely yes, because the dossier-sync job *reads* prose and projects it into the live artifact. The fix is the same fix as the malformation question: structured events between roles, prose only at the operator-facing layer (and even then, machine-projected from events).

### 2.3 "Why has the orchestrator burned 110M tokens on a single WP?"

Three combinable causes, each visible in the harness studies:

**(a) Cache invalidation on every doc revision.** Hermes' `AGENTS.md:521-535` is the canonical statement: *"Do NOT implement changes that would alter past context mid-conversation, change toolsets mid-conversation, [or] reload memories or rebuild system prompts mid-conversation."* If the WP packet is in the orchestrator's prompt history (or worse, the system prompt), every packet update is a cache miss. Anthropic prompt caching is up to 75% input-token reduction (`run_agent.py:9725-9737`). The cost of *not* caching for a long-running session is roughly 4× the input bill — exactly the 4× factor that turns "one MT, three turns" into "one MT, twelve cache-missing turns at full price". Hermes injects ephemeral context into the **user message** with a `<memory-context>` fence and system-note disclaimer (`agent/memory_manager.py:66-80`), preserving the cached prefix.

**(b) Re-reading prior receipts on each turn.** Every time the orchestrator fetches the current WP state, it reads the packet, the dossier, the latest receipts. Each read is tokens through the prompt. Pi's solution is the `details` field — UI/log metadata that **never enters the model's context**. Bash output truncation paths only `details`; the model sees the captured output but not the temp-file path (`packages/coding-agent/src/core/tools/bash.ts`). Handshake currently does the opposite: dossier judgment-notes go into the next turn's prompt because the orchestrator has to "remember" what it decided.

**(c) Babysitting per-MT validators with separate sessions.** Handshake routes per-MT validation through a separate model session via ACP. The round-trip cost is: orchestrator turn → validator session start → validator reads packet + receipts → validator emits verdict → validator turn ends → orchestrator reads verdict → orchestrator updates dossier → orchestrator next turn. Each "→" is a model call with a new (uncached for the validator) prompt. Pi's lesson is brutally direct: a WP_VALIDATOR per microtask **could be a synchronous `afterToolCall` hook on the coder session that calls `completeSimple()` inline** the way pi's compaction calls the same provider for summarization (`compaction.ts:574-578`). One model call, no session boundary, no protocol packet. *If the validator must be a separate role for governance reasons, fine — but the round-trip should not require document authoring on either end.* The validator should consume an event ("coder finished MT-N with diff X, tests Y") and emit an event ("PASS" or "FAIL with reason Z"), neither of which is a document.

The 110M-token figure is the integral of these three causes over 8–20 cycles of orchestrator activity. Even one cycle of "rebuild prompt, re-read dossier, dispatch validator, await result, update dossier, rebuild prompt" can run to 5–10M tokens at a 200k context window. A WP that takes 12 such cycles puts you at 60–120M, exactly where you are.

### 2.4 "Why has microtasks + ACP been so detrimental to speed and accuracy compared to the operator-relay days?"

The operator was an unrecognized compression layer. In operator-relay days, the human read the coder's output, decided what mattered, decided the next step, and typed a focused message to the next session. The "packet" that crossed each boundary was a paragraph or two, hand-curated, with everything irrelevant discarded. There was no document to repair because there was no document.

Microtasks + ACP without that compression layer reproduces the operator's *job* (read, decide, route, format) inside model sessions but at a much higher token cost per pass and with no stable schema for what crosses each boundary. Three specific pathologies:

**(a) Microtasks split work but multiplied handoffs.** Each MT now requires its own packet entry, its own validator round-trip, its own receipt. The "small atomic unit" was supposed to make each step cheaper; instead it multiplied the bookkeeping per unit of work. Pi's design rule (`packages/coding-agent/README.md:454-458`) is the counter: rejecting MCP, sub-agents, plan mode, and permission popups *because* each of those adds tokens to every session "behind your back". Each rejected feature cites observability or token cost as the reason.

**(b) ACP gave you a transport but you kept the documents.** OpenClaw-ACPX's headline insight (`VISION.md`) is that ACP is enough — there is no second contract. ACP carries `tool_call`, `tool_call_update`, `session/update`, `session/request_permission`, and that *is* the audit trail. Handshake adopted ACP as a transport but kept the WP packet, the receipts, and the dossier as parallel contracts. You are paying twice: once for the wire, once for the documents. **The receipts were never the wire format; ACP events were the wire format from day one.**

**(c) ACP without a stable session model thrashes.** Acpx solves this with one queue owner per session, lease files for ownership, and silent reconnect on owner death. Handshake's broker re-launches sessions on cancel and tears down state, requiring the next session to re-read documents to recover context. The acpx target architecture (`docs/2026-02-25-warm-session-owner-architecture.md`) names this exact problem: *"thread message → enqueue prompt → stream output → complete response — no hidden 300s wait in gateway-facing process paths"*. Detached owner lifetimes — owners stay alive in the background per session, callers exit immediately after their turn — would let Handshake keep ACP and lose the document layer.

So the answer is: **ACP did not slow Handshake down; the documents you kept around it did**. Operator-relay was fast because the operator's brain was the cache and the contract. Replacing the operator with broker + model sessions but keeping the documents means you pay for cache misses *plus* document repair *plus* round-trips, all to do what the operator did in their head.

### 2.5 "Why has model improvement not translated to faster work?"

Because the smartness is being consumed by document repair, not by inference. A 4× smarter model that has to spend its first 30k tokens reading receipts, its next 20k repairing a dossier, and its next 10k routing a validator is a 4× smarter model doing 1× the actual work — at 4× the cost, because that smartness is more expensive per token. The document layer is a flat tax on every model improvement.

The four harnesses converge on three patterns that let model improvements *land*:

1. **Tiny stable prompt.** Pi keeps system prompt + tool defs under 1k tokens (`system-prompt.ts:131-147`); Claude Code is around 10k. Hermes' assembly is more elaborate but cached and *not modified mid-conversation*. Smarter models compound when the prompt is small and stable; they do not compound when 30k tokens are dossier history.
2. **Cache stability as policy.** Hermes' rule, again. Smarter models have *higher* per-token cost. Cache misses hurt more, not less, on Opus 4.7 than on Sonnet 4.5. The system that throws away the cache on every WP update gets *worse* returns from upgrading the model.
3. **Tool-output asymmetry (`details` vs `content`).** Pi's split. The smart model sees only what it needs to think about. The audit/UI sees everything. Smarter models stay smart only when their context is curated.

Handshake violates all three: WP packets and receipts inflate the prompt, packet updates invalidate cache, and tool/role outputs round-trip through the model's context window. Upgrading from Sonnet 4.5 to Opus 4.7 in this regime gives you a more expensive, slightly smarter version of the same bookkeeping job.

---

## 3. Cross-cutting findings

These are the patterns that recur across all four harnesses and matter for Handshake.

### 3.1 The contract is structured events, not authored documents

In every harness studied, **what crosses a process or session boundary is a typed message, never a markdown document**:

- **Pi**: tool result (`{content, details, isError, terminate}`) and JSONL events (RPC mode `{type:"prompt"}`, `{type:"event"}`).
- **Hermes**: OpenAI Chat Completions message format (`{role, content, tool_calls?}`) plus the Hermes XML wire format (`<tool_call>{...}</tool_call>` / `<tool_response>{...}</tool_response>`).
- **OpenClaw-ACPX**: ACP JSON-RPC methods (`session/prompt`, `session/update`, `session/request_permission`, `tool_call`, `tool_call_update`).
- **Gastown**: named-verb mail beads (`POLECAT_DONE` / `MERGE_READY` / `MERGED` / `MERGE_FAILED` / `REWORK_REQUEST`) with fixed Subject and Body schemas, plus nudge queue JSON files.

None of these is a document. None of them carries narrative prose between roles. None of them requires a receiver to interpret intent — the message type tells you what to do. **The cost of authoring a 4-line schema is far below the cost of authoring a 4-page receipt, and the failure rate is far lower because the surface area for malformation is smaller.**

### 3.2 State lives outside the model context

When state has to persist beyond a single turn, it lives in machine-written storage that the model only reads on demand:

- **Pi**: `~/.pi/agent/sessions/--<cwd>--/<ts>_<uuid>.jsonl` — append-only, tree-structured by `parentId`, never re-injected into the prompt. AGENTS.md / CLAUDE.md walked from cwd are loaded into the system prompt at startup and **not refreshed** mid-conversation.
- **Hermes**: SQLite SessionDB with FTS5; `session_search` is a tool the model invokes deliberately. The cached system prompt (with frozen MEMORY snapshot) is not rebuilt mid-conversation.
- **OpenClaw-ACPX**: `~/.acpx/sessions/<id>.json` carries `agent command`, `cwd`, agent-side session id, lightweight turn previews — *not* full transcripts. Resume rebuilds context via `session/load`, falls back to `session/new` if load fails.
- **Gastown**: Dolt SQL backing store, beads as queryable rows. The agent calls `bd list` / `bd update` to interact with state. Local TODO/MEMORY files are *forbidden* by `AGENTS.md:155-158`.

Handshake puts state in WP packets / receipts / dossiers / RGFs — files the model reads and authors during turns. **If state is in a file the model authors, every state update is a cache miss; every state read is tokens through the prompt; every multi-writer race is malformation.**

### 3.3 Cache stability is policy, not opportunity

Two of the four harnesses have explicit policy on cache stability:

- **Hermes** (`AGENTS.md:521-535`): hard rule against mid-conversation system prompt mutation; slash commands default to "next session" with opt-in `--now`; ephemeral context goes in the user message with a `<memory-context>` fence; bit-perfect prefix normalization (`run_agent.py:9745-9776`) for KV-cache reuse on local inference.
- **Pi**: stable transcript prefix; `cache_control: ephemeral` on the last user/tool block (`packages/ai/src/providers/anthropic.ts:1090-1112`); long-cache mode via `PI_CACHE_RETENTION=long`; iterative compaction (compactions are summarized into one stable summary that doesn't grow).

OpenClaw and Gastown side-step the issue by not having a long-running shared prompt to invalidate (each role is its own session; the ACP proxy is per-session).

**Handshake's WP-packet-as-prompt-context model fights cache stability at every layer.** A policy that "WP packet edits do not enter the orchestrator's prompt history" — pulled from Hermes — would unlock prompt caching across the entire orchestrator run.

### 3.4 Deterministic absorption beats protocol repair

Each harness has an example of fixing model misbehavior in code, not in workflow:

| Failure mode | Pi | Hermes | OpenClaw-ACPX | Gastown |
|---|---|---|---|---|
| Wrong type in tool args | `prepareArguments` shim per tool | `coerce_tool_args()` against schema | (delegated to underlying agent) | (delegated to underlying runtime) |
| Smart quotes / dashes in edits | Unicode-normalized fuzzy match | — | — | — |
| Truncated `<tool_call>` / `<think>` | — | Regex matches both closed and unclosed | — | — |
| CRLF / BOM round-trip | Detected on read, restored on write | — | — | — |
| Stale tool-result orphans | — | `_sanitize_api_messages` adds stubs | — | — |
| Mid-stream nudge interruption | Steering queue at turn boundary | `/steer` drained pre-API-call | — | Nudge queue with `UserPromptSubmit` hook |
| Concurrent file mutation | Serialized via `file-mutation-queue` | — | — | (worktree isolation) |
| Surrogate pairs from copy-paste | — | `_sanitize_surrogates` | — | — |

**The pattern**: every known model failure mode that recurs deterministically becomes a one-function fix in the harness, *not* a workflow loop that retries on the model's behalf. Handshake's "the WP packet is malformed, route through the coder for repair" pattern is exactly the workflow loop these harnesses replace with code. A list of known WP packet malformation modes (truncated trailing newline, missing dossier section, wrong heading level, etc.) should each be a 10-line normalizer in the orchestrator runtime, not a model task.

### 3.5 Hooks own self-rehydration; orchestrators don't

Gastown's most transferable pattern: **`gt prime` runs in the agent's own `SessionStart` hook, not in the orchestrator's prompt-construction code** (`internal/hooks/templates/claude/settings-autonomous.json:100-110`). The agent self-rehydrates with role identity and current work assignment when its session starts; the orchestrator never has to "build the next prompt".

Pi's analog: AGENTS.md / CLAUDE.md walked from cwd, loaded once at startup. The agent rehydrates from disk; the harness doesn't decide what to put in the prompt beyond what the system-prompt template specifies.

Hermes' analog: cached system prompt loaded from SessionDB on continuation (`run_agent.py:9335-9348`); rebuilds explicitly avoided to preserve cache.

Handshake currently has the orchestrator construct the prompt for each spawned session. That makes the orchestrator (a) responsible for context completeness, (b) the bottleneck on every relaunch, and (c) the source of "I forgot to include X" failure modes. **Move prompt construction into the spawned session's startup hook.** The orchestrator drops a small directive ("WP-N, MT-K"); the hook reads beads, builds the prompt, runs the session.

### 3.6 Turn-boundary delivery, not mid-stream interruption

Three of the four harnesses have explicit primitives for delivering messages at turn boundaries instead of preempting:

- **Pi**: `PendingMessageQueue` with `mode: "all" | "one-at-a-time"` (`packages/agent/src/agent.ts:113-144`); steering messages drained after the current assistant turn (`agent-loop.ts:165, 218`); follow-up messages drained when the agent would otherwise terminate.
- **Hermes**: `_pending_steer` queue drained on the next user/tool message before the API call (`run_agent.py:9591-9639`).
- **Gastown**: nudge queue with rename-claim, FIFO, TTL, requeue-on-failure (`internal/nudge/queue.go`). The `UserPromptSubmit` hook drains it. CHANGELOG 1.0.1 documents the bug Handshake is hitting today (open mail accumulating, ballooning context to 60-70%, freezing the supervisor) and the fix (archive on `done`).

Handshake's "the orchestrator pokes terminals directly" pattern is the failure mode all three avoid. *The orchestrator should never write to a session's stdin mid-turn.* It should drop a queue file; the session's hook drains it on the next prompt.

---

## 4. The "documentation-as-handoff-blocker" anti-pattern, named

Pulling these together, the anti-pattern is precise enough to name:

> **State-in-documents anti-pattern**: a system in which the persistent state of a multi-step workflow is held in markdown (or similar) files that are (a) authored by models during turns, (b) read by other models on subsequent turns, and (c) used as the contract between roles. Symptoms: artifact malformation requiring repair turns; cache invalidation on every state update; multi-writer races; cost growing super-linearly with workflow length; smarter models failing to reduce wall-clock time.

Every harness studied avoids this by adopting one or more of the following:

1. **Structured events as the wire format.** Named types, fixed schemas, machine-written. (All four.)
2. **Audit / context split on tool results.** `details` (audit/UI) vs `content` (model). (Pi explicit, others implicit.)
3. **State in databases, not files.** SQLite, Dolt, JSON-record stores; the model reads via tool, never via prompt. (Hermes, Gastown, OpenClaw.)
4. **One owner per session for writes.** Typed IPC for everyone else. (OpenClaw-ACPX explicit.)
5. **Cache stability as policy.** No mid-conversation system-prompt mutation. (Hermes explicit.)
6. **Hook-driven self-rehydration.** Agents prime themselves on session start; orchestrator does not construct prompts. (Gastown.)
7. **Deterministic absorption shims** for known model misbehavior. (Pi, Hermes.)
8. **Turn-boundary delivery queues** for nudges/steering. (Pi, Hermes, Gastown.)
9. **Forbid agent-local memory files** when they would duplicate structured state. (Gastown explicit.)

Handshake adopts none of these in a load-bearing way today. WP packets are model-authored, multi-writer, in-prompt context. The validator round-trips reload them. The orchestrator rebuilds prompts. The receipt is the contract.

---

## 5. Recommendations for Handshake

In rough order of impact-per-effort. Each names the harness pattern and the specific Handshake change.

### Tier 1 — high impact, low effort, low risk

**T1.1 Move "compact judgment notes" out of model authoring.** Keep the orchestrator's verdict, concern, and phase-transition signals as structured events on the dossier-sync side. Stop having the orchestrator emit prose mid-conversation. Pattern: **Hermes' `MEMORY_GUIDANCE`** ("Do NOT save task progress, session outcomes, completed-work logs, or temporary TODO state to memory; use session_search to recall those from past transcripts" — `agent/prompt_builder.py:144-162`). For Handshake: dossier-sync writes a row per verdict/concern/transition; orchestrator queries via a tool when needed. Eliminates one of the three contributors to the 110M-token orchestrator run.

**T1.2 Adopt the `details` vs `content` split on every tool result Handshake controls.** Pattern: **Pi's `AgentToolResult.details`** (`packages/agent/src/types.ts:291-302`). For Handshake: phase-check, packet-truth, gov-check etc. all return *both* a one-line model-visible summary and a structured `details` payload that is logged but not re-injected into the prompt. The model never re-reads phase-check output; it reads "phase-check ok" or "phase-check FAIL: <one-line reason>". The dossier-sync reads the full structured payload from the log.

**T1.3 Adopt `coerce_tool_args` style normalizers** for every Handshake artifact whose malformation the orchestrator routinely repairs. Pattern: **Hermes `coerce_tool_args()`** (`model_tools.py:382`) and **Pi `prepareArguments`** (`tools/edit.ts:90-114`). For Handshake: enumerate the top-N WP-packet / receipt malformation modes from past sessions, write a 10–30-line normalizer for each, run it before the validator ever sees the artifact. Saves a retry round-trip per occurrence.

**T1.4 Inject ephemeral context into user messages, not the system prompt.** Pattern: **Hermes `<memory-context>` fence in user message** (`agent/memory_manager.py:66-80`). For Handshake: when the orchestrator must pass governance state into a coder/validator turn, inject it into that turn's *user message* with a clear "this is informational, not user input" disclaimer. The system prompt stays cached. Big input-cost win on Anthropic.

**T1.5 Standardize on a small set of named verbs between roles.** Pattern: **Gastown's `mail-protocol.md`** (POLECAT_DONE, MERGE_READY, MERGED, MERGE_FAILED, REWORK_REQUEST, …) and **OpenClaw-ACPX's typed queue messages** (`QueueSubmitRequest`, etc., `src/cli/queue/messages.ts:24-120`). For Handshake: define ~6–8 verbs (CODER_HANDOFF, MT_VERDICT, INTEGRATION_VERDICT, CONCERN, PHASE_TRANSITION, RELAUNCH_REQUEST, ABORT_REQUEST). Each has a fixed body schema (3–5 labelled lines). Roles emit and consume only these. WP packets become *projections* of the event log, not the event log itself.

### Tier 2 — architectural, medium effort, paying back over months

**T2.1 One owner per session, with typed IPC for everyone else.** Pattern: **OpenClaw-ACPX queue ownership** (`src/cli/queue/ipc.ts:89-101`). For Handshake: the broker enforces exactly one owner process per coder/validator session. Other roles submit `QueueSubmit` / `QueueCancel` / `QueueSetMode` requests. Generation numbers prevent stale-owner races. Eliminates the multi-writer-races contributor to malformation.

**T2.2 Hook-driven self-rehydration.** Pattern: **Gastown `gt prime` in `SessionStart` hook** (`internal/hooks/templates/claude/settings-autonomous.json:100-110`). For Handshake: the orchestrator dispatches by dropping a directive into a queue (e.g., "WP-1-Calendar-Sync-Engine-v3 / MT-001 / coder"). The spawned session's startup hook reads from the bead/event store and self-builds its prompt. Orchestrator stops constructing prompts. Frees ~30% of orchestrator turns by inspection of the prior 110M run.

**T2.3 Validator-as-tool-result, not validator-as-session.** Pattern: **Pi's `afterToolCall` hook + `completeSimple()` inline** (`packages/agent/src/types.ts:75-101`, `compaction.ts:574-578`). For Handshake: per-MT validators run as a synchronous tool call invoked from the coder session, returning a typed `{verdict, concerns?, residuals?}` object. No second session, no ACP round-trip, no separate prompt construction. *If the validator must be a separate model session for governance, fine — but the boundary should be "tool call returns a typed object", not "ACP relay between two free-form prompts".*

**T2.4 Turn-boundary nudge queue.** Pattern: **Gastown `internal/nudge/queue.go`** — JSON files in `<root>/.runtime/nudge_queue/<session>/`, FIFO with random suffix, atomic rename-claim, TTL (30 min normal, 2h urgent), requeue on failure. Drained by `UserPromptSubmit` hook. For Handshake: `orchestrator-steer-next` becomes "drop a JSON file"; the coder/validator session drains on next prompt. Solves both polling waste and mid-stream interruption.

**T2.5 Cache-stability rule, codified.** Pattern: **Hermes `AGENTS.md:521-535`**. For Handshake: orchestrator protocol document gets a hard rule: "the orchestrator does not modify a coder/validator session's system prompt mid-conversation. Mutations land in the bead/event store and become visible on the next session." With opt-in `--now` for the rare case of forced invalidation. This is policy, not architecture, but it disciplines the architecture against itself.

### Tier 3 — bigger lifts, larger payoff

**T3.1 Replace the WP packet as primary artifact with an event log + projections.** The packet becomes a view materialized from events on read. Each role emits typed events (above), the dossier-sync materializes the human-facing markdown from those events, the orchestrator queries events via tool. This is the full state-in-documents anti-pattern fix. Pattern reference: **Gastown beads + Dolt** (`docs/design/architecture.md:5-30`); **OpenClaw-ACPX session record + NDJSON event stream**.

**T3.2 Single-process orchestrator-managed loop where possible.** Pattern: **Pi's single-process model**. For ORCHESTRATOR_MANAGED lanes that are mostly mechanical (closeout-repair, phase-check, validator-gate), collapse them into a single loop with extension hooks rather than spawning per-role sessions. The orchestrator becomes a host running a Pi-like loop with `beforeToolCall` / `afterToolCall` hooks for governance. Validators are hooks; receipts are tool-result `details`. *Only* spawn a separate session when the work genuinely requires a different identity/model (CODER for code edits, INTEGRATION_VALIDATOR for spec judgment).

**T3.3 A `seance`-equivalent for predecessor lookup.** Pattern: **Gastown `gt seance`** (predecessor session lookup via `.events.jsonl`). For Handshake: when a session restarts after compaction, it queries its predecessor's structured events instead of re-reading WP/MT documents. Direct fix for re-entry context cost on long WPs.

---

## 6. Breakpoints, concerns, scenarios where this still breaks

A list of the things that can still go wrong after the recommendations above, with mitigations.

### 6.1 Event-log corruption

If events are the contract, a corrupt event log is a corrupt workflow. Mitigations:
- Events are append-only. The runtime never rewrites past events.
- Schema versioned; readers tolerate unknown fields.
- Replay is idempotent — projecting events to a packet view should yield the same packet view regardless of when run.
- Storage redundancy: a JSONL file plus a SQLite mirror is fine; both cheap; cross-checked at read time.

Note: OpenClaw-ACPX explicitly preserves session records on cancel rather than tearing them down. Handshake should copy this.

### 6.2 Schema evolution

Named verbs and their body schemas will need to evolve. Mitigations:
- Pin the schema version per WP. A WP started under v1 reads v1 events; v2 events are projected backward when the WP is queried.
- Reject events with unknown verbs at write time, not read time. Loud failure on emission, soft tolerance on consumption.
- Keep the verb set small (8–12). Each new verb is a structural change reviewed under RGF.

### 6.3 Validator on a stale main branch

The user's scratchpad notes one MT was built on stale main. The fix lives partly in T1.5 (named verb `BRANCH_HARMONIZATION_REQUIRED` rather than free-form prose) and partly in T2.4 (a `nudge` to the coder rather than a packet edit). The deeper issue is a separate concern: WP product branches diverging from main is a worktree-coordination problem. Mitigations:
- Refinery-style merge queue (Gastown `internal/refinery/engineer.go`) or simpler: a pre-CLOSEOUT hook that requires `git merge-base origin/main HEAD` to be ≤ N commits behind.
- Bead/event for the branch state; orchestrator queries via tool, doesn't re-read the WP packet.

### 6.4 Multi-writer collisions on the event log

Even with append-only events, two roles emitting concurrently can race. Mitigations:
- Single-owner-per-session pattern (T2.1) bounds writers per session.
- For cross-session writes (e.g., orchestrator emits `PHASE_TRANSITION`), use the same lease-file pattern (`openclaw-acpx/src/cli/queue/lease-store.ts`). Writers acquire a lease, write, release. Tiny critical section.
- Generation numbers (`assertOwnerGeneration`) catch the rare case of stale leases.

### 6.5 Orchestrator becoming the bottleneck even after recommendations

If the orchestrator still has to be in the loop on every event, it's still the bottleneck. Mitigations:
- T2.2 (hook-driven self-rehydration) explicitly removes the orchestrator from session-start prompt construction.
- T2.3 (validator-as-tool-result) removes orchestrator mediation of validation.
- The orchestrator's job becomes: dispatch, observe, decide on strategy. Not: format, route, repair, sign.

If after all this the orchestrator is still expensive, **measure where the tokens go before assuming it's the orchestrator's fault.** Hermes counts tool schemas in its preflight token estimate (`run_agent.py:9388-9394`); Handshake should add a per-tool-schema and per-event-page token accounting.

### 6.6 Models that don't support hooks (hosted-only inference)

Some providers expose only a chat-completions endpoint with no startup hook. Mitigations:
- Hooks run in the harness (the broker / spawn wrapper), not the model. The model just sees a system prompt that was assembled by the hook.
- For providers without harness control (e.g., raw API call), the harness itself runs the equivalent of `gt prime` before the first turn.
- This is a minor concern — every harness studied runs a process around the model, not inside it.

### 6.7 The "Windows-only scheduler-probe residual" class of concern

A real residual has to land *somewhere* a future role can find it. Mitigations:
- It's a typed event: `{verb: "RESIDUAL_CONCERN", platform: "windows", area: "scheduler-probe", severity: "MEDIUM", notes: "..."}`.
- Materialized into the dossier projection. Read by future validators automatically because they consume the event log.
- Not a free-form judgment note in the orchestrator's prompt. Not a markdown bullet in the dossier the model has to author. A row.

### 6.8 The user's "subscription plan only" cost discipline

All of these recommendations assume the cost shape is roughly: (a) input tokens dominated by cached prefix; (b) output tokens dominated by tool calls; (c) per-session overhead small. Subscription-plan caps are usually expressed in messages or daily tokens. Mitigations:
- Pi's design is the strongest fit here: tiny system prompt, extension hooks for everything, no MCP, no sub-agents — exactly the "stay under the cap" architecture.
- Hermes' `execute_code` (Python sandbox calling tools by RPC) collapses N-call pipelines into one turn. Direct attack on per-message cap.
- Gastown's hook-driven design means the orchestrator runs only when something happens — no idle polling.

### 6.9 Operator handover / continuity

If state is in events not documents, what happens if the user wants to read what's going on without launching a model? Mitigations:
- Dossier-sync materializes a human-readable projection of events on demand. Same content as today, but auto-generated.
- A `gt mail show` / `gt seance` equivalent CLI lets the operator browse events without invoking a model.
- The point is that the *materialization* is read-only and deterministic; no model has to author it.

### 6.10 The "we already have heavy governance, can't rip and replace" concern

True. Recommendations are sequenced (Tier 1 → 3) so each tier compounds without invalidating the prior tiers. T1 is week-of-work scale; T2 is month-of-work scale; T3 is a longer migration. None of T1 requires deleting WP packets. T2.5 (cache-stability rule) and T1.4 (ephemeral context in user message) can be in by Friday and produce immediate token savings.

---

## 7. Migration sketch — what to pilot first

A four-step pilot ordering, each step independently producing measurable wins:

**Step 1 (week 1) — Cache + tool-result hygiene.** Land T1.4 (ephemeral context in user message) and T1.2 (`details` vs `content` on phase-check / gov-check / packet-truth). Measure orchestrator input-token usage before/after. Expected reduction: 40–60% on long WPs because Anthropic prefix cache becomes useful.

**Step 2 (week 2–3) — Named verbs + normalizers.** Land T1.5 (define ~6 named verbs with fixed schemas) and T1.3 (normalizers for top WP-packet malformation modes). Migrate one WP to use named verbs; keep markdown projection for human review. Measure repair-loop turns before/after.

**Step 3 (week 4–6) — Nudge queue + hook-driven rehydration.** Land T2.4 (turn-boundary nudge queue) and T2.2 (`SessionStart` hook for coder/validator sessions). Move `gt prime`-equivalent into the spawned session's hook. Measure orchestrator turn count before/after.

**Step 4 (month 2+) — Validator-as-tool + event log primary.** Land T2.3 (validator-as-tool-result for at least the per-MT validator path) and begin T3.1 (event log as primary artifact, dossier as projection). This is the deepest change; do it after the quick wins have proven the direction.

Throughout, the rule from Hermes' `AGENTS.md:521-535` applies: **changes that would mutate the cached prompt mid-conversation are forbidden by policy, not merely discouraged.**

---

## 8. Reading order for the four drafts

If reviewing these in order:

1. **`04_gastown.md`** first. Closest peer to Handshake, same problem space, same vocabulary. The CHANGELOG 1.0.1 (2026-04-25) entry is the single most validating piece of evidence: someone else hit your bug and shipped a fix.
2. **`03_openclaw.md`** second. Direct ACP comparator — both projects use ACP, so the differences are pure architectural choices, not protocol differences. Section 6 (ACPX deep dive) and Section 11 (comparison to Handshake's ACP usage) are the load-bearing parts.
3. **`02_hermes.md`** third. The cache-stability rule, the `<memory-context>` fence, `coerce_tool_args`, the `MEMORY_GUIDANCE` text. Take the patterns even if the model coupling doesn't apply.
4. **`01_pi.md`** last. The most opinionated and the most distant from Handshake. Read for the *philosophical* claims — Zechner's blog posts and the README's list of rejected features — and the `details`/`content` split.

Each draft has a "NOTES FOR SYNTHESIZER" section at the end summarizing 5–10 key findings. Those sections are good cold-start material.

---

## 9. Sources and local references

Local clones:
- `harnesses/pi-mono` — pi
- `harnesses/hermes-agent` — hermes
- `harnesses/openclaw` — openclaw core
- `harnesses/openclaw-acpx` — acpx (the relevant ACP client)
- `harnesses/gastown` — gastown

Drafts in this folder:
- `01_pi.md` (~4,700 words)
- `02_hermes.md` (~6,000 words)
- `03_openclaw.md` (~3,800 words)
- `04_gastown.md` (~3,200 words)
- `00_HARNESS_COMPARATIVE_ANALYSIS.md` (this document)

External references most likely to be useful next:
- Mario Zechner, "Pi - A minimal coding agent", 2025-11-30
- Mario Zechner, "What if you don't need MCP?", 2025-11-02
- OpenClaw-ACPX `docs/2026-02-25-warm-session-owner-architecture.md` — describes a Handshake-shaped problem and its solution
- Gastown `docs/design/mail-protocol.md` — the named-verb mail schema reference
- Gastown CHANGELOG 1.0.1 (2026-04-25) — the verbatim "we hit your bug, here's the fix" entry
- Hermes `AGENTS.md:521-535` — the cache-stability policy text

---

## Appendix A — The five scratchpad questions, condensed answers

1. **Why malformation every time?** Multi-writer artifacts + audit-and-context conflated + templates not enforced at the wire. Fix: typed events with named verbs; one owner per session; deterministic absorption shims for known failure modes.

2. **Is the orchestrator required to write judgment notes?** No. Self-imposed. Replace with structured event rows that the dossier-sync materializes into prose.

3. **Why 110M orchestrator tokens?** Cache invalidation on every WP-packet edit (4× factor) + re-reading prior receipts on each turn + session-boundary round-trips for per-MT validation. Each is independently fixable; combined fix is ~5–10× reduction.

4. **Why ACP+microtasks slowed vs operator-relay?** The operator was a compression-and-contract layer. ACP without document discipline reproduces the operator's job in models without their compression. The fix is not to delete ACP — it's to delete the document layer that grew up alongside ACP. Microtasks split work but multiplied bookkeeping; the fix is to make per-MT bookkeeping trivial (a row, not a packet edit).

5. **Why no return on smarter models?** Smartness is consumed by document repair, not inference. A 4× smarter model paying 4× cost-per-token to do 1× of the actual work is a net cost increase. The flat tax of the document layer scales linearly with the cost of each token; smarter models hurt more, not less, until that tax is removed.

The same fix answers all five: **state lives outside model context, the wire format is typed events, and no role authors prose for another role to consume**.
