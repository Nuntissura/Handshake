# Hermes Agent — Technical Research

Repo: https://github.com/NousResearch/hermes-agent
Local clone analyzed: `harnesses/hermes-agent`
Commit state: as cloned 2026-04-26 (release lineage v0.2 → v0.11).
Sibling Nous repo referenced: `Hermes-Function-Calling` (defines the model-side tool-call schema; was cloned originally as the supervised-fine-tune repo for Hermes 2 Pro).

---

## 1. Executive summary

Hermes Agent is Nous Research's open-source, single-process autonomous agent. Its top-level entry points are an interactive CLI (`hermes`), a long-running messaging gateway (Telegram/Discord/Slack/etc.), an ACP server for editor integration, and a batch trajectory generator for RL training. All four entry points converge on one class — `AIAgent` in `run_agent.py` (~12.7k LOC, with a companion `cli.py` of ~11.1k LOC). The README pitches it as "the only agent with a built-in learning loop": it auto-creates skills from experience, reviews its own MEMORY.md across sessions, and is designed to host the Hermes model family but works equally well as an OpenAI-compatible client against ~200 providers.

What is distinctive — and the reason this is worth studying for Handshake — is the **explicit coupling between the harness and the Hermes function-calling chat template**. Nous designed the Hermes model series to emit `<tool_call>` XML, and the harness is built so that template is the canonical wire format end-to-end (training data, runtime parsing, fallback parsing, trajectory storage). When a provider supports OpenAI-style structured `tool_calls`, the harness uses those; when it does not (vLLM Hermes2Pro, llama.cpp, Ollama-hosted GLM/Qwen/Kimi), the harness falls back to regex-parsing the model's text output for the same XML tags. This means the agent's "communication protocol" is defined by the chat template, not by a custom orchestrator schema, and there is essentially zero handoff bureaucracy: a turn is just an OpenAI-format message list.

Other distinctive choices, all relevant to your friction problem:
- **Cache-stability is policy.** AGENTS.md explicitly forbids any code change that would mutate the system prompt mid-conversation. Memory updates, skill installs, and tool changes default to "next session" with an opt-in `--now` flag.
- **The orchestrator does not coordinate; it routes.** No multi-role pipeline, no document-shaped handoffs. Multi-step work happens via `delegate_task` (subagent spawn) or via `execute_code` (a Python sandbox that calls tools by RPC and collapses N tool turns into one).
- **Single agent with depth-limited delegation.** Children are isolated by `task_id`, share the OpenAI message format, return JSON results, and never write back to shared MEMORY.

---

## 2. Repo layout

Top of repo (`harnesses/hermes-agent\`):

| Path | Purpose |
|------|---------|
| `run_agent.py` (12.7k LOC) | `AIAgent` — the loop, retry/fallback, compression, persistence |
| `cli.py` (11.1k LOC) | `HermesCLI` — slash commands, banner, prompt_toolkit input |
| `model_tools.py` (670 LOC) | `get_tool_definitions()` and `handle_function_call()` — dispatch shim over `tools/registry.py` |
| `toolsets.py` (759 LOC) | `_HERMES_CORE_TOOLS`, named toolset groupings |
| `hermes_state.py` (1.7k LOC) | SQLite SessionDB with FTS5 search |
| `trajectory_compressor.py` (1.5k LOC) | Post-hoc trajectory compression for RL training data |
| `mini_swe_runner.py` | Minimal SWE-bench-style runner — second copy of the Hermes system prompt |
| `agent/` | Provider adapters, prompt builder, memory, compression, caching |
| `agent/prompt_builder.py` (1084 LOC) | System-prompt assembly |
| `agent/context_compressor.py` | Lossy summarization of middle turns |
| `agent/memory_manager.py` | Builtin + at-most-one external memory provider |
| `agent/transports/` | `chat_completions.py`, `anthropic.py`, `bedrock.py`, `codex.py` |
| `tools/` | 47+ self-registering tool modules (terminal, file, web, browser, delegate, MCP) |
| `tools/registry.py` | Tool registry — schema collection, dispatch |
| `tools/delegate_tool.py` (1900+ LOC) | Subagent spawn: leaf vs. orchestrator role, depth caps |
| `acp_adapter/` | Agent Client Protocol server (VS Code, Zed, JetBrains) |
| `environments/` | Atropos RL environments + tool-call parsers |
| `environments/tool_call_parsers/hermes_parser.py` | The reference text-mode parser for `<tool_call>` |
| `gateway/` | Long-running messaging gateway with 18 platform adapters |
| `skills/`, `optional-skills/` | Procedural memory; SKILL.md format compatible with agentskills.io |
| `website/docs/developer-guide/` | The architecture, agent-loop, prompt-assembly, trajectory-format docs (best primary source for what the codebase intends) |

Key design fact, called out in `AGENTS.md:64-74`: every tool file calls `registry.register()` at import time, and `model_tools.py` triggers discovery by importing them. There is no manual tool list — the registry is the single source of truth.

---

## 3. Architecture

### Components

The architecture diagram from `website/docs/developer-guide/architecture.md:13-49` reduces to:

```
Entry: CLI | Gateway | ACP | Batch | Python lib
        \      |       |      |       /
         \     v       v      v      /
          ───── AIAgent (run_agent.py) ─────
            /          |              \
   prompt_builder    transports     model_tools
   (system prompt)   (3 API modes)  (tool registry)
            |            |              |
       compression   chat_completions   tools/*.py
        & caching    codex_responses
                     anthropic_messages
                          |
                  Session SQLite + FTS5
```

The agent is **synchronous**. Entry points (gateway, CLI) handle their own async I/O, but `run_conversation()` itself is a straight Python while-loop. Tool calls fan out to a `ThreadPoolExecutor` only when there are multiple parallel-safe ones in a single response.

### The agent loop

`AIAgent.run_conversation()` lives at `run_agent.py:9161`. The actual loop is at `run_agent.py:9530`:

```python
while (api_call_count < self.max_iterations
       and self.iteration_budget.remaining > 0) or self._budget_grace_call:
    ...
    api_call_count += 1
    ...
    response = client.chat.completions.create(model=..., messages=api_messages, tools=...)
    if response.tool_calls:
        for tc in response.tool_calls:
            result = handle_function_call(tc.name, tc.args, task_id)
            messages.append({"role": "tool", "tool_call_id": tc.id, "content": result})
    else:
        return response.content
```

That is "generic ReAct" plus 9000 lines of operational hardening. What `AGENTS.md:114-134` calls out as the canonical loop is essentially a 12-line summary of those 9000 lines:

```python
while (api_call_count < self.max_iterations and self.iteration_budget.remaining > 0) \
        or self._budget_grace_call:
    if self._interrupt_requested: break
    response = client.chat.completions.create(model=model, messages=messages, tools=tool_schemas)
    if response.tool_calls:
        for tool_call in response.tool_calls:
            result = handle_function_call(tool_call.name, tool_call.args, task_id)
            messages.append(tool_result_message(result))
        api_call_count += 1
    else:
        return response.content
```

The hardening around it includes: pre-API-call `/steer` drain (out-of-band user guidance injected onto the next tool message — `run_agent.py:9591-9639`), surrogate-character sanitization (`run_agent.py:9778-9782`), preflight context compression when conversation is large (`run_agent.py:9376-9442`), tool-argument JSON repair (`_sanitize_tool_call_arguments`), and the IterationBudget (`run_agent.py:214-256`).

### Where the model coupling lives

Three places.

1. **`run_agent.py:3463-3475`** — when saving a trajectory for RL training, the harness rebuilds the system prompt in the canonical Hermes function-calling template (see Section 4). This is the same template that trained the Hermes 2 Pro / Hermes 3 / Hermes 4 series; the harness and the model are one ecosystem.
2. **`environments/tool_call_parsers/hermes_parser.py`** — text-mode parser for `<tool_call>` blocks, used when the provider doesn't return structured `tool_calls`.
3. **`environments/agent_loop.py:268-289`** — fallback in the RL agent loop: if `assistant_msg.tool_calls` is empty but content contains `<tool_call>`, the harness invokes the Hermes parser to recover structured calls before continuing.

The implication: at runtime against a cloud provider, Hermes Agent uses the OpenAI tool-call protocol; against a local Hermes model on vLLM or llama.cpp, the same harness gracefully degrades to regex-matching the model's emitted XML. Either way, the conversation history stored in `messages` is OpenAI-format.

---

## 4. The tool-call protocol (the key section)

Hermes has **one** protocol, expressed in two surface forms:

### Model-side: the Hermes function-calling chat template

This is defined by Nous Research and trained into every Hermes model. It is reproduced verbatim by the harness when generating training data. From `run_agent.py:3463-3475`:

```python
system_msg = (
    "You are a function calling AI model. You are provided with function signatures within <tools> </tools> XML tags. "
    "You may call one or more functions to assist with the user query. If available tools are not relevant in assisting "
    "with user query, just respond in natural conversational language. Don't make assumptions about what values to plug "
    "into functions. After calling & executing the functions, you will be provided with function results within "
    "<tool_response> </tool_response> XML tags. Here are the available tools:\n"
    f"<tools>\n{self._format_tools_for_system_message()}\n</tools>\n"
    "For each function call return a JSON object, with the following pydantic model json schema for each:\n"
    "{'title': 'FunctionCall', 'type': 'object', 'properties': {'name': {'title': 'Name', 'type': 'string'}, "
    "'arguments': {'title': 'Arguments', 'type': 'object'}}, 'required': ['name', 'arguments']}\n"
    "Each function call should be enclosed within <tool_call> </tool_call> XML tags.\n"
    "Example:\n<tool_call>\n{'name': <function-name>,'arguments': <args-dict>}\n</tool_call>"
)
```

A complete training-format turn looks like this (`website/docs/developer-guide/trajectory-format.md:79-108`):

```json
{ "from": "system", "value": "<the prompt above, with <tools>[…]</tools> filled in>" }
{ "from": "human",  "value": "What Python version is installed?" }
{ "from": "gpt",    "value":
   "<think>\nThe user wants to know the Python version. I should run python3 --version.\n</think>\n"
   "<tool_call>\n{\"name\": \"terminal\", \"arguments\": {\"command\": \"python3 --version\"}}\n</tool_call>" }
{ "from": "tool",   "value":
   "<tool_response>\n{\"tool_call_id\": \"call_abc123\", \"name\": \"terminal\", \"content\": \"Python 3.11.6\"}\n</tool_response>" }
{ "from": "gpt",    "value": "<think>\nGot the version. I can now answer the user.\n</think>\nPython 3.11.6 is installed on this system." }
```

So the wire format the model sees is:
- **Tools advertised** inside `<tools>…</tools>` as a JSON array of `{name, description, parameters, required}` objects.
- **Reasoning** wrapped in `<think>…</think>` (training data normalizes every assistant turn to have one, even if empty — see `run_agent.py:3531-3534`).
- **Tool calls** wrapped in `<tool_call>…</tool_call>`, each containing a JSON `{"name": ..., "arguments": ...}` object. Multiple calls in one turn = multiple `<tool_call>` blocks.
- **Tool results** wrapped in `<tool_response>…</tool_response>`, each containing a JSON `{"tool_call_id", "name", "content"}` object.

### Runtime-side: the OpenAI-style structured form

Internally, `run_agent.py` keeps `messages` in OpenAI Chat Completions format (see `AGENTS.md:131-134` and the architecture doc):

```python
{"role": "system", "content": "..."}
{"role": "user", "content": "..."}
{"role": "assistant", "content": "...", "tool_calls": [{"id": "call_...", "type": "function",
                                                         "function": {"name": "...", "arguments": "<json string>"}}]}
{"role": "tool", "tool_call_id": "call_...", "content": "..."}
```

When the provider returns `response.tool_calls` natively, the loop appends them as-is. When the provider returns text containing `<tool_call>` XML (e.g. local vLLM Hermes2Pro, Ollama-hosted models without OpenAI-tool-calls support), `environments/agent_loop.py:268-289` invokes the fallback parser:

```python
if (not assistant_msg.tool_calls and assistant_msg.content
        and self.tool_schemas and "<tool_call>" in (assistant_msg.content or "")):
    fallback_parser = get_parser("hermes")
    parsed_content, parsed_calls = fallback_parser.parse(assistant_msg.content)
    if parsed_calls:
        assistant_msg.tool_calls = parsed_calls
        if parsed_content is not None:
            assistant_msg.content = parsed_content
```

The parser itself, `environments/tool_call_parsers/hermes_parser.py:30-72`:

```python
PATTERN = re.compile(
    r"<tool_call>\s*(.*?)\s*</tool_call>|<tool_call>\s*(.*)", re.DOTALL
)

def parse(self, text: str) -> ParseResult:
    if "<tool_call>" not in text:
        return text, None
    matches = self.PATTERN.findall(text)
    tool_calls: List[ChatCompletionMessageToolCall] = []
    for match in matches:
        raw_json = match[0] if match[0] else match[1]
        if not raw_json.strip():
            continue
        tc_data = json.loads(raw_json)
        if "name" not in tc_data:
            continue
        tool_calls.append(ChatCompletionMessageToolCall(
            id=f"call_{uuid.uuid4().hex[:8]}",
            type="function",
            function=Function(
                name=tc_data["name"],
                arguments=json.dumps(tc_data.get("arguments", {}), ensure_ascii=False),
            ),
        ))
    content = text[: text.find("<tool_call>")].strip()
    return content if content else None, tool_calls
```

Two details worth noting:
- The regex matches both closed `<tool_call>…</tool_call>` and unclosed `<tool_call>…<EOS>`. Truncated generations during streaming or hitting context limits don't drop the call — the harness recovers it.
- A new `id` is minted for parsed calls. Tool-result correlation across turns relies on positional ordering of `tool_calls` against subsequent `tool` messages (see how trajectory normalization at `run_agent.py:3559-3567` matches by index).

### Dispatch

`model_tools.handle_function_call()` (`model_tools.py:494-641`) is the only entry point. Key behaviors:

- `coerce_tool_args()` (`model_tools.py:382`) — repairs LLM-style type errors before dispatch (`"42"` → `42`, `"true"` → `true`) by matching against the tool's JSON Schema. Saves a retry round-trip.
- `_AGENT_LOOP_TOOLS = {"todo", "memory", "session_search", "delegate_task"}` is intercepted by the agent loop *before* reaching `handle_function_call` because they need agent-state (the registry returns a stub error if one slips through).
- `pre_tool_call` / `post_tool_call` / `transform_tool_result` plugin hooks fire around dispatch. `pre_tool_call` can return a block message that short-circuits execution.
- Every handler returns a JSON string. That string becomes the `content` of the next `tool` message.
- Errors are wrapped: any exception inside a handler returns `json.dumps({"error": "..."})` rather than raising. The model sees a well-formed error and can recover.

Schemas are sanitized just before being sent to the API (`model_tools.py:357-359` calls `sanitize_tool_schemas`) for compatibility with llama.cpp's GBNF grammar generator, which rejects shapes cloud providers silently accept.

---

## 5. Reasoning / thinking handling

Hermes treats reasoning as **first-class but heterogeneous-source** content, normalizing everything to `<think>` tags at storage time and stripping them at display time.

Sources of reasoning:
1. Native thinking tokens from Anthropic / OpenAI o-series / etc., delivered on `assistant_msg["reasoning"]` (a separate API field).
2. Models trained to emit XML scratchpads (`<think>`, `<thinking>`, `<reasoning>`, `<REASONING_SCRATCHPAD>`, `<thought>` for Gemma 4).
3. Models that emit `<tool_call>` inside text rather than via structured tool-call APIs.

The trajectory writer at `run_agent.py:3500-3534` normalizes all three:

```python
if msg.get("reasoning") and msg["reasoning"].strip():
    content = f"<think>\n{msg['reasoning']}\n</think>\n"
if msg.get("content") and msg["content"].strip():
    content += convert_scratchpad_to_think(msg["content"]) + "\n"
...
if "<think>" not in content:
    content = "<think>\n</think>\n" + content
```

Every assistant turn in saved trajectories has a `<think>` block — empty if the model didn't think — so training data has a uniform shape.

Display is handled by `AIAgent._strip_think_blocks()` at `run_agent.py:2781-2873`. It handles four cases (case-insensitive across all variants):

```python
# 1. Closed tag pairs
content = re.sub(r'<think>.*?</think>', '', content, flags=re.DOTALL | re.IGNORECASE)
content = re.sub(r'<thinking>.*?</thinking>', '', content, ...)
content = re.sub(r'<reasoning>.*?</reasoning>', '', content, ...)
content = re.sub(r'<REASONING_SCRATCHPAD>.*?</REASONING_SCRATCHPAD>', '', content, ...)
content = re.sub(r'<thought>.*?</thought>', '', content, ...)

# 1b. Standalone tool-call XML blocks that some models emit inside content
for _tc_name in ("tool_call", "tool_calls", "tool_result", "function_call", "function_calls"):
    content = re.sub(rf'<{_tc_name}\b[^>]*>.*?</{_tc_name}>', '', content, ...)

# 2. Unterminated reasoning block (MiniMax M2.7 / NIM endpoints drop closers)
content = re.sub(
    r'(?:^|\n)[ \t]*<(?:think|thinking|reasoning|thought|REASONING_SCRATCHPAD)\b[^>]*>.*$',
    '', content, flags=re.DOTALL | re.IGNORECASE,
)
```

For multi-turn correctness, reasoning is **passed back to the API** on subsequent turns via `_copy_reasoning_content_for_api()` (`run_agent.py:9683`) so providers like Moonshot AI and OpenRouter, which use a separate `reasoning_content` field for cache continuity, see the prior thinking. The internal `messages` list keeps reasoning in `msg["reasoning"]`; the API copy moves it to `msg["reasoning_content"]` and strips the local-only field.

What I did not find: any "thinking budget" reservation logic (other than provider-config passthrough — `reasoning_config` with `effort` levels). The harness doesn't decide *when* to think; it forwards model-side configuration.

---

## 6. Communication & handoff

**Hermes Agent has no broker and no multi-role pipeline.** It is single-agent by default. Multi-step work happens through three mechanisms:

### (a) Single conversational loop with tool batching

The default. The model decides, calls one or many tools in a single response, sees results, and continues. `_PARALLEL_SAFE_TOOLS` (`run_agent.py:262-275`) is a frozenset of read-only tools (`read_file`, `search_files`, `web_search`, `web_extract`, `vision_analyze`, etc.) that can run concurrently in a `ThreadPoolExecutor` of up to 8 workers. `_NEVER_PARALLEL_TOOLS = {"clarify"}` forces sequential execution when any interactive tool is in the batch. Path-scoped tools (`read_file`, `write_file`, `patch`) can run concurrently when targeting independent paths.

### (b) `delegate_task` — subagent spawn

`tools/delegate_tool.py:1792-…`. A child `AIAgent` is constructed in `_build_child_agent()` (line 835) with:
- A fresh conversation (no parent history).
- Its own `task_id` (own terminal session, file-ops cache).
- A restricted toolset, intersected with the parent's enabled toolsets so a child cannot escalate.
- A focused system prompt built from the goal + context (the parent's SOUL.md and full skills index are *not* loaded — `skip_context_files=True`).
- Always-blocked tools: `delegate_task` (no recursive spawn unless `role='orchestrator'`), `clarify`, `memory`, `send_message`, `execute_code`. From `tools/delegate_tool.py:41-49`:

  ```python
  DELEGATE_BLOCKED_TOOLS = frozenset([
      "delegate_task",  # no recursive delegation
      "clarify",        # no user interaction
      "memory",         # no writes to shared MEMORY.md
      "send_message",   # no cross-platform side effects
      "execute_code",   # children should reason step-by-step, not write scripts
  ])
  ```

- Approval is handled with non-interactive callbacks (`_subagent_auto_deny` default, `_subagent_auto_approve` opt-in via `delegation.subagent_auto_approve`). This avoids deadlocking the parent's prompt_toolkit TUI when a child encounters a dangerous-command prompt.

The parent's context **only sees the delegation call and the summary result** — not the child's tool calls or reasoning. That's the key handoff property: the child returns a JSON object, and the parent's conversation gains one tool-result message.

Two roles, controlled by the `role` argument:
- `leaf` (default) — cannot further delegate.
- `orchestrator` — retains the `delegation` toolset and can spawn its own workers, bounded by `delegation.max_spawn_depth` (default 2). Even when called with `role='orchestrator'`, depth and a kill-switch (`is_spawn_paused()`) demote to `leaf` — the rule is single-point.

Children get an independent `IterationBudget` capped at `delegation.max_iterations` (default 50). Total iterations across parent + subagents can therefore exceed the parent's cap.

### (c) `execute_code` — Python sandbox with tool RPC

The README touts this: *"Write Python scripts that call tools via RPC, collapsing multi-step pipelines into zero-context-cost turns."* The model writes a Python program; the program calls Hermes tools (whichever are exposed by `SANDBOX_ALLOWED_TOOLS`); the program's stdout/stderr/return value comes back as one tool result. So a pipeline of 10 read_file + grep + parse calls becomes *one* turn in the conversation. That's a real, working response to "tool surface explosion eating context".

### Cross-process communication

- **ACP** (`acp_adapter/server.py`) — exposes Hermes as an editor-native agent over JSON-RPC stdio. This is for talking *to* Hermes from VS Code / Zed / JetBrains, not Hermes talking to other agents. The ACP registry entry is the whole `acp_registry/agent.json` (12 lines):
  ```json
  {"schema_version": 1, "name": "hermes-agent", ...
   "distribution": {"type": "command", "command": "hermes", "args": ["acp"]}}
  ```
- **Gateway** (`gateway/run.py`, ~9k LOC) — accepts messages from 18 platforms (Telegram/Discord/Slack/etc.), runs them through `AIAgent`, and delivers the response back. There's a single agent per session_key; the gateway doesn't fan out to multiple roles.

So the answer to "could Hermes Agent work in a multi-role pipeline?" is: not natively. There is no role abstraction (orchestrator/coder/validator). The only multi-instance primitive is `delegate_task`, which is sequential-by-design and assumes the parent fully reasons about which child to call and what context to pass.

---

## 7. State, memory, context management

### Persistent state surfaces

| Surface | Path | Use |
|---------|------|-----|
| Sessions DB | `~/.hermes/sessions.db` (SQLite + FTS5) | Conversation history, system-prompt snapshot per session, lineage on compression |
| MEMORY.md | `~/.hermes/MEMORY.md` | Frozen at session start; injected into system prompt |
| USER.md | `~/.hermes/USER.md` | User profile (Honcho when active; static file otherwise) |
| SOUL.md | `~/.hermes/SOUL.md` | Optional personality file; replaces `DEFAULT_AGENT_IDENTITY` if present |
| Skills | `~/.hermes/skills/<category>/<name>/SKILL.md` | Procedural memory; auto-discovered |
| Snapshot | `~/.hermes/.skills_prompt_snapshot.json` | Cached skill-index manifest for cold start |

### System-prompt assembly (cached once per session)

From `prompt-assembly.md` and `agent/prompt_builder.py`, the cached prompt has 10 layers (in order):
1. Agent identity (SOUL.md, else `DEFAULT_AGENT_IDENTITY`).
2. Tool-aware behavior guidance (`MEMORY_GUIDANCE`, `SESSION_SEARCH_GUIDANCE`, `SKILLS_GUIDANCE`, `TOOL_USE_ENFORCEMENT_GUIDANCE`, `OPENAI_MODEL_EXECUTION_GUIDANCE`, `GOOGLE_MODEL_OPERATIONAL_GUIDANCE` — model-specific).
3. Honcho static block (when active).
4. Optional system message override.
5. Frozen MEMORY snapshot.
6. Frozen USER profile snapshot.
7. Skills index (`<available_skills>` block).
8. Context files (`AGENTS.md`, `.cursorrules`, `.cursor/rules/*.mdc`).
9. Timestamp + session ID.
10. Platform hint (one of ~18 platform strings — CLI, telegram, slack, discord, sms, cron, …).

The assembled prompt is **stored in SQLite** at `session_db.update_system_prompt(session_id, prompt)` after build (`run_agent.py:9367-9372`) and reloaded verbatim on continuation (`run_agent.py:9335-9348`). Rebuilding would pick up disk changes and break the Anthropic prefix cache — so the harness explicitly does not rebuild.

### Ephemeral injection (per-API-call only)

- Memory-manager prefetch result, fenced with `<memory-context>…</memory-context>` tags and a system-note disclaimer (see `agent/memory_manager.py:66-80`), appended to the **current turn's user message** — not the system prompt. This is the cache-stability rule in action.
- `pre_llm_call` plugin-hook context — same target.
- Ephemeral system prompt (e.g. budget warnings) appended to the cached system prompt only at API-call time.

### Compression

Two triggers (`agent/context_compressor.py`, see also `architecture.md:201-213`):
1. **Preflight** — at the top of `run_conversation`, if `estimate_request_tokens_rough` ≥ 50% of model context window. Up to 3 passes.
2. **Gateway auto-compression** — between turns at 85%, more aggressive.

Algorithm:
- Protect first N (usually system + first user/assistant pair) and last N (default 20) messages.
- Tool call/result message *pairs* are kept together — never split.
- Middle turns are passed to an auxiliary cheap-fast LLM with a structured summary template that tracks Resolved/Pending questions, "Active Task", and "Remaining Work" (renamed from "Next Steps" to avoid the model reading it as new instructions). The summary preamble is `[CONTEXT COMPACTION — REFERENCE ONLY]` and explicitly tells the model *not* to re-answer summarized requests.
- A new session lineage ID is generated; compression creates a "child" session in the DB.

This is the **only** sanctioned way to mutate the cached prompt mid-conversation.

### Memory write discipline (anti-noise)

`MEMORY_GUIDANCE` in `agent/prompt_builder.py:144-162` is unusually prescriptive — and the prescriptions match Handshake's pain points:

> "Prioritize what reduces future user steering — the most valuable memory is one that prevents the user from having to correct or remind you again."
>
> "Do NOT save task progress, session outcomes, completed-work logs, or temporary TODO state to memory; use session_search to recall those from past transcripts."
>
> "Write memories as declarative facts, not instructions to yourself. 'User prefers concise responses' ✓ — 'Always respond concisely' ✗. […] Imperative phrasing gets re-read as a directive in later sessions and can cause repeated work or override the user's current request."

Memory and skill nudges are turn-counted (`_memory_nudge_interval`, `_skill_nudge_interval` default 10). When the threshold hits, a **background review** is spawned *after* the user response is delivered (`run_agent.py:12490-12515`), so it never competes with the user's task for model attention.

---

## 8. Risk mitigation

`run_agent.py` is unusually defensive. A non-exhaustive list of guards I found:

| Risk | Mitigation | Where |
|------|-----------|-------|
| Bad tool calls (wrong types) | `coerce_tool_args()` repairs string→int/bool/float against schema | `model_tools.py:382` |
| Hallucinated tool names (cross-tool refs in descriptions) | `available_tool_names` filter; `browser_navigate` dynamic description rewrite | `model_tools.py:277-339` |
| Hallucinated tools mentioned in schema descriptions | `AGENTS.md:625` rule — never reference other tools by name in schema descriptions; do it dynamically in `get_tool_definitions()` | rule + code |
| Infinite loops | `IterationBudget` (default 90), with a one-turn `_budget_grace_call` for graceful exit | `run_agent.py:214-256, 9530, 9551` |
| Empty / malformed JSON arguments | `_repair_tool_call_arguments()` and a per-turn retry counter (`_invalid_json_retries`, `_invalid_tool_retries`) | `run_agent.py:9647-9657, 9762-9774` |
| Surrogate characters from copy-paste / Ollama | `_sanitize_surrogates`, `_sanitize_messages_surrogates` | `run_agent.py:9206, 9778` |
| Truncated `<think>` blocks (MiniMax/NIM drop closers) | Boundary-gated regex strips unterminated blocks from start-of-line | `run_agent.py:2850-2855` |
| Stale TCP connections after provider outages | `_cleanup_dead_connections()` runs preflight | `run_agent.py:9250-9259` |
| Orphan tool messages / missing tool results in history | `_sanitize_api_messages()` adds stubs / strips orphans every API call | `run_agent.py:9743` |
| Subagent dangerous-command prompts deadlocking parent TUI | Worker-thread-installed callback; default `_subagent_auto_deny` | `tools/delegate_tool.py:69-93` |
| Recursive delegation / spawn explosion | `delegation.max_spawn_depth` (default 2), `is_spawn_paused()` kill-switch via TUI, `max_concurrent_children` cap | `tools/delegate_tool.py:1820-1846, 1875-1884` |
| Prompt injection from project context files | `_scan_context_content()` checks ~10 patterns (ignore-prior-instructions, hidden divs, exfil curl, secret reads) before injection | `agent/prompt_builder.py:36-73` |
| Cache poisoning by mid-conversation prompt mutation | Policy in `AGENTS.md:521-535`; `_cached_system_prompt` only invalidated on compression; slash commands default to "next session" | rule + code |
| Codex Responses API leaking unsupported fields to strict providers | `_should_sanitize_tool_calls()` / `_sanitize_tool_calls_for_strict_api()` | `run_agent.py:9698-9699` |
| llama.cpp grammar rejecting MCP-server schemas | `tools/schema_sanitizer.py` runs at every `get_tool_definitions()` call | `model_tools.py:357-361` |
| User interruption mid-API-call | `_interruptible_api_call` runs HTTP in a thread with cancellable Event | `run_agent.py` and architecture doc |
| Out-of-band user steering | `/steer` queues into `_pending_steer`, drained pre-API-call onto the latest tool message | `run_agent.py:9591-9639` |

Two patterns stand out for Handshake: (i) **every error path in tool dispatch returns a JSON-shaped error to the model rather than raising** — the model sees a sensible message and recovers, the conversation never aborts; (ii) **compatibility shims are silent and exhaustive**, so the same agent code runs unchanged across cloud and local providers.

---

## 9. Clever engineering

### Hermes function-calling template as universal wire format

The same `<tool_call>` XML format is used in (i) the SFT training data for Hermes 2 Pro and successors, (ii) the saved trajectory format that `trajectory_compressor.py` produces for further training, (iii) the runtime fallback parser when providers don't expose structured tool-calls, and (iv) the chat templates shipped with vLLM (`Hermes2ProToolParser`) and llama.cpp. One format, full lifecycle. This is the strongest "model+harness coupling" payoff in the repo.

### `execute_code` as a context-collapsing mechanism

Multi-step pipelines turn into one Python script run in a sandbox. The sandbox can call back into the model's tool registry. This sidesteps the worst case of agentic loops where 30 tool calls eat 100k tokens of context. The Hermes README is explicit: *"Write Python scripts that call tools via RPC, collapsing multi-step pipelines into zero-context-cost turns."*

### Cache-stability is a hard rule, not a goal

`AGENTS.md:521-535`:
> "Hermes-Agent ensures caching remains valid throughout a conversation. **Do NOT implement changes that would:** Alter past context mid-conversation, Change toolsets mid-conversation, Reload memories or rebuild system prompts mid-conversation. […] Slash commands that mutate system-prompt state (skills, tools, memory, etc.) must be **cache-aware**: default to deferred invalidation (change takes effect next session), with an opt-in `--now` flag for immediate invalidation."

This is the architectural choice that makes Anthropic prompt caching pay off. From `run_agent.py:9725-9737`, prefix caching breakpoints are auto-injected on Claude models (system + last 3 messages) for ~75% input-token cost reduction.

### Bit-perfect prefix normalization for local KV-cache reuse

`run_agent.py:9745-9776` strips trailing whitespace from string content and re-serializes tool-call argument JSON with `separators=(",", ":")` and `sort_keys=True` before sending. The comment:

> "Ensures bit-perfect prefixes across turns, which enables KV cache reuse on local inference servers (llama.cpp, vLLM, Ollama) and improves cache hit rates for cloud providers."

This is the kind of detail you only learn by losing money on cache misses. The harness does it preemptively.

### Cache-stable ephemeral context

Memory prefetch is injected into the **current turn's user message**, not the system prompt — preserving the cached prefix while still steering this turn. The injection is wrapped in `<memory-context>…</memory-context>` with a system-note disclaimer telling the model to treat the content as informational only, not as the user's actual words. `agent/memory_manager.py:66-80`.

### Self-improving skill loop

After completing a complex task (≥10 tool iterations by default), the agent gets a *background nudge* — spawned after the user response, so it doesn't compete for context — to consider creating or patching a skill. `SKILLS_GUIDANCE` in `prompt_builder.py:170-177` explicitly says: *"When using a skill and finding it outdated, incomplete, or wrong, patch it immediately […] Skills that aren't maintained become liabilities."* This is procedural memory with an in-band maintenance loop.

### Fallback parser handles truncation gracefully

The Hermes parser regex `<tool_call>\s*(.*?)\s*</tool_call>|<tool_call>\s*(.*)` matches both closed and unclosed tags, so a streaming generation cut off at the EOS still recovers the tool call. Truncation isn't a fatal error — it's just a slightly noisier parse.

---

## 10. Token / cost behavior

### Iteration budget

Default `max_iterations=90` per top-level turn (`run_agent.py:844`). Subagents get an independent budget of 50 each. `IterationBudget.refund()` exists specifically so `execute_code` turns don't eat the budget — programmatic tool use is free.

### Context window detection & compression

`agent/model_metadata.py` has a context-length lookup keyed on lower-cased model name; `MINIMUM_CONTEXT_LENGTH` is the floor. Compression triggers at 50% (preflight) / 85% (gateway). `_SUMMARY_RATIO = 0.20` of compressed content is allocated as summary budget, with an absolute ceiling of 12k tokens. Tool outputs are pre-pruned (replaced with `[Old tool output cleared to save context space]`) before LLM summarization to cut the cost of summarizing.

### Schema sanitization

`tools/schema_sanitizer.py` runs at every `get_tool_definitions()` call. With 47 tools and many MCP-imported tools, a few KB per tool can add up to 20–30k tokens of tool schemas alone — `agent/model_metadata.estimate_request_tokens_rough()` includes tool schemas in its preflight token estimate (`run_agent.py:9388-9394`). This is one of the few harnesses I've seen that actually counts tool tokens in its compression decisions.

### Tool-surface gating

`available_tool_names` is rebuilt on every `get_tool_definitions()` call from `check_fn` results. Tools that don't have their API key configured or whose toolset is disabled are simply absent from the schema list. Cross-references in static descriptions are scrubbed dynamically (`browser_navigate` description has `"prefer web_search or web_extract"` removed when those tools aren't available — `model_tools.py:325-339`). Result: the model never sees a tool name it can't actually invoke.

### Reasoning passthrough cost

For models with native thinking (Anthropic, o-series, etc.), reasoning content is preserved across turns in `msg["reasoning"]` and copied to `reasoning_content` for the API call. This costs tokens — but is often required for cache continuity (Moonshot AI explicitly demands it), and the harness does it conditionally via `_copy_reasoning_content_for_api()`.

### Anthropic prompt caching

Auto-detected on Claude models on Anthropic, OpenRouter, and third-party Anthropic gateways. `apply_anthropic_cache_control()` injects `cache_control` breakpoints at system + last 3 messages. Up to 75% input-token cost reduction on multi-turn conversations.

---

## 11. Comparison to ACP-style brokers

Hermes Agent **does** use ACP — but in the opposite direction from your usage.

In Handshake, ACP is the broker through which orchestrator → coder → validator role-handoffs flow; the broker is the connective tissue of a multi-agent pipeline.

In Hermes, ACP is the **server protocol that exposes a single Hermes Agent to external editors** (VS Code, Zed, JetBrains). The agent is the agent; ACP is just the IDE-integration layer. `acp_adapter/server.py` translates `prompt.submit` / `tool.start` / `approval.request` JSON-RPC calls into method calls on `AIAgent`. There is no second agent on the other side of the protocol.

So could Hermes Agent work in a multi-role pipeline? Not without significant reshaping. Concrete gaps:
- **No role abstraction.** `AIAgent` parameterizes via `enabled_toolsets`, `system_message`, and `prefill_messages`, but there's no concept of "this agent's job is validation, route X to it." Roles would have to be constructed by tooling at the layer above.
- **No artifact protocol between agents.** The only inter-agent surface is `delegate_task` returning a JSON dict to the parent. There is no notion of "produce a packet, hand it off, validator inspects it." A child returns text/JSON; the parent reads it; that's it.
- **No work-queue / mailbox / event-bus.** Sessions are independent SQLite rows. The gateway routes by session_key, not by role.

What Hermes does instead, and what's interesting: it pushes everything into **one conversation per task**, leans on `delegate_task` for *bounded* parallel sub-tasks, and uses `execute_code` to collapse procedural pipelines. The "communication overhead" in your mental model essentially does not exist — there is nothing to communicate, because there is one agent.

That said, you could imagine a Handshake-style layout where each role is a separate Hermes profile (`hermes -p coder`, `hermes -p validator`) with its own MEMORY/skills, and a thin orchestrator script dispatches work to them via the gateway/webhook. The infrastructure for this exists (profiles are isolated, gateway accepts webhook payloads), but Nous have not built it that way and have no in-repo abstraction for it.

---

## 12. Lessons for Handshake

**The single most transferable lesson is the cache-stability rule, codified.** Hermes' AGENTS.md elevates "do not mutate the system prompt mid-conversation" from a recommendation to a hard policy with reviewer-enforced patterns: slash commands default to deferred invalidation with an opt-in `--now` flag; memory updates land in disk MEMORY.md but the in-memory cached prompt only refreshes on next session; the *only* sanctioned mid-conversation mutation is compression, which deliberately rebuilds and accepts the cache loss. This is the architectural change that makes prompt caching, KV-cache reuse, and predictable token costs *possible*. If Handshake's governance docs (receipts, packets, dossiers) are part of the system prompt or the conversation history, every doc repair costs you a cache miss — possibly 4× the input-token bill on each turn that follows. Treating governance artifacts as **session-DB rows** queryable via a tool (Hermes' analog: `session_search`) rather than as in-prompt context would let you keep a stable cached prefix and still get the audit trail.

**The model+harness coupling reduces friction by eliminating handoff contract validation.** Hermes models are trained to emit `<tool_call>` JSON; the harness is trained to parse it; vLLM ships a compatible parser; the trajectory format reproduces it for the next training run. Nothing in the loop is reformatting, validating, or repairing handoff documents — there are no handoff documents. A turn is a list of OpenAI-format messages, full stop. The cost saving versus a bureaucratic broker is enormous and entirely from *not having a translation layer between agents*. The Handshake equivalent would be: define one canonical message format that *every role* speaks (orchestrator, coder, validator), shape it like OpenAI tool calls, and remove every "produce a receipt in this shape" step. The receipt is a tool result; the validator's job is a tool call against the coder-produced artifact; the orchestrator never asks "did you write the doc correctly?" because there is no doc to write — only structured tool-result content.

**Three concrete patterns Handshake could adopt with minimal disruption.** (1) `coerce_tool_args()` — a 30-line function that fixes LLM type errors against the schema *before* dispatch saves a full retry round-trip every time the validator says `"42"` instead of `42`. (2) Background-spawned governance review — Hermes' skill/memory nudge spawns a separate review *after* the user response is delivered, never during the user's task. If your governance update steps must happen, do them in the cooldown after the work is done, not before the next packet. (3) The `<memory-context>` fenced injection pattern — when you must inject governance state into a turn, inject it into the *user message* with a clear "this is informational, not user input" marker, never into the system prompt. Cache stays warm; the model still sees the context. These are tactical, but each one removes a token-burn loop.

---

## 13. Source list

Local clone (read in source):
- `harnesses/hermes-agent\AGENTS.md` (development guide; the primary operator-facing doc)
- `harnesses/hermes-agent\README.md`
- `harnesses/hermes-agent\run_agent.py` (specifically lines 214-256, 2781-2873, 3450-3580, 9161-9800, 12485-12540)
- `harnesses/hermes-agent\model_tools.py` (lines 203-641)
- `harnesses/hermes-agent\toolsets.py` (lines 1-130)
- `harnesses/hermes-agent\agent\prompt_builder.py` (lines 1-282, 134-192 for guidance text)
- `harnesses/hermes-agent\agent\memory_manager.py` (lines 1-80)
- `harnesses/hermes-agent\agent\context_compressor.py` (lines 1-80)
- `harnesses/hermes-agent\tools\delegate_tool.py` (lines 1-200, 835-1040, 1792-1910)
- `harnesses/hermes-agent\environments\tool_call_parsers\hermes_parser.py`
- `harnesses/hermes-agent\environments\agent_loop.py` (lines 240-360)
- `harnesses/hermes-agent\acp_adapter\server.py` (lines 1-80)
- `harnesses/hermes-agent\acp_registry\agent.json`

Local docs (read as documentation, not source-of-truth):
- `website/docs/developer-guide/architecture.md`
- `website/docs/developer-guide/agent-loop.md`
- `website/docs/developer-guide/prompt-assembly.md`
- `website/docs/developer-guide/trajectory-format.md` (lines 60-180; canonical format example)
- `website/docs/developer-guide/context-compression-and-caching.md`
- `hermes-already-has-routines.md` (Nous' positioning vs. Anthropic's Claude Code Routines)

External:
- WebFetch of `https://github.com/NousResearch/Hermes-Function-Calling` README — confirms the canonical system-prompt template and `<tool_call>` / `<tool_response>` XML format match what `run_agent.py:3463-3475` emits.

What I did *not* read but would for a deeper pass: the website docs for `tools-runtime.md` (more on tool registry), `session-storage.md` (FTS5 schema details), `gateway-internals.md` (long-running adapter patterns), and any Nous Research blog post specifically on Hermes 4 or the Hermes Function-Calling SFT methodology. I also did not run the harness or trace a live conversation — all behavioral claims are from reading the source.

---

## NOTES FOR SYNTHESIZER

- **Model+harness coupling is the headline.** Hermes models are trained to emit `<tool_call>{"name":...,"arguments":{...}}</tool_call>` and `<think>...</think>`; the harness has a regex parser that recovers structured calls from text when providers don't expose `tool_calls` natively. Same XML format spans training data → runtime parsing → trajectory storage → next training run. One format, full lifecycle. Quoted system-prompt template at `run_agent.py:3463-3475` and parser at `environments/tool_call_parsers/hermes_parser.py:30-72`.
- **Tool-call protocol internally is just OpenAI Chat Completions format.** `messages` is `[{role, content, tool_calls?}, ...]`. No custom schema, no doc shapes, no handoff packets. A "turn" is a message list. This is the strongest possible counter to artifact-repair burn.
- **Cache stability is policy.** AGENTS.md forbids mid-conversation system-prompt mutation. Slash commands default to "next session." The `_cached_system_prompt` only invalidates on context compression. This is what makes prompt caching pay off.
- **No multi-agent broker. Single agent + `delegate_task` (depth-2 default) + `execute_code` (Python sandbox calling tools by RPC).** Children return one JSON tool result; parent context never sees their reasoning. No handoff bureaucracy because there is no handoff document.
- **Governance-relevant guidance is in `MEMORY_GUIDANCE` (`prompt_builder.py:144-162`)**: don't write task progress to memory; use session_search instead. Write declarative facts, not imperatives. Imperative phrasing gets re-read as a directive next session and causes repeated work.
- **Two patterns most worth borrowing:** (1) inject ephemeral context (memory prefetch, plugin hints) into the **user message** with a `<memory-context>…</memory-context>` fence and system-note disclaimer — never the system prompt; (2) `coerce_tool_args()` repairs LLM type errors before dispatch, eliminating a class of retry round-trips entirely.
- **Risk mitigation pattern:** every error in tool dispatch returns a JSON error object to the model. The model recovers; the conversation never aborts. Iteration budgets, depth caps, kill-switches, and a one-turn grace call prevent runaway loops without aggressive aborts.
- **ACP in Hermes is opposite of Handshake's usage.** Hermes uses ACP as a *server* exposing one agent to editors. There is no "ACP between roles" — that role abstraction does not exist in Hermes.
- **`execute_code` is the answer to tool-surface explosion.** A Python sandbox that calls tools by RPC turns 10-tool pipelines into one tool result. Designed exactly for the "context burn from many small tool calls" failure mode.
