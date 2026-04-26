# 03 — OpenClaw and ACPX

## 1. Executive summary

OpenClaw is a personal AI assistant that runs on your own devices and answers you on the messaging channels you already use — WhatsApp, Telegram, Slack, Discord, Signal, iMessage, and roughly twenty more. The product sits behind a single local "Gateway" daemon that owns sessions, channels, tools, and events; a constellation of channel adapters, mobile apps, a macOS menu bar app, voice/canvas surfaces, and a CLI all talk to that Gateway. The lobster mascot ("EXFOLIATE! EXFOLIATE!") is a Doctor Who joke; the engineering underneath is much more sober. OpenClaw's tagline — "the AI that actually does things" — describes a single-user assistant whose code-and-tool loop is roughly Codex-shaped, but whose channel and identity model is closer to a personal IRC bouncer than to a coding harness.

The reason OpenClaw is interesting for Handshake is not the channel matrix; it is the *sister* project, **acpx** — a small, headless CLI client for the Agent Client Protocol (ACP), the same protocol Handshake's broker speaks. Acpx exists for exactly the use-case Handshake has: one agent talking to another agent, programmatically, without scraping a PTY. Acpx is the closest production-quality reference implementation of "what a clean ACP client process should look like" that exists outside the protocol's own SDK examples. Its stated goal in `VISION.md` is "the smallest useful ACP client … not a giant orchestration layer". That self-restraint is the lesson.

This document focuses on acpx for the deep technical detail, with the OpenClaw core, SOUL.md, the channel layer, and the wider project family (alphaclaw, openclaw-harness, awesome-openclaw-agents) covered as context. The point is to understand how OpenClaw separates "stable structured protocol traffic" from "model-shaped reasoning work", and from that, to extract why Handshake's artifact-repair tax exists and where it could be cut.

## 2. Project family

The OpenClaw ecosystem fans out across several repos. They are intentionally separated by responsibility rather than collapsed into a single product.

- **openclaw/openclaw** — the core. A pnpm monorepo with `src/`, `ui/`, `apps/`, `packages/`, `extensions/`, `docs/`. Contains the Gateway, channel adapters, tools, the agent loop, the macOS/iOS/Android apps, the canvas, and the `openclaw acp` bridge. Node 24 / 22.14+ runtime. Roughly Codex-class scope.
- **openclaw/acpx** — a separate, much smaller TypeScript package published to npm. ACP CLI client. ~30 source files in `src/`, with `acp/`, `cli/session/`, `cli/queue/`, `flows/`, `runtime/`. Built on `@agentclientprotocol/sdk`. Node 22.12+. The repo has its own VISION.md, AGENTS.md, SKILL.md, and ACP coverage roadmap.
- **chrysb/alphaclaw** — third-party "setup harness for OpenClaw". A non-invasive wrapper that gives you a browser-based setup UI, a watchdog with auto-restart and notifications, multi-agent dashboards, hourly git auto-commits for audit, and a docker-first deploy story. Deliberately documented as "wraps OpenClaw, it's not a dependency". You can remove it without breaking the agent.
- **sparkishy/openclaw-harness** — a security-focused harness around OpenClaw (the link returns 404 today; the project is referenced from awesome-openclaw lists and from OpenClaw's own sandbox documentation as a path that bolts stricter destructive-tool gating onto the core). Conceptually it sits between "trust everything because it's your machine" (OpenClaw default) and "trust nothing" (sandboxed mode).
- **mergisi/awesome-openclaw-agents** — a curated list of 162+ (now 205+) ready-to-paste `SOUL.md` agent personalities across categories (productivity, dev, marketing, business, devops, healthcare, etc.). This is the community surface that proves SOUL.md is the actual customisation interface for OpenClaw, not the source code or system prompt.

The split matters: openclaw is the product, acpx is the protocol client, alphaclaw is ops/wrapping, openclaw-harness is safety, awesome-openclaw-agents is content. Each lives in a different repo with a different governance model.

## 3. Architecture of openclaw core

Layout from `openclaw/AGENTS.md`:

> Core TS: `src/`, `ui/`, `packages/`; plugins: `extensions/`; SDK: `src/plugin-sdk/*`; channels: `src/channels/*`; loader: `src/plugins/*`; protocol: `src/gateway/protocol/*`; docs/apps: `docs/`, `apps/`, `Swabble/`.

The runtime is a Gateway process (single control plane on `ws://127.0.0.1:18789` by default) plus connected clients. Tools are conventional: `bash`, `process`, `read`, `write`, `edit`, `browser`, `canvas`, `nodes`, `cron`, `discord`, `gateway`, `sessions_list`, `sessions_history`, `sessions_send`, `sessions_spawn`. Models are pluggable per agent through `auth-profiles` rotation (see `src/agents/auth-profiles.ts`) with cooldown, failover, round-robin and per-provider quirks. Subscription OAuth (e.g. ChatGPT/Codex) is a first-class auth profile, not a side hack.

The agent loop itself is intentionally generic. From the AGENTS.md design rule: "core stays extension-agnostic. No bundled ids in core when manifest/registry/capability contracts work … core owns generic loop; provider plugins own auth/catalog/runtime hooks". Channels are explicitly *implementation* — only seams are exposed to plugins. New providers ship as plugin packages, not core merges. That is much closer to Pi's architecture than to Codex's monolith — though Codex got more disciplined about this in 2026.

Sessions are persisted on disk, scoped by an "agent-scoped session key" of form `agent:<agentId>:<sessionId>`, e.g. `agent:main:main`, `agent:design:main`, `agent:qa:bug-123`. The default session for a single user is just `main:main`. Multi-agent routing is real: inbound channels can be mapped to different agents with different workspaces and different sandbox modes.

Sandboxing is opt-in by configuration: `agents.defaults.sandbox.mode: "non-main"` runs every session that is *not* the user's primary `main` session inside a Docker (default), SSH, or OpenShell sandbox. This is the security trick — the assistant has full host access for the owner, but anything triggered from a group chat or unknown DM runs in a constrained world with `bash`/`process`/file tools allowed and `browser`/`canvas`/`discord`/`gateway` denied. The Dockerfiles `Dockerfile.sandbox`, `Dockerfile.sandbox-browser`, and `Dockerfile.sandbox-common` in the repo root back this up.

## 4. Channel multiplexing

OpenClaw supports an unusually wide channel matrix for a coding-shaped agent: WhatsApp, Telegram, Slack, Discord, Google Chat, Signal, iMessage, BlueBubbles, IRC, Microsoft Teams, Matrix, Feishu, LINE, Mattermost, Nextcloud Talk, Nostr, Synology Chat, Tlon, Twitch, Zalo, Zalo Personal, WeChat, QQ, WebChat, plus first-party macOS/iOS/Android nodes. The abstraction that enables this lives in `src/channels/*` and `src/gateway/protocol/*`.

Each channel adapter implements the same internal contract: inbound message normalised into a Gateway event, outbound text/edits/streaming controls translated into the channel's native API. The adapters do not see the agent loop — they emit and consume Gateway events. A *binding* (`src/acp/persistent-bindings.*`) maps a (channel, account, peer) triple to a Gateway session key, which is then routed to whichever agent owns that key.

This is the architectural payoff of the "session key as universal address" decision. The agent loop, the ACP bridge, the macOS menu bar, an inbound WhatsApp DM, a Discord slash command, and a Zed editor opening an ACP thread all converge on the same key namespace. There is exactly one rendering surface for a session at a time (the channel that owns it), but reads and tool side-effects all flow through the Gateway.

The DM safety story is also worth noting: the default `dmPolicy="pairing"` means an unknown sender does *not* trigger a model call — they get a short pairing code and the message is dropped until the operator runs `openclaw pairing approve <channel> <code>`. This is an example of OpenClaw using a deterministic non-model gate to refuse work, instead of relying on the agent to handle adversarial input correctly. Handshake's broker has nothing equivalent today.

## 5. SOUL.md system

`SOUL.md` is OpenClaw's per-agent personality file. From `docs/concepts/soul.md`:

> `SOUL.md` is where your agent's voice lives. OpenClaw injects it on normal sessions, so it has real weight. If your agent sounds bland, hedgy, or weirdly corporate, this is usually the file to fix.

Crucially, SOUL.md is **not** a system prompt and **not** a task packet. It is a small, stable, opinionated file that governs:

- tone
- opinions
- brevity
- humor
- boundaries
- default level of bluntness

It is explicitly *not*:

- a life story
- a changelog
- a security policy dump
- a giant wall of vibes with no behavioral effect

Operating rules go to `AGENTS.md` (or the per-tree `AGENTS.md` files OpenClaw uses everywhere). Personality goes to `SOUL.md`. Memory is the running session transcript and any memory-plugin database. The three concerns are separated by file, not by section header in one giant prompt.

The template at `docs/reference/templates/SOUL.md` is short — about 50 lines — and the canonical example consists of five "Core Truths" rules ("Be genuinely helpful, not performatively helpful", "Have opinions", "Be resourceful before asking", "Earn trust through competence", "Remember you're a guest"), a Boundaries block, a Vibe block, and a Continuity note. The community repo `awesome-openclaw-agents` exists *because* SOUL.md is the actual hot-swap interface — picking a SOUL.md is like picking a character class, not editing source.

Compare to a Handshake "Work Packet": Handshake packets are large, structured artifacts with receipts, dossiers, microtask contracts, validator outputs, and back-references. They are *both* identity (who is this role) *and* task spec (what is this WP) *and* state (what receipts have been produced). SOUL.md takes only the first of those three roles. The task spec lives in chat history; the state lives in the session store. OpenClaw refuses to fuse them.

## 6. ACPX deep dive

This is the core of the document. ACPX is the most direct comparator to Handshake's ACP usage.

### 6.1 Core idea

From `acpx/VISION.md`:

> `acpx` should be the smallest useful ACP client: a lightweight CLI that lets one agent talk to another agent through the Agent Client Protocol without PTY scraping or adapter-specific glue.

And from the README:

> Your agents love acpx! 🤖❤️ They hate having to scrape characters from a PTY session 😤

That is the *whole* design. Acpx is a CLI you can invoke from another agent (`acpx codex "fix the failing test"`) and what comes back is structured ACP traffic, not terminal junk.

### 6.2 Transport and wire format

The transport is **NDJSON over stdio**. The agent process is spawned as a normal child process. Stdin and stdout carry one JSON-RPC message per line. The implementation in `src/acp/client.ts:176-236` is worth reading verbatim:

```ts
function createNdJsonMessageStream(
  agentCommand: string,
  output: WritableStream<Uint8Array>,
  input: ReadableStream<Uint8Array>,
): {
  readable: ReadableStream<AnyMessage>;
  writable: WritableStream<AnyMessage>;
} {
  // ...
  const readable = new ReadableStream<AnyMessage>({
    async start(controller) {
      let content = "";
      // ...
      while (true) {
        const { value, done } = await reader.read();
        // ...
        content += textDecoder.decode(value, { stream: true });
        const lines = content.split("\n");
        content = lines.pop() || "";
        for (const line of lines) {
          const trimmedLine = line.trim();
          if (!trimmedLine || shouldIgnoreNonJsonAgentOutputLine(agentCommand, trimmedLine)) {
            continue;
          }
          try {
            const message = JSON.parse(trimmedLine) as AnyMessage;
            controller.enqueue(message);
          } catch (err) {
            console.error("Failed to parse JSON message:", trimmedLine, err);
          }
        }
      }
      // ...
    },
  });
  // ...
}
```
(`src/acp/client.ts:176-236`)

That's it. There is no framing protocol, no length prefix, no sidecar pipe. Each side just emits one JSON-RPC object per line. Banner lines that aren't JSON are silently filtered.

### 6.3 Message types implemented

From `docs/2026-02-19-acp-coverage-roadmap.md`, the supported ACP method set is:

- `initialize` — handshake, capability negotiation
- `session/new` — `sessions new`
- `session/load` — crash resume / reconnect
- `session/prompt` — `prompt`, `exec`, implicit prompt
- `session/update` — streaming output (thinking, tools, text, diffs)
- `session/cancel` — graceful cancel
- `session/request_permission` — `--approve-all`, `--approve-reads`, `--deny-all`
- `session/set_mode` — `acpx <agent> set-mode <mode>`
- `session/set_config_option` — `acpx <agent> set <key> <value>`
- `fs/read_text_file`, `fs/write_text_file` — client file handlers
- `terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release` — full terminal lifecycle handled by the *client*
- `authenticate` — auth handshake

Everything is structured. No tool result is a glob of stdout text — it is `tool_call` and `tool_call_update` events with named statuses, content blocks, and locations. Permissioning is RPC, not a heuristic over the prompt.

### 6.4 Session model

The session model is explicit in the README:

> Session state lives in `~/.acpx/` either way. Global install is a little faster, but `npx acpx@latest` works fine.

Each session has a *local* acpx record id (the persisted session in `~/.acpx/sessions/`) and optionally an *agent-side* session id (whatever the wrapped CLI assigns). They are surfaced separately in JSON output:

```json
{
  "eventVersion": 1,
  "sessionId": "abc123",
  "requestId": "req-42",
  "seq": 7,
  "stream": "prompt",
  "type": "tool_call"
}
```

The README warns explicitly: "the text/quiet session id is the local acpx record id; do not assume it can be passed to the native provider CLI unless `agentSessionId` is present." This dual-id model is what makes resume work across agent restarts — acpx owns the persistent identity, the underlying agent owns the runtime identity, and the bridge correlates them.

Sessions can be parallel and named: `acpx codex -s backend "..."` and `acpx codex -s frontend "..."` run side by side in the same repo. There is also explicit cwd-walking behaviour: prompts route by walking up to the nearest git root and selecting the nearest active session matching `(agent command, dir, optional name)`.

### 6.5 Statefulness without PTY scraping

The big architectural difference from terminal scraping is in `AcpClient.start()` (`src/acp/client.ts:405-573`). The client spawns the agent, opens a `ClientSideConnection` over the NDJSON stream, and sends `initialize` with negotiated capabilities:

```ts
const initResult = await Promise.race([
  (async () => {
    const initializePromise = connection.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: {
        fs: {
          readTextFile: true,
          writeTextFile: true,
        },
        terminal: this.options.terminal !== false,
      },
      clientInfo: {
        name: "acpx",
        version: "0.1.0",
      },
    });
    // ...
    await this.authenticateIfRequired(connection, initialized.authMethods ?? []);
    return initialized;
  })(),
  startupFailure.promise,
]);
```
(`src/acp/client.ts:523-547`)

State is preserved by the agent process owning its own model context plus acpx persisting the session record on disk. If the agent crashes, acpx reconnects: "If a saved session pid is dead on the next prompt, `acpx` respawns the agent, attempts `session/load`, and transparently falls back to `session/new` if loading fails." (README, Session behavior section.) The fallback is silent and atomic, but the contributor guide forbids using it in flows: "If the original persistent session cannot be resumed, fail the node or workflow clearly instead." (`AGENTS.md`.)

### 6.6 Queue ownership and warm sessions

Acpx splits the world into queue *owner* processes and queue *client* processes (see `src/cli/queue/ipc.ts`, `ipc-server.ts`, `lease-store.ts`). Exactly one owner exists per session at a time. New prompts coming through `acpx <agent> "..."` discover the owner via a lease file and submit a typed request:

```ts
export type QueueSubmitRequest = {
  type: "submit_prompt";
  requestId: string;
  ownerGeneration?: number;
  message: string;
  prompt?: PromptInput;
  permissionMode: PermissionMode;
  resumePolicy?: SessionResumePolicy;
  nonInteractivePermissions?: NonInteractivePermissionPolicy;
  timeoutMs?: number;
  suppressSdkConsoleErrors?: boolean;
  waitForCompletion: boolean;
  sessionOptions?: QueueSessionOptions;
};
```
(`src/cli/queue/messages.ts:24-37`)

The owner gets back typed messages too — `accepted`, `event`, `result`, `cancel_result`, `set_mode_result`, etc. (`messages.ts:77-120`) — instead of a stream of bytes. Generation numbers prevent stale-owner bugs after restarts.

The "warm session" architecture is documented as the target evolution in `docs/2026-02-25-warm-session-owner-architecture.md`: split caller and owner into two roles, owners stay alive in the background per session, callers exit immediately after their turn. That document explicitly cites Handshake-shaped problems: "thread message -> enqueue prompt -> stream output -> complete response — no hidden 300s wait in gateway-facing process paths".

### 6.7 Permission handling

Permissions are not text. `RequestPermissionRequest` comes in over the wire from the agent, and the client decides via `resolvePermissionRequest`:

```ts
private async handlePermissionRequest(
  params: RequestPermissionRequest,
): Promise<RequestPermissionResponse> {
  if (this.cancellingSessionIds.has(params.sessionId)) {
    return { outcome: { outcome: "cancelled" } };
  }
  let response: RequestPermissionResponse;
  try {
    response = await resolvePermissionRequest(
      params,
      this.options.permissionMode,
      this.options.nonInteractivePermissions ?? "deny",
    );
  } catch (error) {
    // ...
  }
  const decision = classifyPermissionDecision(params, response);
  this.recordPermissionDecision(decision);
  return response;
}
```
(`src/acp/client.ts:1180-1215`)

This is what "non-interactive permissions" means: the client decides on policy, not on a string-match against a prompt. The four modes are `--approve-all`, `--approve-reads` (default), `--deny-all`, and explicit per-call interactive prompting.

## 7. Communication and handoff model

In the OpenClaw + acpx world, the components talk like this:

1. **External orchestrator → acpx (CLI invocation).** The orchestrator runs `acpx codex 'fix the failing tests'` or `acpx --format json codex exec '...'`. With `--format json` the output is NDJSON of structured ACP events that any other agent can parse with `jq`. The "handoff" is a CLI invocation with stdin/stdout/exit-code semantics — no document needs to be in any particular shape, no receipt has to be written, no validator has to sign anything off. The structured stream *is* the receipt.
2. **Acpx → adapter.** Acpx spawns the adapter (e.g. `npx @zed-industries/codex-acp`) over stdio. NDJSON in, NDJSON out. Acpx is the ACP *client*; the adapter is the ACP *agent*.
3. **Acpx → openclaw bridge.** When the adapter is `openclaw acp` itself, you get a bridge process that is *also* an ACP agent on the upstream side, and a Gateway WebSocket *client* on the downstream side. The bridge translates `session/prompt` into Gateway `chat.send`, and Gateway streaming events back into ACP `session/update` notifications. See `docs.acp.md`:
   > ACP `prompt` translates to Gateway `chat.send`. Gateway streaming events are translated back into ACP streaming events. ACP `cancel` maps to Gateway `chat.abort` for the active run.
4. **Acpx → flow.** The newer `acpx flow run <file>` runs a TypeScript flow module: `acp` nodes do model-shaped work, `action` nodes do deterministic shell/GitHub work, `compute` nodes do local routing, `checkpoint` nodes pause for external events. The flow runtime owns graph traversal and persistence; the model never sees the workflow.

The handoff between these layers is *typed*. There is no document the model has to repair to make the handoff valid. There is no "did the validator sign the receipt?" loop. The contract is the JSON schema of ACP messages.

## 8. State, memory, long sessions

State lives in three places: `~/.acpx/sessions/` (acpx local records), the agent process's own working memory while running, and — for `openclaw acp` specifically — the OpenClaw Gateway's session store. Multiple ACP sessions can map to the same Gateway session key with `_meta.sessionKey` (or `--session agent:design:main`); each ACP session can be reset (`resetSession`/`--reset-session`) to mint a new transcript on the same key.

Resume semantics: on the next prompt, acpx checks if the saved pid is alive. If not, it respawns and calls `session/load`. If load fails, it falls back to `session/new`. The loaded transcript may not include tool-call history — `docs.acp.md` is honest about this:

> `loadSession` replays stored user and assistant text history, but it does not reconstruct historic tool calls, system notices, or richer ACP-native event types.

That's a real limitation, but it is bounded: the contract says "user/assistant text only" and the model can rely on it.

Soft-close vs hard-close: `sessions close` marks a record as closed but keeps it on disk for inspection (`acpx codex sessions history --limit <n>`). The history is "lightweight turn previews (`role`, `timestamp`, `textPreview`)" — not the entire transcript inflated. That matters for ops sanity.

Branching: `session/fork` exists in the ACP spec but is on the "Not Yet Supported" list as `unstable`. The roadmap notes it would "allow branching one conversation into parallel alternatives". Acpx is conservative about adopting unstable spec methods.

## 9. Risk mitigation and permissioning

OpenClaw splits responsibilities by sandbox mode:

- **Default (you're the only user).** Tools run on host. Full bash, full filesystem, full browser. The assumption is that `main:main` is the operator and operator already trusts the machine.
- **`agents.defaults.sandbox.mode: "non-main"`.** Anything triggered from a non-`main` session (group chat, unknown DM, secondary agent) runs inside Docker/SSH/OpenShell. Default sandbox allow-list: `bash`, `process`, `read`, `write`, `edit`, `sessions_list`, `sessions_history`, `sessions_send`, `sessions_spawn`. Default deny: `browser`, `canvas`, `nodes`, `cron`, `discord`, `gateway`. This is a *deterministic* trust gate, not a model-judged one.
- **DM pairing.** Default DM policy on Telegram/WhatsApp/Signal/iMessage/Microsoft Teams/Discord/Google Chat/Slack is `pairing`: unknown senders get a code, message is dropped. The model is not asked to be careful; it is simply not invoked.
- **acpx permission modes.** Independent of the above: `--approve-all` for trusted scripts, `--approve-reads` for default exploratory work, `--deny-all` for explainer-only runs, `--non-interactive-permissions fail|deny` for headless lanes. These are policy modes, not prompt heuristics.
- **openclaw-harness (third-party).** Adds stricter destructive-tool gating around the OpenClaw core. It is *not* in the main repo.
- **alphaclaw (third-party).** Adds watchdog auto-restart, audit-log auto-commits, multi-agent dashboards. It is *not* in the main repo.

The deliberate pattern: keep the dangerous decisions deterministic, keep the agent loop generic, push extra safety/ops/UX into satellite repos.

## 10. Clever engineering tricks

A few pieces worth lifting:

1. **`runConnectionRequest`** (`src/acp/client.ts:1275-1311`). Every outbound request is registered in a `pendingConnectionRequests` set. If the agent process dies mid-request, *all* pending requests are rejected with a typed `AgentDisconnectedError` carrying the disconnect reason. No ghost promises.

   ```ts
   private async runConnectionRequest<T>(run: () => Promise<T>): Promise<T> {
     return await new Promise<T>((resolve, reject) => {
       const pending: PendingConnectionRequest = { settled: false, reject };
       const finish = (cb: () => void) => {
         if (pending.settled) return;
         pending.settled = true;
         this.pendingConnectionRequests.delete(pending);
         cb();
       };
       this.pendingConnectionRequests.add(pending);
       void Promise.resolve()
         .then(run)
         .then(
           (value) => finish(() => resolve(value)),
           (error) => finish(() => reject(error)),
         );
     });
   }
   ```

2. **Three-stage agent shutdown** (`src/acp/client.ts:940-976`). End stdin (graceful), wait, SIGTERM (forceful), wait, SIGKILL (last resort). Each stage has a documented grace period (`stdinCloseGraceMs`, `AGENT_CLOSE_TERM_GRACE_MS = 1500`, `AGENT_CLOSE_KILL_GRACE_MS = 1000`). No orphans.

3. **Suppress the SDK's `Error handling request` console spam selectively** (`src/acp/client.ts:156-174`). The ACP SDK logs noisy errors that confuse human users; acpx wraps `console.error` for the duration of a prompt and restores it after.

4. **Replay drain semantics** for `session/load` (`src/acp/client.ts:1406-1436`). When loading a session, you may not want the replay's `session/update` notifications to be sent to the user-facing handler — they are history, not new turns. Acpx exposes `suppressReplayUpdates` and uses an idle-counter (`observedSessionUpdates` vs `processedSessionUpdates`) to detect drain instead of guessing with a fixed sleep.

5. **Generation-numbered queue ownership.** `assertOwnerGeneration` (`src/cli/queue/ipc.ts:89-101`) catches the case where a stale lease file points to an owner whose generation has advanced — an explicit `QUEUE_OWNER_GENERATION_MISMATCH` rather than a timeout.

6. **Doc ordering policy as governance.** The acpx `AGENTS.md` mandates an exact ordering (`pi`, `openclaw`, `codex`, `claude`, then others) for example sets in main landing docs and explicitly forbids harness-specific promotion in `README.md`. This is a meta-trick: codify "neutrality" so agents don't drift the docs to favor whoever wrote the last PR.

## 11. Comparison to Handshake's ACP usage

Both projects use the Agent Client Protocol over JSON-RPC. The structural differences are sharp.

- **Acpx is one process per session.** Acpx spawns the agent, owns the connection, persists the record, and dies. Handshake's broker is long-running and orchestrates many sessions; the model sessions are usually outside the broker. This is fine, but it means Handshake has more state to keep coherent.
- **Acpx has no "documents".** A turn produces ACP events, and that is the audit trail. Handshake has WP packets, receipts, dossiers, RGFs, intent snapshots, microtask contracts. The model has to author and update those *as part of the turn*, and the next role has to read them *before* its own turn. That work is not free.
- **Acpx serialises through a single owner.** One queue owner per session. No two roles trying to write to the same packet. Handshake's role-split (orchestrator, coder, validator, integrator) writes to overlapping artifacts; coordination cost lives in document shape.
- **Acpx's permissions are protocol-level.** `session/request_permission` is an RPC the agent sends and the client decides, with policy modes selectable from the CLI. Handshake's "should this tool run?" decision is often baked into prompt instructions, which the model has to remember and obey.
- **Acpx uses `session/cancel` correctly.** `Ctrl+C` sends ACP `session/cancel` and waits briefly for `stopReason=cancelled` before SIGKILL. The session record is preserved. Handshake's broker tends to tear sessions down on cancel, losing state.
- **Acpx accepts that the agent owns its memory.** The agent's process holds the model context. Acpx persists only what is needed to resume (lightweight previews, agent session id, cwd, agent command). Handshake persists *everything* in artifacts and then has to keep those artifacts in sync with the model's actual context — a constant repair tax.
- **Acpx's flows run *outside* the agent.** When you need multi-step work, you write a TypeScript flow with explicit `acp`/`action`/`compute`/`checkpoint` nodes. The model never sees the graph. Handshake's WP lifecycle currently lives partly in protocol (broker) and partly in artifacts the model has to read and re-author. The flow boundary is fuzzy.
- **Acpx is open about not implementing parts of ACP.** `session/fork`, `session/list`, `session/resume`, `session/set_model`, `$/cancel_request` are documented as "unstable, not yet supported". Handshake imports the protocol assumptions wholesale.

The headline divergence: Handshake uses ACP as a *transport*, but pays for handoffs with *artifact authoring*. Acpx uses ACP as both transport *and* contract — there is nothing else the next process needs in order to work.

## 12. Lessons for Handshake

**The artifact-repair tax in Handshake comes from putting state and policy in documents the model has to author.** Each WP packet, receipt, dossier, and RGF is a document the model writes during its turn. Each handoff requires the next role to read those documents and verify they are in the right shape, and to author its own. A coder's failed test run bleeds into the validator's receipt, which bleeds into the integrator's dossier, which bleeds into the orchestrator's verdict — and any malformed artifact in the chain blocks the chain. The model is doing two jobs: the work, and bookkeeping the work. ACPX shows you can keep ACP entirely *and* delete most of the bookkeeping by moving it into structured runtime data (`~/.acpx/sessions/*.json`, queue-owner messages, NDJSON event streams) that no model has to read or write to make a handoff valid. The session record, the permission stats, the tool-call locations, and the stop reasons are all the receipt the next role needs.

**Acpx-style ownership ends the "who edits this packet" problem.** Exactly one queue owner per session. Other callers submit typed `QueueSubmitRequest`/`QueueCancelRequest`/`QueueSetModeRequest` messages and get typed responses. Handshake could collapse most of its packet-update friction by adopting the same shape: one owner process per WP, every other role submits typed requests and consumes typed events. Roles would still differ in *intent* (orchestrator decides, coder writes code, validator checks), but the document-as-shared-spreadsheet model would go away. The orchestrator-no-coding rule and the per-MT validator pattern survive cleanly under this — they become "validator submits a `verify_microtask` request and gets a `verify_result` event", not "validator opens packet.md, finds the right section, edits the receipt".

**Use SOUL.md / AGENTS.md / CLAUDE.md separation, not packet stuffing.** OpenClaw keeps voice (SOUL.md), operating rules (AGENTS.md, scoped per subtree), and product code (`src/`) in separate files with separate change cadences. Handshake currently mixes role identity, work spec, status, receipts, and history into a single WP packet that everyone edits. Splitting those — role identity in a stable per-role file, work spec in a small immutable WP intent file, status as ACP events streamed to a structured log, receipts auto-derived from the event stream — would let each surface evolve at its own speed. The orchestrator could hot-swap a coder's identity without touching the WP intent. The validator could ingest the event log without re-reading every prior packet revision. The artifact-repair tax disappears because there is nothing structural to repair — the documents are either intent (rare writes) or projections of the event stream (auto-generated).

## 13. Source list

Local clones used:

- `harnesses/openclaw` — main repo.
- `harnesses/openclaw-acpx` — ACPX CLI client.

Files quoted or read:

- `openclaw/AGENTS.md` — root contributor rules and architecture map.
- `openclaw/README.md` — install, channels, security model, sandbox defaults.
- `openclaw/VISION.md` — project priorities and plugin philosophy.
- `openclaw/docs.acp.md` — ACP bridge spec, session mapping, prompt translation.
- `openclaw/docs/concepts/soul.md` — SOUL.md design philosophy.
- `openclaw/docs/reference/templates/SOUL.md` — canonical SOUL template.
- `openclaw/src/acp/server.ts` — bridge entry point.
- `openclaw/src/acp/translator.ts` — ACP-to-Gateway translation layer.
- `openclaw/src/acp/session-mapper.ts` — session key resolution and meta parsing.
- `openclaw-acpx/README.md` — feature surface, session model, output formats.
- `openclaw-acpx/AGENTS.md` — contributor and doc-ordering policy.
- `openclaw-acpx/VISION.md` — interoperability-first principles.
- `openclaw-acpx/skills/acpx/SKILL.md` — agent-facing usage guide.
- `openclaw-acpx/src/acp/client.ts` — ACP client core, NDJSON stream, lifecycle, permissions, drain logic.
- `openclaw-acpx/src/cli/queue/ipc.ts` and `messages.ts` — queue ownership and typed messages.
- `openclaw-acpx/src/cli/session/runtime.ts` — session lifecycle and queue-task output formatting.
- `openclaw-acpx/docs/2026-02-19-acp-coverage-roadmap.md` — implemented vs unsupported ACP methods.
- `openclaw-acpx/docs/2026-02-25-warm-session-owner-architecture.md` — target detached-owner topology.
- `openclaw-acpx/docs/2026-03-25-acpx-flows-architecture.md` — multi-step flow runtime.

External:

- [openclaw/openclaw on GitHub](https://github.com/openclaw/openclaw)
- [openclaw/acpx on GitHub](https://github.com/openclaw/acpx)
- [Agent Client Protocol spec](https://agentclientprotocol.com)
- [chrysb/alphaclaw on GitHub](https://github.com/chrysb/alphaclaw)
- [mergisi/awesome-openclaw-agents on GitHub](https://github.com/mergisi/awesome-openclaw-agents)
- [acpx on npm](https://www.npmjs.com/package/acpx)

## NOTES FOR SYNTHESIZER

- ACPX is the cleanest counterexample to "ACP requires document-shaped handoffs". It uses ACP for everything — transport, control, permission, handoff — and persists *only* a small session record plus the NDJSON event stream. No packet, no receipt, no dossier.
- Anti-pattern to call out in Handshake: state-in-documents. Acpx puts state in `~/.acpx/sessions/*.json` (machine-written) and in the agent process; nothing the model has to author makes a handoff valid.
- Anti-pattern: many writers per artifact. Acpx enforces one queue owner per session. Handshake's WP packet has many writers and that's where the repair tax lives.
- Anti-pattern: PTY scraping. Handshake uses ACP, so it dodged this one — but if the broker ever silently falls back to text scraping on bridge failures, that is the same failure mode the acpx README mocks.
- Pattern to lift: typed queue requests (`QueueSubmitRequest`, `QueueCancelRequest`, `QueueSetModeRequest`) instead of "edit the section of the markdown".
- Pattern to lift: SOUL.md / AGENTS.md split. Identity, operating rules, and work spec are different files with different change cadences — none of them is the receipt.
- Pattern to lift: deterministic non-model gates (DM pairing, sandbox mode by session key, permission modes). Don't ask the model to be careful; refuse the work upstream.
- The roadmap doc `2026-02-25-warm-session-owner-architecture.md` describes a Handshake-shaped problem ("hidden 300s wait in gateway-facing process paths") and solves it by making owner lifetime independent of caller lifetime. Worth flagging for the Handshake broker.
- Acpx is open about which ACP methods are unstable and unsupported (`session/fork`, `session/list`, `session/resume`, `session/set_model`, `$/cancel_request`). Handshake should similarly pin which protocol methods it depends on and refuse to drift.
- Compare to other harnesses: Acpx is much smaller than Codex's harness or Claude Code's runtime, by design. The lesson is *scope discipline* — Handshake's broker should resist becoming a workflow engine.
