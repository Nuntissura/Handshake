# 04 — Gastown (Gas Town)

> Deep technical dive into `github.com/steveyegge/gastown` (a.k.a. Gas Town), commit state of local clone at `harnesses/gastown` as of 2026-04-26.
>
> Author: research synthesis for Handshake harness study.

---

## 1. What is Gastown?

**Disambiguation.** "Gastown" by `gastownhall` redirects to the canonical repo `github.com/steveyegge/gastown` — this is **Steve Yegge's** multi-agent orchestration system, a sister project to his issue tracker `beads` (also referenced throughout). It is unambiguously an agent harness. From the README: *"Gas Town is a workspace manager that lets you coordinate multiple AI coding agents (Claude Code, GitHub Copilot, Codex, Gemini, and others) working on different tasks."*

**One-paragraph framing.** Gas Town is a Go CLI (`gt`) that wraps a town of long-running AI coding agents. Its central thesis is that *agents lose context on restart, so persist work state outside the agent in a git-backed structured ledger* (the `beads` system, backed by a Dolt SQL server). Rather than re-prompting models with massive context, Gas Town lets agents query the ledger, read structured "mail," receive non-destructive "nudges" delivered at turn boundaries, and recover state via `gt prime`. It is opinionated, thematically branded (mayor / polecats / refinery / witness / deacon / wasteland), and surprisingly mature — 405 files in `internal/cmd/`, full ACP proxy implementation, OTLP telemetry, three-tier health watchdog, a Bors-style merge queue, and federation across "towns" via DoltHub. This is the densest harness in this study set.

---

## 2. Repo Layout

Top-level (`harnesses/gastown\`):

- `cmd/gt/main.go` — CLI entry point (12 lines; just delegates to `internal/cmd`)
- `cmd/gt-proxy-server/`, `cmd/gt-proxy-client/` — ACP proxy server/client binaries
- `internal/cmd/` — **405 command files** (`agents.go`, `convoy.go`, `mail.go`, `nudge.go`, `prime.go`, `sling.go`, `done.go`, `mayor.go`, `polecat.go`, …)
- `internal/acp/` — Agent Client Protocol JSON-RPC proxy: `proxy.go`, `propulsion.go`, parity/handshake/keepalive tests
- `internal/mail/` — bead-backed inter-agent mail (`router.go`, `mailbox.go`, `delivery.go`, `types.go`)
- `internal/nudge/` — non-destructive nudge queue (`queue.go`, `poller.go`)
- `internal/protocol/` — internal RPC types and handlers
- `internal/agent/provider/` — model/runtime presets (claude, gemini, codex, copilot, cursor, auggie, amp, opencode, pi, omp)
- `internal/runtime/runtime.go` — per-role hook/settings provisioning, startup-fallback
- `internal/hooks/templates/{claude,codex,copilot,cursor,gemini,opencode,omp,pi}/` — provider-specific hook config templates
- `internal/polecat/`, `internal/witness/`, `internal/refinery/`, `internal/mayor/`, `internal/deacon/`, `internal/dog/`, `internal/boot/` — agent role implementations
- `internal/wasteland/` — federated cross-town work coordination
- `internal/wisp/`, `internal/formula/`, `internal/checkpoint/` — workflow / checkpoint primitives
- `internal/doltserver/`, `internal/beads/` — state persistence backed by Dolt
- `internal/tmux/`, `internal/keepalive/`, `internal/telemetry/` — terminal multiplexing + OTel
- `internal/refinery/engineer.go` — Bors-style bisecting merge queue
- `docs/design/` — 25+ design docs: `architecture.md`, `mail-protocol.md`, `escalation.md`, `scheduler.md`, `witness-at-team-lead.md`, `polecat-lifecycle-patrol.md`, `plugin-system.md`, …
- `docs/contrib-harnesses/`, `docs/runtimes/` — provider integration docs
- `Dockerfile`, `docker-compose.yml`, `flake.nix` — packaging
- `README.md` (~745 lines) — exceptionally detailed user-facing docs
- `AGENTS.md` — context loaded into every agent session

The codebase is **Go 1.25**, single repo, MIT licensed, ~CHANGELOG `1.0.1` released 2026-04-25 — actively maintained.

---

## 3. Architecture

### 3.1 Roles (the "Town")

| Role | Scope | Persistence | Job |
|---|---|---|---|
| **Mayor** | Town | Persistent | Top-level coordinator; user's primary chat interface (`gt mayor attach`) |
| **Deacon** | Town | Persistent | Daemon-driven supervisor; runs continuous patrol cycles across rigs |
| **Boot** | Town | Ephemeral | Watchdog that spawns when Deacon is down |
| **Dogs** | Town | Variable | Workers dispatched by Deacon for cross-rig maintenance |
| **Witness** | Per-rig | Persistent | Per-project lifecycle manager; monitors polecats, triggers recovery |
| **Refinery** | Per-rig | Persistent | Bors-style merge queue processor |
| **Polecats** | Per-rig | Persistent identity, ephemeral session | Worker agents — get assigned beads, run them in worktrees, `gt done` when finished |
| **Crew** | Per-rig | Persistent | The human's workspace |

Notably, the user (operator) is treated as a first-class addressable identity called `overseer` — see `internal/mail/types.go:554-557`:

```go
// Overseer (human operator) - no trailing slash, distinct from agents
if s == "overseer" {
    return "overseer"
}
```

### 3.2 Storage layer

All state is in **Dolt SQL Server** (one process per town, `~/gt/.dolt-data/`), addressed through the **beads** issue tracker. Two-level beads architecture (`docs/design/architecture.md:5-30`):

- Town beads at `~/gt/.beads/` with `hq-*` prefix — coordination, mayor mail, role definitions
- Rig beads at `<rig>/mayor/rig/.beads/` with project prefix — implementation work, MRs

This means **inter-agent messages are just rows in a versioned SQL database** — every message, every status change, every assignment is queryable history that survives any agent restart.

### 3.3 Worktree model

Polecats and refinery agents are **git worktrees**, not full clones (`docs/design/architecture.md:135-146`). The base is `mayor/rig`. Worktrees enable fast spawning (seconds) and shared object storage. Crew (humans) get full clones.

### 3.4 The agent loop (per polecat)

1. Mayor (or operator) issues `gt sling <bead-id> <rig>` — creates a worktree, spawns a polecat session in tmux running the configured runtime (e.g., `claude`).
2. Runtime starts → SessionStart hook fires `gt prime --hook && gt mail check --inject` (see `internal/hooks/templates/claude/settings-autonomous.json:100-110`).
3. `gt prime` injects role identity + work assignment as system context. `gt mail check --inject` pours pending mail into the prompt.
4. Polecat works on the bead, calling `bd update`, `bd close`, etc.
5. `UserPromptSubmit` hook on every turn re-runs `gt mail check --inject` — pending messages get delivered at the next natural prompt.
6. `gt done` writes a `POLECAT_DONE` mail bead → Witness → `MERGE_READY` → Refinery → bisecting merge.
7. On `Stop` hook: `gt costs record &` (telemetry).

The whole architecture revolves around **hook-driven mail delivery** rather than active polling.

---

## 4. Communication & Handoff

This is the single most relevant section for Handshake.

### 4.1 Three transport channels, deliberately separated

From `AGENTS.md:78-126` — the harness draws a hard line between three communication primitives:

| Channel | Persistence | Delivery | Use |
|---|---|---|---|
| **Mail** (`gt mail`) | Bead in Dolt; survives restart | Pulled via `gt mail check --inject` at hook time | Tasks, escalations, multi-line context |
| **Nudge** (`gt nudge`) | Transient JSON file in queue dir | Picked up at next agent turn boundary | Wake a sleeping agent; brief poke |
| **ACP session/prompt** | None | Injected directly into JSON-RPC stream | Real-time UI/agent updates when an IDE is connected |

The README documents this distinction explicitly:
> "**Important:** `gt nudge` is the ONLY way to send text to another agent's session. Never print 'Hey @name' — the other agent cannot see your terminal output." (`AGENTS.md:97-99`)

### 4.2 The Nudge queue — non-destructive turn-boundary delivery

`internal/nudge/queue.go:1-10` is the most elegant piece in the codebase:

> "The nudge queue allows messages to be delivered cooperatively: instead of sending text directly to a tmux session (which cancels in-flight tool calls), nudges are written to a queue directory and picked up by the agent's UserPromptSubmit hook at the next natural turn boundary."

Key mechanics:
- Queue dir per session: `<townRoot>/.runtime/nudge_queue/<session>/`
- Each nudge is a JSON file named `<unix-nano>-<random-hex>.json` for FIFO ordering with collision-resistance (`queue.go:127-130`)
- Drain uses **rename-then-process**: each file is atomically renamed to a `.claimed` suffix before reading, so concurrent drainers can't double-deliver (`queue.go:155-163`)
- Stale `.claimed` files older than 5 minutes from crashed drainers are *renamed back* to `.json`, not deleted, so the nudge survives drainer crashes (`queue.go:175-181`)
- TTLs: 30 min for normal, 2h for urgent (`queue.go:40-43`)
- Hard cap: `MaxQueueDepth = 50` per session (`queue.go:46`)
- Failed deliveries get **Requeue** with original timestamps preserved (`queue.go:140-153`)

Compare this to Handshake, where the orchestrator pokes terminals directly and explicitly worries about polling waste.

### 4.3 ACP Proxy — they implement the Zed Agent Client Protocol

`internal/acp/proxy.go` is a full **Agent Client Protocol** implementation (the JSON-RPC 2.0 protocol popularised by Zed). The proxy `Start()` (`proxy.go:181-199`) spawns the agent (`claude`, `gemini`, …) as a subprocess, wires `agentStdin`/`agentStdout`/`agentStderr` to its own pipes, and sits between the IDE and the model runtime.

This is **directly analogous to Handshake's ACP broker**. Differences:

- Gastown's proxy is per-session, not centrally shared — each polecat has its own ACP proxy process
- The proxy parses every `session/prompt`, every `session/update`, and tracks handshake state (`handshakeInit → handshakeWaitingForInit → handshakeWaitingForSessionNew → handshakeComplete`, `proxy.go:22-27`)
- It supports **propulsion** (`proxy.go:77`, `propulsion.go`) — a `Propeller` watches the nudge queue via fsnotify and **injects mail/nudges into both the UI (`session/update`) and the agent (`session/prompt`)** when they arrive (`propulsion.go:336-368`)

This dual injection ("Always notify the UI" then "Notify the Agent only if session is ready", `propulsion.go:344-353`) is novel — they explicitly couple terminal-style display and prompt injection so the UI sees the same thing the model is told, like `tmux` capture, but for IDEs.

### 4.4 Mail protocol — structured handoff with named verbs

`docs/design/mail-protocol.md` defines a small set of **named message types** (POLECAT_DONE, MERGE_READY, MERGED, MERGE_FAILED, REWORK_REQUEST, …). Each has:

- A fixed Subject format (e.g. `POLECAT_DONE <polecat-name>`)
- A fixed Body schema (key-value lines: `Exit:`, `Issue:`, `Branch:`, …)
- A defined Route (`Polecat → Witness`, `Witness → Refinery`, …)
- A defined Handler

This is **exactly** what Handshake is missing. Handoff is not "write a 4-page receipt" — it's "send `POLECAT_DONE` with three labelled lines and the Witness handler does the rest."

### 4.5 Mail is just labelled beads

`internal/mail/types.go:299-369` reveals: a mail message is a beads issue with `type=message`. Routing metadata (sender, thread, queue, channel, claimed-by, delivery-state) is encoded as **labels** like `from:gastown/Toast`, `thread:abc123`, `cc:mayor/`, `queue:bugfix`. This means:

- Sending mail = creating a bead (one CLI call, no document)
- Reading mail = `bd list` filtered by labels
- All mail is auto-versioned in git via Dolt
- "Threads" (`thread_id`) and "replies" (`reply_to`) are first-class — proper conversation state

### 4.6 Three routing modes

`internal/mail/types.go:230-259`:
- **Direct** — `To:` is set
- **Queue** — `Queue:` is set, eligible agents `claim` the message
- **Channel** — `Channel:` is set, broadcast to all readers

Mutually exclusive, validated. So a message is either point-to-point, work-stealing, or pub-sub — the system knows which.

---

## 5. Tools, Providers, State

### 5.1 Provider integration

Built-in agent presets (README key commands section):
> claude, gemini, codex, cursor, auggie, amp, opencode, copilot, pi, omp

Each has a hook-template in `internal/hooks/templates/<provider>/`. For Claude, that's `settings-autonomous.json` (autonomous polecats) and `settings-interactive.json` (the human-facing crew). For Cursor/Codex/Gemini, it's the equivalent JSON. For runtimes without hooks (Codex), the proxy emits a "startup fallback" sequence (`internal/runtime/runtime.go:85-115`):

```go
command := "gt prime"
if isAutonomousRole(role) {
    command += " && gt mail check --inject"
}
```

Per-rig runtime config sits in `settings/config.json` (README, "Runtime Configuration"). Runtime override per-spawn: `gt sling <bead-id> <rig> --agent cursor`.

### 5.2 Tools the polecat actually uses

The polecat is a vanilla Claude/Codex/Cursor session with the harness's CLI on PATH. So tools are **the standard runtime tools (Bash, Read, Edit, Write) plus the `gt` and `bd` CLIs**. The harness *does not* implement custom MCP tools — coordination is done through the CLI. The `bd` (beads) CLI handles all task-tracking; `bv` provides graph-aware triage (PageRank, betweenness, cycles).

Notably (from `AGENTS.md:155-158`):
> "Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists"
> "Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files"

**They explicitly forbid the agent from making local TODO/MEMORY documents.** All state goes through the structured ledger.

### 5.3 State / persistence model

- Code state: git worktrees per polecat
- Work state: beads (Dolt-backed SQL with git sync)
- Identity state: agent-bead per agent (`<prefix>-<rig>-polecat-<name>`)
- Role state: role-bead (global template, e.g. `hq-polecat-role`)
- Mail state: beads with `type=message`
- Nudge state: JSON files in `.runtime/nudge_queue/`
- Session events: `.events.jsonl` per session (read by `gt seance` for predecessor lookup)
- Telemetry: OTLP to VictoriaMetrics/Logs
- Federation: DoltHub-based "wasteland"

### 5.4 Permissions / safety

- Hooks enforce dangerous-command guards: `Bash(sudo *)`, `Bash(apt install*)`, `Bash(brew install*)` etc. all gated through `gt tap guard dangerous-command` (`settings-autonomous.json:36-98`)
- PR-creation guard: `Bash(gh pr create*)` and `Bash(git checkout -b*)` gated through `gt tap guard pr-workflow`
- `skipDangerousModePermissionPrompt: true` for autonomous polecats (they trust the hook guards instead)
- Polecats never push to main directly — Refinery (Bors queue) does, after verification gates
- Escalation severity routing: CRITICAL/HIGH/MEDIUM through Deacon → Mayor → Overseer

---

## 6. Risk Mitigation

The harness has named, encoded mitigations for several failure modes Handshake also faces:

- **Stuck agent detection** — Witness watches polecats; classifies states `GUPP Violation / Stalled / Zombie / Working / Idle` (README "Problems View"). Recovery: `nudge` then `handoff` (refresh context).
- **Daemon crash** — three-tier watchdog: Daemon → Boot → Deacon → Witness (`README:528-533`). Boot is the watcher's watcher.
- **Crashed nudge drainer** — orphan `.claimed` files older than 5 min get renamed back to `.json` (`queue.go:175-181`).
- **Queue overflow** — `MaxQueueDepth = 50`; Enqueue returns error rather than dropping silently (`queue.go:99-103`).
- **Session-not-ready races** — Propeller requeues nudges if proxy not handshaked (`propulsion.go:230-238`); ACP handshake has 30s timeout with degraded-mode fallback (`propulsion.go:151-168`).
- **API rate-limit exhaustion** — Scheduler caps concurrent polecats (`gt config set scheduler.max_polecats 5`).
- **Merge conflicts cascading** — Refinery uses bisecting Bors queue; bad MRs get isolated, good ones merge (`README:561-566`).
- **Context loss after compaction** — `PreCompact` hook re-runs `gt prime --hook` (`settings-autonomous.json:111-121`).
- **Agent "context bloat" from open mail** — explicit fix in CHANGELOG 1.0.1 (2026-04-25): `gt dog done` now **archives all open mails before clearing work**, after a bug where unclosed plugin mails ballooned context to 60-70% and froze the deacon. **This is the same failure mode Handshake suffers from artifact accumulation.**

---

## 7. Clever Engineering

What stands out:

1. **Turn-boundary nudging instead of mid-stream injection** (`nudge/queue.go`). The single biggest insight: don't tear into a running tool call to deliver a message. Drop a JSON file, let the agent's own `UserPromptSubmit` hook pull it. Cooperative > preemptive.
2. **Rename-then-process for queue drains** (`queue.go:155-163`). Atomic, robust to concurrent drainers, handles crashes by un-renaming stale claims. Production-grade primitive in ~30 lines.
3. **Mail is structured beads with named verbs** (`docs/design/mail-protocol.md`). POLECAT_DONE / MERGE_READY etc. are *named handoffs with schemas*, not free-form receipts. This is what bypasses documentation-as-blocking-gate: handoff is one bead with three labelled lines.
4. **Two-level beads** — town for coordination, rig for implementation, with prefix routing. Prevents the "everything in one issue tracker" mess.
5. **Worktrees for polecats** — sub-second agent spawn, no clone cost, shared object storage. Polecats are basically free.
6. **Forbid agent-local memory files** — `Use 'bd remember' for persistent knowledge — do NOT use MEMORY.md files` (`AGENTS.md:158`). Single source of truth.
7. **Wasteland federation** — towns post wanted-items to DoltHub, claim work from other towns, earn portable reputation stamps. Multi-tenant beyond single-machine.
8. **OTel-first** — every agent action emits structured logs and metrics. `gastown.session.starts.total`, `gastown.bd.calls.total`, `gastown.polecat.spawns.total`. Designed for fleet observability.
9. **`gt seance`** — agents can query *predecessor* sessions via `.events.jsonl` rather than re-reading the codebase. Conversation continuity without context bloat.
10. **Bors-style merge queue** with bisection — the harness ships its own CI/merge pipeline. Polecats never push to main.

This is **far past "minimal/early."** Gastown is the most complete harness in this study set.

---

## 8. Comparison to Pi, Hermes, OpenClaw

(High-level — the synthesizer will go deeper.)

| Axis | Pi | Hermes | OpenClaw | **Gastown** |
|---|---|---|---|---|
| Scope | Single-agent shell | Single-agent terminal | OSS Claude Code clone | Multi-agent town with named roles |
| Communication | n/a | n/a | n/a | **Mail (beads) + Nudge (queue) + ACP (stream)** |
| State persistence | Local files | Local files | Local files | **Dolt SQL server, git-versioned** |
| Provider abstraction | Single | Single | Multiple via internal | **10+ presets with per-provider hook templates** |
| Handoff format | n/a | n/a | n/a | **Named bead types (POLECAT_DONE, MERGE_READY, …)** |
| Multi-agent | No | No | No | **Yes, with role taxonomy and watchdog hierarchy** |

**Most striking similarity:** OpenClaw and Gastown both use ACP/JSON-RPC proxies between IDE and runtime — they share the Zed-style protocol. But OpenClaw is a single-agent IDE, while Gastown turns the proxy into a per-session message-bus (`Propeller`).

**Most striking difference:** Pi/Hermes/OpenClaw are *runtimes for one agent*. Gastown is **a coordination plane for many agents** that delegates the actual code-edit work to one of those single-agent runtimes (Claude, Codex, Cursor, …). It is *complementary*, not competitive, with the others.

**Closest sibling to Handshake:** Gastown is the only harness in this study set that solves the *same* problem Handshake solves — multi-agent coordination, role-bound sessions, structured handoff between roles, watchdog patrol of stuck agents, ACP-style proxy. The vocabulary is wildly different (mayor/polecats vs. orchestrator/coder), but the architecture maps almost 1:1. Handshake should treat Gastown as the primary reference design.

---

## 9. Lessons for Handshake

In rough order of impact for the user's token-burn / repair-loop pain:

1. **Replace governance receipts with named bead handoffs.** Handshake's "WP receipts / packets / dossiers" are equivalent to Gastown's `POLECAT_DONE` / `MERGE_READY` mails — except Gastown's are 3-5 labelled lines, not full markdown documents. Adopt a **named-verb mail protocol** with fixed schemas (`docs/design/mail-protocol.md` is the template). Models cannot mis-format a 4-line schema the way they mis-format a 4-page receipt.

2. **Adopt turn-boundary nudge queues.** The orchestrator pain about "mid-stream interruption cancels tool calls" is the exact problem `internal/nudge/queue.go` solves. Drop nudges as JSON files in a per-session dir; have the agent's hook drain them at the next prompt. **No polling, no tearing, FIFO with TTL, atomic claim-and-process, automatic re-queue on failure.** This is portable to any runtime with a `UserPromptSubmit`-style hook (Claude has it; Codex/Cursor have rough equivalents).

3. **Structured ledger > documents.** Replace as much WP/MT/RGF documentation as possible with **bead-equivalent structured rows**. Gastown's rule "use `bd remember`, NOT MEMORY.md files" is the inverse of where Handshake currently sits. Beads/Dolt is overkill for one developer; SQLite or even JSON-line files would do — but the *idea* (every coordination signal is a structured row, queryable, versioned) is the lesson.

4. **Hook-driven context injection, not orchestrator-driven prompt construction.** Gastown's `gt prime` runs in the SessionStart hook, not from the orchestrator. The agent self-rehydrates on every session start / pre-compact. Orchestrator never has to reconstruct context for a relaunch — the hook does.

5. **Forbid the agent from creating local memory artifacts.** Handshake spends cycles repairing receipts/packets that the agent itself authored. Gastown explicitly bans `MEMORY.md` / `TodoWrite` and routes everything through `bd`. **Documentation cannot become a blocking gate if there is no document.**

6. **Three-tier watchdog instead of orchestrator-as-watcher.** Gastown's Witness/Deacon/Boot lets the orchestrator stay strategic; per-rig Witness handles per-agent recovery; Boot watches Deacon. Handshake's orchestrator currently does all three.

7. **`gt seance` for predecessor lookup.** When a session compacts or restarts, agents query their predecessor's `.events.jsonl` instead of re-reading the codebase. Direct fix for re-entry context cost.

8. **Per-session ACP proxy with dual-channel injection.** Notify the UI *and* the agent in the same call (`propulsion.go:344-353`). Handshake's broker should treat IDE display and agent prompt as coupled, not one or the other.

9. **MR queue for code-edit safety.** Polecats never push to main; Refinery batches and verifies. For Handshake's coder-session work, route through a verification queue so a bad MT can't poison the main worktree.

10. **The smallest action**: even before any of the above, **forbid Handshake's orchestrator from sending free-form text between roles**. Force every cross-role message into a tiny set of named verbs with three labelled lines. That alone will collapse repair loops.

---

## 10. Source list

Primary (this repo):
- `harnesses/gastown\README.md`
- `harnesses/gastown\AGENTS.md`
- `harnesses/gastown\CHANGELOG.md` (esp. 1.0.1, 2026-04-25 — context-bloat fix)
- `harnesses/gastown\internal\acp\proxy.go`
- `harnesses/gastown\internal\acp\propulsion.go`
- `harnesses/gastown\internal\nudge\queue.go`
- `harnesses/gastown\internal\mail\types.go`
- `harnesses/gastown\internal\runtime\runtime.go`
- `harnesses/gastown\internal\hooks\templates\claude\settings-autonomous.json`
- `harnesses/gastown\docs\design\architecture.md`
- `harnesses/gastown\docs\design\mail-protocol.md`
- `harnesses/gastown\docs\design\witness-at-team-lead.md`
- `harnesses/gastown\docs\design\escalation.md`, `scheduler.md`, `polecat-lifecycle-patrol.md` (skimmed)

External (referenced in repo, not directly browsed for this report):
- `https://github.com/steveyegge/gastown` — canonical upstream
- `https://github.com/steveyegge/beads` — sister project, structured issue ledger
- `https://github.com/dolthub/dolt` — versioned SQL backing store
- Zed Agent Client Protocol (referenced via the JSON-RPC schema in `internal/acp/proxy.go`)

Author profile: **Steve Yegge** (formerly Google, Stitch Fix, Sourcegraph; well-known engineer, blogger). The vocabulary, opinionated banning of TODO/MEMORY files, and the "town" theming are unmistakably his. No external blog posts on Gastown specifically were retrieved for this report — primary truth is the code and `docs/`.

---

## NOTES FOR SYNTHESIZER

- Gastown is the **closest peer to Handshake** in this study — both are multi-agent harnesses with role-bound sessions, ACP proxies, and structured handoff between named roles. Read this report alongside the Handshake architecture for direct mapping.
- The **single most transferable mechanism** is `internal/nudge/queue.go` — turn-boundary, FIFO, rename-claim, TTL'd, requeue-on-failure. It directly addresses Handshake's "polling waste" and "mid-stream interruption" memos.
- The **second most transferable** is the named-verb mail protocol (`docs/design/mail-protocol.md`). Tiny labelled-line schemas, not free-form receipts. This is the antidote to Handshake's documentation-as-blocking-gate problem.
- Note CHANGELOG 1.0.1 — Gastown literally hit *exactly* the same bug Handshake describes: open mail accumulating, re-injected on every hook, ballooning context, freezing the supervisor. Their fix: archive on `done`. Worth quoting verbatim in the synthesis.
- Gastown's three-tier Daemon→Boot→Deacon→Witness chain is more elaborate than Handshake needs but the **principle** (don't make the orchestrator the watchdog) is the lesson.
- Don't oversell: Gastown is opinionated, theme-heavy, and Linux/macOS-first (Windows support arrived 2026-04-02). Handshake should borrow patterns, not vocabulary.
- The "forbid local memory files" rule is striking and probably more controversial than any technical mechanism — flag it for explicit operator decision.
- ACP details: their proxy is per-session, not centrally shared. Handshake's centralized broker is a *different* design choice, not a deficient one. Highlight the trade-off.
