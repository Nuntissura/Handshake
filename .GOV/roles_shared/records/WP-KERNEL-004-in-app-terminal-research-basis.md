---
file_id: WP-KERNEL-004-in-app-terminal-research-basis
file_kind: research-basis
updated_at: "2026-05-31"
wp: WP-KERNEL-004
spec_anchor: "master-spec-v02.188 §10.1 Integrated Terminal (TERM-V1-SCOPE, TERM-INVARIANTS)"
---

<topic id="purpose-and-scope" summary="What this research basis covers and why">

# Purpose and scope

This is the [GLOBAL-RESEARCH-048] research basis for the in-app interactive
terminal deliverable under WP-KERNEL-004. The deliverable is NOT a standalone
shell. The core requirement is a **capture seam**: an off-main-window,
disclosure-hosted terminal panel that can both (a) run interactive PTY sessions
and (b) **inspect all background work** — swarms, sub-agents, the cloud CLI
bridge, MCP servers, sandbox adapters — by attaching their already-piped
stdout/stderr to read-only AiJob sessions bound by `swarm_id`/`instance_id`, fanned
to the Flight Recorder and the swarm board.

Canonical law (spec §10.1): integrated panel; multiple tabbed sessions; AI-job
terminals SEPARATE from human; file-path linkification; policy-scoped security.
Invariants: AI command exec MUST be capability-checked + trace-linked; AI MUST
NOT type into human terminals by default; every AI-run command MUST appear in
the Flight Recorder.

This document records: sources checked, patterns found in real field
implementations, reuse opportunities in the existing `TerminalService`, rejected
options, the selected approach, risks + mitigations, and the validation plan,
so a fresh no-context model can see WHY the chosen design is field-aligned and
project-appropriate before any code is written.

</topic>

<topic id="sources-checked" summary="Field implementations and docs inspected">

# Sources checked

Inspected real source/docs, not only blog summaries:

- **wezterm `portable-pty` crate** (docs.rs, crates.io, GitHub `pty/src/win/conpty.rs`
  and `pty/src/unix.rs`). Confirmed: latest published **0.9.0**; trait set
  `PtySystem` / `PtyPair{master,slave}` / `MasterPty` / `SlavePty` / `Child`;
  `native_pty_system()` factory; `openpty(PtySize{rows,cols,pixel_width,pixel_height})`;
  `MasterPty::try_clone_reader()` (blocking `std::io::Read`), `take_writer()`
  (blocking `std::io::Write`), `resize()`, `get_size()`; `SlavePty::spawn_command(CommandBuilder)`;
  `Child::{wait, kill, try_wait}`. Targets include `x86_64-pc-windows-msvc` with a
  ConPTY backend that portable-pty manages internally.
- **Rust PTY reader pattern** (developerlife.com 2025-08, "Capturing Real-Time
  Build Progress from Cargo Using PTY and OSC Sequences"). Confirmed the
  field-standard bridge: the `try_clone_reader()` handle is **blocking**, so it is
  read on a dedicated thread (`tokio::task::spawn_blocking`) and bytes are pushed
  into an async channel; the slave is explicitly `drop()`-ped after spawn so the
  reader sees EOF (otherwise the fd leaks and the reader never terminates).
- **xterm.js v5** (xtermjs.org flow-control / encoding guides; GitHub issues
  #2077, #1918, #2326; PR #1904). Confirmed: `writeUtf8()` consolidated into
  `write()` which accepts `Uint8Array` (raw bytes treated as UTF-8); `write(chunk,
  callback)` fires the callback when the chunk is processed — the hook for
  backpressure; node-pty integrations use `pause()/resume()` with HIGH/LOW
  watermarks to avoid pause/resume thrash.
- **VS Code integrated terminal** (DeepWiki microsoft/vscode §6.2/6.3/6.6;
  GitHub issues #74620, #113827, #114377, #117265, #252018). Confirmed the
  reference architecture: a dedicated **Pty Host process** isolates heavy/hung
  shells from the UI; **flow control** tracks unacknowledged bytes and pauses the
  PTY at a high watermark until the renderer ACKs; a **headless xterm.js replay
  buffer** holds scrollback server-side; event batching coalesces small writes.
- **zellij** (DeepWiki zellij-org/zellij; client-server model). Confirmed a
  multiplexer keeps a server owning many PTYs + panes, a multi-threaded design so
  one noisy PTY cannot block others, and multiple clients attach to one session —
  the model for "many concurrent sessions keyed by id, multiple viewers."
- **Existing Handshake code** (read directly): `terminal/mod.rs` (TerminalService),
  `terminal/session.rs` (TerminalSessionType), `swarm_orchestration/events.rs`
  (BroadcastSwarmSink / DurableSwarmFrBridge / FanoutSwarmSink),
  `app/src-tauri/src/commands/swarm_runtime.rs` (`spawn_swarm_board_forwarder`),
  `model_runtime/cloud/official_cli_bridge.rs` (piped spawn + CREATE_NO_WINDOW),
  `app/src/components/common/Disclosure.tsx` (lazy collapsible), `Cargo.toml`
  (tokio + `base64 0.22` already present), `lib.rs` (handler macro + forwarder
  spawn site).

</topic>

<topic id="patterns-found" summary="Concrete field patterns the design adopts">

# Patterns found

1. **Blocking PTY reader on a dedicated thread → async channel.** portable-pty
   readers/writers are blocking `std::io` types by design (wezterm spawns a thread
   per reader). The correct Tokio bridge is `spawn_blocking` reading a fixed buffer
   in a loop and forwarding chunks over a channel; never `.await` a blocking PTY
   read on a runtime worker.
2. **Drop the slave after spawn.** The slave handle must be dropped once the child
   is spawned so the master reader receives EOF on child exit; otherwise the
   reader hangs and the session leaks.
3. **Flow control via watermark + ACK (VS Code).** Server counts bytes written to
   the broadcast; if unacknowledged bytes exceed a HIGH watermark, pause draining
   the PTY reader until the frontend ACKs down to a LOW watermark. xterm.js gives
   the per-chunk `write(data, cb)` callback as the ACK trigger. For a v1 we can
   approximate with a **bounded broadcast + resync** (matching the existing swarm
   board), upgrading to true ACK flow-control only if floods prove it necessary.
4. **Bounded broadcast + seq + resync (existing swarm board).** `BroadcastSwarmSink`
   already implements exactly the lag-aware fan-out the terminal needs: a bounded
   `tokio::broadcast`, a single forwarder task assigning a monotonic `seq`, and a
   `Lagged(dropped)` arm that emits a resync. The terminal forwarder is a near
   carbon copy emitting `terminal://output` / `terminal://exit` / `terminal://resync`.
5. **base64 chunk over Tauri events.** Tauri events serialize JSON; raw PTY bytes
   (which include control sequences and invalid UTF-8 fragments) must be carried
   as `chunk_base64`, decoded on the frontend to `Uint8Array`, and handed to
   `term.write(bytes)`. This preserves partial UTF-8 multibyte sequences across
   chunk boundaries (xterm.js reassembles them).
6. **Captured background process = read-only session.** VS Code "task terminals"
   and zellij "command panes" bind an already-running process's output to a
   view-only pane. The Handshake analogue: a `CaptureSink` that any background
   producer (already piping stdout/stderr) clones its bytes into; the sink fans to
   the same broadcast + FR, tagged with `swarm_id`/`instance_id`, and the session
   is marked `AiJob` + read-only (no writer wired).
7. **Quiet spawn on Windows.** `CREATE_NO_WINDOW (0x0800_0000)` is already applied
   at CLI/MCP spawn sites; portable-pty owns ConPTY allocation itself, so the PTY
   session does not pop a console — do not regress the existing flag at the piped
   producer sites.

</topic>

<topic id="reuse-opportunities" summary="Existing TerminalService and infra to reuse, not duplicate">

# Reuse opportunities

The existing backend already solves the hard governance pieces; the new PTY layer
must compose them, not re-implement them.

- **`TerminalService::run_command`** (`terminal/mod.rs`) — REUSE verbatim for the
  one-shot `kernel_terminal_run_command`. It already does capability gating,
  cwd validation, timeout, cancellation (watch channel), secret redaction, the
  FR `TerminalCommand` event, and DuckDB output storage. The interactive runtime
  wraps it for one-shots; it does NOT re-roll redaction/guards/config.
- **`SecretRedactor`** (`terminal/redaction.rs`) — REUSE on **captured streaming
  output** before it reaches the broadcast/FR. The redactor's `redact_output` is
  the same call `run_command` uses; the streaming path must apply it per-chunk
  (with carry across chunk boundaries so a secret split across two reads is still
  caught — a documented risk below).
- **`TerminalGuard` / `DefaultTerminalGuard`** (`terminal/guards.rs`) — REUSE for
  the capability + session-isolation checks before interactive AI exec and before
  an AiJob session is allowed to "take control" (write stdin). `check_session_isolation`
  is exactly the AI-must-not-type-into-human gate (TERM-INVARIANTS).
- **`TerminalConfig`** (`terminal/config.rs`) — REUSE for max-output bytes,
  timeout, kill grace, redaction toggle. The PTY scrollback cap reuses the same
  config surface (add a `max_scrollback_bytes` if not present).
- **`TerminalSessionType`** (`terminal/session.rs`) — REUSE the HumanDev/AiJob/
  PluginTool enum as the per-session tag in the new runtime registry. Capture
  sessions are `AiJob`.
- **`BroadcastSwarmSink` / `DurableSwarmFrBridge` / `FanoutSwarmSink`**
  (`swarm_orchestration/events.rs`) — the terminal runtime's output fan-out is a
  direct structural copy: bounded broadcast for the live UI, a bounded
  FR bridge (`try_send` + dropped counter) for durable capture, fanned together.
  Reuse the SAME `FlightRecorder` the swarm path uses for FR-EVT-TERMINAL-* events.
- **`spawn_swarm_board_forwarder`** (`commands/swarm_runtime.rs`) — the literal
  template for the new `terminal://` forwarder: subscribe to a broadcast, assign
  monotonic `seq`, emit typed deltas, handle `Lagged -> resync`, `Closed -> break`.
- **Official CLI bridge piped spawn** (`official_cli_bridge.rs` lines 480-494) —
  the first real capture producer: it already `Stdio::piped()`s stdout/stderr and
  applies `CREATE_NO_WINDOW`. Attach a `CaptureSink` clone of those pipes.
- **Frontend `Disclosure`** (`components/common/Disclosure.tsx`) — REUSE with
  `lazy + defaultOpen=false` as the off-main-window panel host: collapsed by
  default, children (xterm.js + subscriptions) not mounted until first opened, so
  a closed panel costs nothing. Stable `data-testid`s already present for the
  visual matrix.
- **Swarm board read-model** (`SwarmBoard.tsx`, `lib/ipc/swarm_runtime.ts`) — the
  board affordance "open this swarm's captured terminal" reuses the existing
  `swarm_id` swimlane and `subscribeBoardEvents` pattern.
- **`base64 0.22` + `tokio` (sync/rt-multi-thread/io-util)** — already in
  `Cargo.toml`; only `portable-pty` is a NEW dependency.

</topic>

<topic id="rejected-options" summary="Approaches considered and why they were rejected">

# Rejected options

- **Roll our own ConPTY/openpty FFI.** Rejected: ConPTY pseudoconsole lifecycle
  (CreatePseudoConsole, attribute lists, anonymous-pipe plumbing) is exactly the
  bug surface wezterm has already hardened across versions. portable-pty is the
  field-tested abstraction and cross-platform-future-proofs the Linux VM-worktree
  swarm runtime.
- **`pty-process` / `conpty` (alternate crates).** Rejected: narrower or
  less-maintained; `pty-process` is Unix-leaning, the standalone `conpty` crate is
  Windows-only. portable-pty 0.9.0 covers both with one trait set and is the same
  family VS Code's node-pty occupies conceptually.
- **`tokio::process::Command` with piped stdout (what `run_command` already does)
  for the INTERACTIVE terminal.** Rejected for interactivity: pipes are not a TTY,
  so programs that detect a terminal (color, line editing, progress bars, `less`,
  REPLs) behave differently or hang waiting for a tty. Pipes remain correct for
  the one-shot `run_command` and for the CAPTURE seam (the producers already use
  pipes and we only mirror their bytes), but the human/AI interactive shell needs
  a real PTY.
- **Async PTY read directly on a tokio worker.** Rejected: portable-pty readers
  are blocking; awaiting them starves the runtime. Must use `spawn_blocking`.
- **Raw bytes as a JSON array or UTF-8 string over Tauri events.** Rejected:
  number-array bloats payloads ~4x and UTF-8 string loses invalid/partial
  multibyte control data. base64 is compact and lossless for arbitrary bytes.
- **Unbounded channel/broadcast for output.** Rejected: a flooding child would
  grow memory without bound. Bounded broadcast + dropped/resync (swarm board
  pattern) makes loss observable and recoverable instead of OOM.
- **A separate pty-host OS process (full VS Code isolation).** Deferred, not
  adopted for v1: VS Code's out-of-process pty host exists to survive renderer
  reloads and isolate crashes. In Tauri the backend is already a separate process
  from the WebView; `kill_on_drop` + a reaper give us crash isolation without a
  third process. Revisit if persistent-across-reload terminals become a
  requirement.
- **Wiring stdin for AiJob capture sessions by default.** Rejected by
  TERM-INVARIANTS: capture/AiJob sessions are read-only (inspect) until an
  explicit operator "Take control / interact" toggle passes the capability gate.

</topic>

<topic id="selected-approach" summary="The chosen design, end to end">

# Selected approach

**Backend (handshake_core/src/terminal/):**

- Add `portable-pty = "0.9"` to `src/backend/handshake_core/Cargo.toml`.
- `pty.rs` — `PtySession`: spawn a shell (default OS shell, configurable) via
  `native_pty_system().openpty(PtySize)` + `slave.spawn_command(CommandBuilder)`;
  **drop the slave** after spawn. A `spawn_blocking` reader thread reads the
  cloned reader into a fixed buffer and pushes chunks to a **bounded
  `tokio::broadcast`** (the scrollback cap truncates with a marker when exceeded).
  A `take_writer()` held behind a mutex for stdin; `resize()` and `get_size()`
  pass-through; `Child::kill` on drop (`kill_on_drop` semantics) + on explicit
  close. Child crash is surfaced as a `terminal://exit{exit_code}` event, never
  swallowed.
- `runtime.rs` — `TerminalRuntime`: a registry (`HashMap<session_id, …>`) of many
  concurrent sessions, each carrying `TerminalSessionType`, optional
  `swarm_id`/`worktree_id`/`instance_id` binding, capability scope, and an FR
  trace id. Methods: `create / write / resize / close / list / run_command /
  scrollback`. `run_command` delegates to `TerminalService::run_command` (reuse).
  A reaper closes leaked/idle sessions. Output fan-out = `FanoutSwarmSink`-style:
  bounded broadcast (live UI) + bounded FR bridge (durable, dropped counter).
- `CaptureSink` / `TerminalCaptureRegistry` — the capture seam: a producer that
  already pipes stdout/stderr clones its bytes into a `CaptureSink` bound to a
  read-only `AiJob` session tagged with `swarm_id`/`instance_id`. The sink applies
  `SecretRedactor` per-chunk, fans to the same broadcast + FR + board. **First
  real producers:** the cloud CLI bridge (`official_cli_bridge.rs`, already piped)
  and the swarm session spawn path (`production_factory.rs` /
  `swarm_runtime.rs`). The sink interface is uniform so MCP stdio
  (`mcp/transport/stdio.rs`) and sandbox adapters attach the same way later.
- FR events `FR-EVT-TERMINAL-SESSION-OPEN / -COMMAND-EXEC / -SESSION-CLOSE` via the
  same `FlightRecorder` the swarm path uses, through a bounded channel with a
  dropped counter (mirror `DurableSwarmFrBridge`).

**IPC (app/src-tauri/src/commands/terminal.rs):**

- `kernel_terminal_create_session`, `_write_stdin`, `_resize`, `_close_session`,
  `_list_sessions`, `_run_command` (one-shot), `_scrollback`. A managed
  `TerminalRuntimeState`.
- `spawn_terminal_forwarder(app, broadcast)` mirroring `spawn_swarm_board_forwarder`:
  emits `terminal://output {session_id, seq, chunk_base64}`,
  `terminal://exit {session_id, exit_code}`, and on `Lagged` emits
  `terminal://resync {session_id, dropped}`.
- **lib.rs (Integrate phase owns lib.rs):** register every `kernel_terminal_*` in
  **both arms** of `handshake_invoke_handlers!`, `.manage(TerminalRuntimeState)`,
  and spawn the terminal forwarder in `setup` next to the swarm forwarder.

**Frontend (app/src/components/terminal/ + lib/ipc/terminal.ts):**

- `TerminalPanel.tsx` — off-main-window dockable drawer via `Disclosure`
  (`lazy`, collapsed by default); tabs per session grouped by swarm; `@xterm/xterm`
  + `addon-fit` + `addon-web-links` (file-path linkification) + `addon-search`.
  **AiJob sessions are READ-ONLY by default** (inspect); an explicit "Take control /
  interact" toggle must pass the capability gate before stdin is wired, honoring
  TERM-INVARIANTS.
- `lib/ipc/terminal.ts` — create/write/resize/close/list/subscribe; subscribe
  decodes `chunk_base64` to `Uint8Array` and `term.write(bytes)`; a `seq` gap or
  `terminal://resync` triggers scrollback refetch.
- Board affordance in `SwarmBoard.tsx` to open a swarm's captured terminal by
  `swarm_id`. Terminal rows in `SettingsMenu` (default shell, max scrollback,
  output-logging policy) — honestly **DISABLED** (`NotYetWiredRow`) until wired.
- Add `@xterm/xterm` + `@xterm/addon-fit` + `@xterm/addon-web-links` +
  `@xterm/addon-search` to `app/package.json`.

**Honesty discipline:** no mockups; any un-backed control is DISABLED, never
faked (matches the existing `ProviderNotConfigured`/`NotYetWiredRow` posture).

</topic>

<topic id="risks-and-mitigations" summary="Fail scenarios and the hardening for each">

# Risks and mitigations

- **PTY child crash silently leaves a dead tab.** Mitigation: reader EOF + `Child::wait`
  → emit `terminal://exit{exit_code}`; the tab shows the exit code, never a frozen
  blank.
- **Output flood OOM / UI freeze.** Mitigation: bounded broadcast + scrollback
  byte cap with a truncation marker; (future) VS Code-style watermark+ACK flow
  control if a v1 bounded-buffer proves insufficient under real floods.
- **Broadcast backpressure / slow viewer drops events.** Mitigation: bounded
  channel, `Lagged(dropped)` → `terminal://resync` → frontend refetches scrollback
  (exact swarm-board pattern).
- **Secret split across two read chunks evades redaction.** Mitigation: streaming
  redactor keeps a carry buffer of the last N bytes (max secret token length)
  across chunk boundaries before fan-out; document the carry size; redact on the
  capture path BEFORE broadcast and BEFORE FR.
- **AI types into a human terminal (TERM-INVARIANTS breach).** Mitigation:
  `check_session_isolation` gate; AiJob/capture sessions have NO writer wired by
  default; "Take control" requires the capability gate to pass first.
- **Interactive AI exec without capability.** Mitigation: capability check (reuse
  guard) before any stdin write on an AiJob session; deny is FR-audited like
  `run_command` already does.
- **Session leak (orphan PTY/child).** Mitigation: `kill_on_drop`/`Child::kill` on
  close + a reaper that closes idle/leaked sessions; FR-EVT-TERMINAL-SESSION-CLOSE
  on every teardown so START/CLOSE counts reconcile (the swarm no-orphan
  invariant).
- **FR write overwhelmed.** Mitigation: bounded FR channel + dropped counter
  (mirror `DurableSwarmFrBridge`); loss is observable, not silent.
- **ConPTY window pops on Windows / regresses CREATE_NO_WINDOW.** Mitigation:
  portable-pty manages ConPTY; do NOT touch the existing `CREATE_NO_WINDOW` flag at
  piped producer sites; assert via the QUIET build-rule check.
- **Worktree path mismatch (portability).** Mitigation: no hardcoded shell path;
  default shell from config/env with an OS fallback (`cmd.exe`/`$SHELL`), honoring
  [GLOBAL-PORTABILITY].
- **base64 chunk boundary splits a UTF-8 multibyte char.** Mitigation: xterm.js
  `write(Uint8Array)` reassembles partial multibyte sequences across writes; do
  NOT String::from_utf8 on the streaming path — pass raw bytes through base64.

</topic>

<topic id="validation-plan" summary="Gates and how each track is proven">

# Validation plan

Gates (all must pass; real implementation only):

1. `cargo build --manifest-path app/src-tauri/Cargo.toml --lib` — compiles with the
   new portable-pty dep and IPC commands.
2. `cargo test` for the terminal modules — unit/integration:
   - PtySession spawns a real short-lived shell command, captures its output,
     observes EOF + exit code (cfg(windows) path exercised since target is MSVC).
   - Slave-drop → reader EOF (no hang) proven with a timeout-bounded test.
   - Bounded broadcast lag → resync signal asserted.
   - Capture seam: a fake piped producer's bytes appear on the broadcast + an FR
     event, tagged with the bound `swarm_id`, redacted.
   - Capability gate denies AiJob stdin without "take control"; isolation gate
     denies AI→human write.
   - Reaper closes a leaked session; SESSION-OPEN/CLOSE counts reconcile.
3. `cd app && npx tsc --noEmit` — frontend types clean.
4. `npx vitest run` — `lib/ipc/terminal.ts` base64-decode→write, seq-gap→resync;
   `TerminalPanel` read-only-by-default + "Take control" toggle behavior
   (@testing-library/react, jsdom).
5. `npx playwright test` for the panel (`app/tests/visual/*.spec.ts`, fixtures via
   `page.setContent`, baselines under `.GOV/visual_baselines/`): collapsed-by-
   default, readable when opened, tabs grouped by swarm, no overlap, disabled
   controls visibly disabled. Use the WebView2 CDP harness (`visual_debug.rs`,
   `kernel_visual_debug_*`) for live DOM/state snapshots.

Discipline: each track adversarially reviewed by an independent agent and
remediated; fail-scenario hardening from the risks topic must be demonstrably
present (PTY crash surfaced, flood capped, backpressure resync, redaction on
capture, capability gate, no AI-into-human, kill_on_drop + reaper, FR drop
counter). Leave changes UNCOMMITTED; the orchestrator reviews verdicts and commits.

</topic>
