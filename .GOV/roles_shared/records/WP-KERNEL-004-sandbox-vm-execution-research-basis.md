---
file_id: WP-KERNEL-004-sandbox-vm-execution-research-basis
file_kind: research_basis
updated_at: 2026-06-01
wp: WP-KERNEL-004
title: Sandbox microVM model execution (SandboxModelRuntime) research basis
honesty_boundary: live CH end-to-end requires operator KVM/WSL/CH desktop; headless = fake adapter + #[ignore] live test
selected_approach: ephemeral-exec-per-generate over hsk.cmd+serial via existing CH adapter (wave 1)
---

<topic id="operator-request-and-scope" status="active" wp="WP-KERNEL-004" summary="What wave 1 must build and the honesty boundary">

# Operator request

Route swarm model execution INTO a `cloud_hypervisor` Tier-3 microVM so a real LOCAL model (a `llama.cpp` runner + a GGUF the operator confirms exist under `/home/ilja_smets/handshake-sandbox/`) runs INSIDE an isolated microVM worktree. The swarm proxies `generate()` to it, with per-worktree snapshot/restore state recovery. Operator chose `cloud_hypervisor` + snapshot recovery (the most vision-aligned path).

# Wave-1 deliverable (the remaining vision, wave 1)

1. A `SandboxModelRuntime` (new module `model_runtime/sandbox_runtime.rs`) impl `ModelRuntime`, holding an injected `Arc<dyn SandboxAdapter>` + model config (GGUF host path, llama runner guest path/cmd template). `load()` validates/prepares + mints a `ModelId`. `generate(req)` drives inference INSIDE the CH microVM via the adapter `exec` path, captures the completion from serial/exec output, and streams it back as `GeneratedToken`s through the OS-thread + tokio mpsc + `stream::unfold` bridge. Honest error item on exec failure; `cancel()` kills the VM via `adapter.kill`.
2. Factory routing: `ProductionModelSessionFactory` gains an injected `Arc<SandboxAdapterRegistry>`; `create()` routes a `Local`+`LlamaCpp` spawn whose `SpawnRequest.isolation_tier == Some(Tier3Microvm)` to a new `create_sandboxed_local` path that selects the CH adapter via registry/selection, builds the `SandboxModelRuntime`, and records the process-ledger START with `sandbox_adapter_id` + `sandbox_internal_id` POPULATED (and STOP on teardown). Non-sandbox spawns unchanged.
3. Worktree binding + state recovery: bind the session `worktree_id` to a persistent CH VM for that worktree; provide `snapshot(worktree)` / `restore(worktree)` wiring (reuse the CH snapshot/restore + adapter clone-safety). Design decides how much recovery lands in wave 1 vs a flagged follow-on; at minimum snapshot/restore is REACHABLE for a worktree-bound VM and the path is tested against the fake adapter.
4. Tests (headless, FAKE adapter) + one `#[ignore]`-d operator-desktop live test.

# Honesty boundary (stated in code + reports)

A real model inferring inside a real CH microVM requires the operator's KVM/WSL/Cloud-Hypervisor desktop and CANNOT run in the headless agent/CI. So: BUILD the plumbing for real, UNIT-TEST it headlessly against an injected FAKE `SandboxAdapter`, and gate the LIVE CH end-to-end as an `#[ignore]`-d operator-desktop test. The live path is NEVER faked-as-passing. This mirrors the existing `#[ignore]` pattern: `terminal/pty.rs:565-577` (real-ConPTY round-trip, runnable via `cargo test -- --ignored`) and the env-gated real-parallel swarm test.

# Non-goals (wave 1)

- No persistent in-VM serial daemon / vsock guest agent (flagged follow-on; the CH adapter's `exec` already fails closed on a persistent handle — `cloud_hypervisor/adapter.rs:816-836` — because the idle initramfs only loops printing TICK).
- No allowlist networking inside the VM (CH boots with no net device; only `DenyAll`/`LoopbackOnly` — `adapter.rs:1008-1025`).
- No real per-token streaming out of the runner (ephemeral exec captures the whole completion, then chunks it; flagged below).

</topic>

<topic id="grounded-surfaces" status="active" wp="WP-KERNEL-004" summary="Exact existing code to reuse, with file/line anchors; do NOT rebuild">

# Reuse map (inspected, do NOT rebuild)

## `SandboxAdapter` trait — `sandbox/adapter.rs:139-239`
`async exec(&handle, Command) -> ExecResult{exit_code, stdout: Bytes, stderr, duration_ms}`, `spawn`, `fs_bind`, `net_policy`, `kill(&handle, Signal)`, `snapshot(&handle)->SnapshotRef` (default `SnapshotUnsupported`; CH overrides), `restore(&SnapshotRef)->ProcessHandle` (CH overrides), `capabilities()`. The runtime is injected as `Arc<dyn SandboxAdapter>` so tests use a fake.

## Types — `sandbox/types.rs`
`Command { argv: Vec<String>, env_overlay: BTreeMap, stdin: Option<Bytes>, timeout_ms: Option<u64> }` (line 277). `ProcessSpec { id, image_or_root, cmd, env, cwd, binds:Vec<BindSpec>, net_policy, resource_limits, required_capabilities, trust_class, metadata }`. `BindSpec { host_path, guest_path, mode: ReadOnly|ReadWrite|NoExec }`. `NetPolicy::DenyAll`. `ResourceLimits { memory_bytes, cpu_cores, timeout_ms, ... }` (line 171). `ExecResult` (line 224) — `stdout` is `Bytes`. `SnapshotRef { id, adapter_id, snapshot_dir, created_at_utc, observe_path }` (line 241).

## CH adapter — `sandbox/cloud_hypervisor/adapter.rs`
- Ephemeral `exec` (807-967): builds a per-exec initramfs baking the declared binds, base64-encodes the joined argv into `hsk.cmd=` on the kernel cmdline, boots a FRESH CH microVM via `wsl.exe`, parses the serial log between `---HSK-BEGIN--- / ---HSK-END rc=<code>---` markers, returns `ExecResult{exit_code, stdout=parsed.stdout (Bytes), ...}`. RW binds tar back via `---HSK-FILES-BEGIN/END---`. **This is the wave-1 inference channel.**
- `exec` on a PERSISTENT handle FAILS CLOSED (816-836): "exec into a persistent (snapshot-capable) VM is not yet supported; it requires a vsock guest agent (out of scope)." => persistent-server pattern is NOT available without new guest-agent work.
- `net_policy` (1008-1025): no `--net` flag at boot => `DenyAll`/`LoopbackOnly` satisfied by absence of guest networking; `Allowlist` fails closed.
- `snapshot` (1104+): requires a persistent handle (`hsk.sandbox.mode=persistent` metadata); `ch-remote pause` + `snapshot file://<dir>`; carries the original serial path on `SnapshotRef.observe_path`.
- `restore` (1240+): TOCTOU-clone-safe — mints handle id EARLY, `try_reserve_restore` atomic gate + reservation, copies snapshot into a per-restore scratch root, rewrites CH `serial.file`, restores from the COPY with `resume=true`, `release_restore_reservation` on every failure path. Reuse as-is.
- `kill` (1027-1074): persistent handle => terminates the live CH child + cleans socket/scratch; ephemeral => in-flight child reaped by `kill_on_drop`.

## CLI-bridge runtime (THE TEMPLATE) — `model_runtime/cloud/cli_bridge_runtime.rs`
Mirror this exact shape: a sync `generate()` spawns a dedicated OS thread (the blocking work must NOT occupy a tokio worker), the thread sends `Ok(GeneratedToken{...})` onto an `mpsc::unbounded_channel`, and the async side is `Box::pin(stream::unfold(state, ...))` draining the channel (591-682). Honest errors: `single_error_stream(ModelRuntimeError::GenerateError(..))` (688) for a preflight failure; a producer-side `tx.send(Err(GenerateError(..)))` (510-514) for an exec failure — **never a silently empty stream**. Terminal token carries `finish_reason` (`Stop`/`Cancelled`/`Error`) (693-700). `cancel()` flips a `runtime_cancel` token the producer polls (790-793). `score/embed/kv_cache/lora_stack/steering_hooks` all return `CapabilityNotSupported` (743-788) — copy verbatim for the sandbox runtime.

## `ModelRuntime` trait — `model_runtime/trait.rs:10-34`
`async load(LoadSpec)->ModelId`, `async unload`, **sync** `generate(req)->TokenStream` (returns a `Box::pin`ned `Stream<Item=Result<GeneratedToken, ModelRuntimeError>>`), `cancel(CancellationToken)`. `GenerateRequest` (`types.rs:152`): `id, prompt: GenPrompt{text}, sampling, cancel: CancellationToken, max_tokens: u32, stop_sequences, ...`.

## Existing `model_runtime/sandbox_binding.rs` — PARTIAL reuse, NOT a substitute
`process_spec_from_model_spec` (171-236) already builds a `ProcessSpec` with a GGUF `ReadOnly` bind under `/models/gguf`, `NetPolicy::DenyAll`, guest-path translation, and metadata. **Reuse the ProcessSpec-construction logic.** BUT it is the WRONG runtime shape for this deliverable: (a) it `argv`s `llama-server` (a network server) — useless in a no-net CH VM; wave 1 needs `llama-cli`/`llama-run` ONE-SHOT; (b) it uses `adapter.spawn` (boxing a process), not `adapter.exec` (run-and-capture), which is what the ephemeral CH channel implements; (c) it impls `ModelAdapter` (kernel adapter), not `ModelRuntime`, and returns a synthetic `response_text`, never streamed tokens. Treat it as the ProcessSpec builder to lift from, not the runtime.

## Factory — `swarm_orchestration/production_factory.rs`
`create()` (852-868) dispatches `None|Local` + `RuntimeBinding::LlamaCpp` -> `create_local_llama` (743-798). Add a sandbox branch keyed on `request.isolation_tier == Some(Tier3Microvm)`. `record_start` (674-691) builds `SpawnMeta` but never sets the sandbox fields — wire them.

## `SpawnRequest` — `swarm_orchestration/ids.rs:40-223`
Already carries `runtime_binding`, `worktree_id: Option<String>` (80), `working_dir` (87), `isolation_tier: Option<IsolationTier>` (94) with builders `with_worktree` / `with_isolation_tier` (177-196) and accessors (212-223). NOTE the field docs say these are currently ATTRIBUTION-ONLY / not enforced — wave 1 makes `isolation_tier` load-bearing for the Local+LlamaCpp route only.

## Process ledger — `process_ledger/table.rs`
`SpawnMeta` has `sandbox_adapter_id: Option<String>` (224) + `sandbox_internal_id` (225) with builders `with_sandbox_adapter_id` (281) / `with_sandbox_internal_id` (286). The upsert SQL already persists both (23-24, 59-60, 80-81). `record_start` must call these builders with the CH adapter id + the `ProcessHandle.sandbox_internal_id`.

</topic>

<topic id="external-research" status="active" wp="WP-KERNEL-004" summary="Field implementations checked: llama.cpp in microVMs, serial vs vsock, sizing, snapshot warm-start">

# Sources checked

- Firecracker FAQ / SPECIFICATION / device model — minimal 6-device model (virtio-net, -balloon, -block, -vsock, serial console, stop-keyboard). Confirms serial console is a first-class, always-available guest<->host channel and that a microVM can be booted with NO network device. (github.com/firecracker-microvm/firecracker)
- Cloud Hypervisor `snapshot_restore.md` + release notes — snapshot = memory-ranges + `state.json`; restore via `--restore source_url=file://...`, VM resumes PAUSED, must be explicitly resumed; `memory_restore_mode` (userfaultfd lazy population) + sparse memory backing dramatically cut restore-to-resume latency — the warm-start primitive for a model-loaded VM. (github.com/cloud-hypervisor/cloud-hypervisor/blob/main/docs/snapshot_restore.md)
- Cloud Hypervisor `vsock.md` + Guest Agent discussion #5431 + Cocoon / Kuasar / cocoonstack — vsock is the modern host<->guest command transport (stream sockets, no SSH, kubectl-exec-style stdin/stdout/stderr); a guest agent (QMP-over-vsock prototype; Kuasar `vmm-task`; cocoon-agent) is what enables `exec` into a *running* VM. Serial console is the fallback when no agent exists. (github.com/cloud-hypervisor/cloud-hypervisor/blob/main/docs/vsock.md, discussion #5431, github.com/cocoonstack/cocoon)
- llama.cpp `llama-cli(1)` manpage + discussion #15709 + issue #1689 — single-shot non-interactive generation: `llama-cli -m model.gguf -p "<prompt>" -n <N> -no-cnv --single-turn --no-display-prompt --log-disable`. `-no-cnv` / `--single-turn` prevent the interactive REPL; `--no-display-prompt` keeps stdout = completion only; `--log-disable` keeps stderr noise out of the captured stream. (manpages.debian.org/.../llama-cli.1, github.com/ggml-org/llama.cpp/discussions/15709, issue #1689)
- llama.cpp memory-sizing discussions (#3847, #638, issue #13) + 2026 VRAM guides — rough rule ~0.56 GB per 1B params at Q4; 1B-Q4 ~0.6 GB, 3B-Q4 ~1.7 GB, +1-2 GB KV cache/runtime overhead at 4K-8K context => 2-4 GB total for a practical CPU-only microVM. (github.com/ggml-org/llama.cpp/discussions/3847, localllm.in)
- Firecracker serverless-inference / cold-start material + Tangram (arXiv 2512.01357) — for one-shot "generate a chart / run inference" the dominant metric is cold start; sub-second snapshot restore favours a warm pool; ephemeral stateless model-server aligns with the microVM design philosophy, persistent long-running services need warm-pool autoscaling complexity. (aws.amazon.com/blogs/.../firecracker, arxiv.org/pdf/2512.01357, manveerc.substack.com/p/ai-agent-sandboxing-guide)

# Patterns found

1. **Serial-console capture is the established no-network channel.** The CH adapter already implements exactly this: kernel-cmdline command injection (`hsk.cmd=<base64>`) + framed serial output (`---HSK-BEGIN--- ... ---HSK-END rc=N---`). This is the field-standard "no SSH, no net" exec channel, identical in spirit to a vsock agent but with serial as the transport.
2. **vsock + guest agent is the field-standard for `exec` into a *running* VM** (Cocoon, Kuasar, the CH guest-agent prototype). This is the upgrade path to a persistent in-VM `llama-server`/daemon with real per-token streaming — but it requires building a guest agent, which the CH adapter explicitly flags as out of scope today.
3. **Snapshot/restore = warm-start for a model-loaded VM.** CH `snapshot` of a VM that has already loaded the GGUF into RAM, then `restore` with lazy userfaultfd population, is the proven path to skip model reload on subsequent starts. The CH adapter's `snapshot`/`restore` already exist and are clone-safe.
4. **Ephemeral-boot-per-inference is the simplest correct serverless-inference pattern** and is what the existing ephemeral CH `exec` gives us for free.

# Reuse opportunities

- The CH adapter's ephemeral `exec` IS the inference channel — no new VM plumbing.
- The CLI-bridge runtime's OS-thread + mpsc + `unfold` bridge IS the token-stream shape — copy it, swap the byte source from a CLI subprocess to one `adapter.exec().await` call.
- `sandbox_binding.rs::process_spec_from_model_spec` IS the GGUF-bind + DenyAll + guest-path ProcessSpec builder — lift it, change the engine argv to one-shot `llama-cli`.
- The CH adapter's `snapshot`/`restore` + clone-safety IS the state-recovery primitive.
- The ledger's `with_sandbox_adapter_id` / `with_sandbox_internal_id` builders ARE the wiring.

</topic>

<topic id="selected-approach" status="active" wp="WP-KERNEL-004" summary="Ephemeral-exec-per-generate over hsk.cmd+serial; persistent daemon flagged follow-on">

# Decision: ephemeral-exec-per-generate (wave 1)

`SandboxModelRuntime.generate(req)` performs ONE `adapter.exec(handle, Command)` that boots a fresh CH microVM, runs the llama runner one-shot on the prompt, captures the completion from the framed serial output, and chunks it into `GeneratedToken`s. Rationale:

- The existing CH adapter's ephemeral `exec` path is REAL, tested, and already does kernel-cmdline injection + framed serial capture. Persistent-VM `exec` FAILS CLOSED today (`adapter.rs:816-836`) — it needs a vsock guest agent the operator has not built. Ephemeral is the only tractable real path for wave 1.
- It matches the field-standard serverless-inference cold-start pattern for one-shot generation.
- It is honest: model reload happens per `generate()` (no warm KV). This is documented, not hidden.

## Comms channel (wave 1): `hsk.cmd` + framed serial

The runtime builds a `Command{ argv: [runner, "--model", "/models/<gguf>", "-p", <prompt>, "-n", <max_tokens>, "-no-cnv", "--single-turn", "--no-display-prompt", "--log-disable"], timeout_ms }`. `fs_bind`s the GGUF host file's directory `ReadOnly` under `/models` (binds are baked into the initramfs at boot) and the runner if it is not already in the guest rootfs/PATH. `net_policy(DenyAll)`. The CH adapter base64-encodes the argv onto `hsk.cmd=`, boots the VM, and returns `ExecResult.stdout` = the runner's completion (stdout only, because `--no-display-prompt` + `--log-disable`).

## Token streaming (wave 1): capture-then-chunk, honest single terminal token

`generate()` is sync and returns a `TokenStream`. The OS thread does ONE blocking `Handle::block_on(adapter.exec(...))` (or a small runtime), then:
- On success: chunk `ExecResult.stdout` (UTF-8, reusing the CLI bridge's `Utf8ChunkDecoder` discipline) into `GeneratedToken{ token_id, text, finish_reason: None }` pieces (e.g. word/whitespace or fixed-size chunks — honest, NOT real per-token logprobs), then one terminal `GeneratedToken{ finish_reason: Some(Stop) }`. If `exit_code != 0`, emit an honest `Err(GenerateError(...))` item (carry stderr/exit code), NEVER an empty stream.
- On `adapter.exec` error: `tx.send(Err(ModelRuntimeError::GenerateError(format!("sandbox exec failed: {e}"))))`.
- `cancel()` flips a `runtime_cancel` token AND calls `adapter.kill(&handle, Signal::Kill)` so the in-flight VM is torn down; the terminal token is `Cancelled`.

**Honesty flag in code + report:** because exec is whole-completion, the per-token stream is a post-hoc chunking of a captured string, not live decode. This is a deliberate wave-1 simplification; real streaming needs a persistent in-VM serial daemon (below).

## Worktree binding + state recovery (wave 1 minimum)

- A `WorktreeVmRegistry` (or a field on the runtime) maps `worktree_id -> persistent ProcessHandle`. For wave 1, the snapshot/restore path is made REACHABLE and tested against the fake adapter: `snapshot(worktree_id)` looks up the handle and calls `adapter.snapshot(&handle) -> SnapshotRef`; `restore(worktree_id, &SnapshotRef)` calls `adapter.restore(&snap) -> ProcessHandle` and rebinds it. The CH adapter's clone-safety + per-restore isolation is reused unchanged.
- FLAGGED FOLLOW-ON: actually serving `generate()` FROM a persistent snapshot-restored VM (warm model, no reload) requires the persistent-`exec` guest agent. Wave 1 keeps generate() on ephemeral exec and proves snapshot/restore wiring separately. This split is honest and matches the adapter's current capability boundary.

# Rejected options

- **Persistent in-VM `llama-server` + HTTP** — REJECTED: CH VM has no network device (`adapter.rs:1014`); HTTP is impossible without a tap/virtio-net bind that fails closed today.
- **Persistent in-VM serial daemon (one VM, many prompts over serial, real per-token streaming)** — DEFERRED to follow-on: the CH adapter's persistent `exec` fails closed pending a vsock/serial guest agent; building that agent is its own work item. Flagged, not done in wave 1.
- **vsock guest agent (Cocoon/Kuasar-style)** — DEFERRED: the cleanest long-term channel and the right warm-start path, but it is net-new guest-side code outside this wave.
- **Reusing `sandbox_binding.rs` `SandboxRoutedModelAdapter` as the runtime** — REJECTED: it is a `ModelAdapter` returning a synthetic `response_text` via `spawn`, not a streaming `ModelRuntime` via `exec`. Lift only its ProcessSpec builder.
- **`candle` in-VM** — out of scope; wave 1 is `LlamaCpp` only, gated on `RuntimeBinding::LlamaCpp`.

</topic>

<topic id="risks-and-mitigations" status="active" wp="WP-KERNEL-004" summary="Risks, plausible failures, mitigations, hardening">

# Risks / failure scenarios / mitigations

1. **Live path silently faked as passing** (the cardinal sin). Mitigation: the live CH end-to-end is a single `#[ignore]`-d test gated behind an env var (e.g. `HANDSHAKE_CH_LIVE=1` + the CH bins), mirroring `pty.rs:565-577`; all headless tests use the injected FAKE adapter; the runtime + report state the boundary explicitly.
2. **Memory undersizing** — a 3B-Q4 GGUF + KV needs ~2-4 GB; default `HANDSHAKE_CH_MEMORY_MIB` may be too small and the runner OOM-kills. Mitigation: set `ResourceLimits.memory_bytes` / the CH `_MEMORY_MIB` env from the model config; document the 0.56 GB/1B-at-Q4 rule + 1-2 GB overhead; surface an honest exec error (non-zero exit + stderr) rather than an empty stream.
3. **Boot + reload latency per generate** — ephemeral exec reloads the model every call (cold start each time). Mitigation: document it; flag the persistent-VM/snapshot warm-start follow-on; keep `Command.timeout_ms` generous and configurable.
4. **GGUF not under the bind root / path translation wrong** — the runner sees `/models/...`; a mismatch yields a runner "file not found". Mitigation: reuse `sandbox_binding.rs` guest-path translation + bind-root validation; bind the GGUF's containing dir `ReadOnly`; assert the guest path in the unit test.
5. **Stderr noise / prompt echo polluting the captured completion.** Mitigation: `--no-display-prompt` + `--log-disable` + capture stdout only (CH `ExecResult.stderr` is empty by design — `adapter.rs:964`); unit test asserts captured text == fake completion exactly.
6. **Cancel leaks a live VM.** Mitigation: `cancel()` calls `adapter.kill` AND flips the runtime token; ephemeral children are also reaped by the adapter's `kill_on_drop`; unit test asserts `adapter.kill` was called.
7. **Ledger sandbox fields left None** (the explicit gap to close). Mitigation: `record_start` calls `with_sandbox_adapter_id(adapter.capabilities().adapter_id)` + `with_sandbox_internal_id(handle.sandbox_internal_id)`; unit test asserts both are non-None on the START row.
8. **Concurrent restore clobber / clone identity duplication.** Mitigation: NONE needed in new code — the CH adapter's `restore` already enforces TOCTOU-safe single-live-clone reservation (`adapter.rs:1252-1277`); wave 1 reuses it and the fake adapter mimics the contract.
9. **Non-sandbox spawns regressed by the new branch.** Mitigation: the sandbox branch is keyed strictly on `provider in {None,Local}` + `RuntimeBinding::LlamaCpp` + `isolation_tier == Some(Tier3Microvm)`; all other paths fall through unchanged; a regression test keeps a plain Local+LlamaCpp spawn on `create_local_llama`.
10. **`generate()` blocking the tokio runtime.** Mitigation: copy the CLI-bridge discipline exactly — the blocking `adapter.exec` runs on a dedicated OS thread, never a tokio worker; the async side only drains the mpsc via `unfold`.

</topic>

<topic id="validation-plan" status="active" wp="WP-KERNEL-004" summary="Headless fake-adapter tests + the gated live test + build gates">

# Headless tests (FAKE injected SandboxAdapter)

A `FakeSandboxAdapter` implementing `SandboxAdapter` with scripted `exec` results (success completion / non-zero exit / error), a recorded last-`Command`, a `kill`-called flag, and scripted `snapshot`/`restore`. Tests:

1. **ProcessSpec/Command correctness** — `generate()` (or the spec builder) produces a `Command` whose argv contains the runner, `--model /models/<gguf>`, `-p <prompt>`, `-n <max_tokens>`, the GGUF bind is `ReadOnly`, `net_policy == DenyAll`, `trust_class` set, Tier3 selected.
2. **Stream order** — fake exec returns a known completion; the drained non-terminal tokens concatenate to it in order, terminal `finish_reason == Stop`.
3. **Honest exec failure** — fake exec returns non-zero exit / `Err`; the stream yields an `Err(GenerateError(..))` item and is NOT silently empty.
4. **Cancel** — cancel mid-generate => `adapter.kill` was called and the terminal is `Cancelled`.
5. **Factory routing + ledger fields** — a `SpawnRequest` with `isolation_tier == Some(Tier3Microvm)`, `Local`+`LlamaCpp`, routes to the sandbox runtime; the recorded START row has `sandbox_adapter_id` AND `sandbox_internal_id` non-None. A control spawn without Tier3 stays on `create_local_llama` (sandbox fields None).
6. **Snapshot/restore wiring** — `snapshot(worktree)` calls `adapter.snapshot` and returns the `SnapshotRef`; `restore(worktree, snap)` calls `adapter.restore` and rebinds the handle; assert the fake observed both.

# Live operator-desktop test (`#[ignore]`-d)

One `#[ignore]` test gated on a real KVM/WSL/CH host (env `HANDSHAKE_CH_LIVE=1` + `HANDSHAKE_CH_BIN/_REMOTE_BIN/_KERNEL/_INITRAMFS/_BUSYBOX/_WORK_DIR` and the operator's GGUF + runner under the sandbox env). It builds the REAL CH adapter, runs `SandboxModelRuntime.generate` on a tiny prompt, and asserts a NON-EMPTY completion. Documented run: `cargo test -p handshake_core sandbox_runtime -- --ignored` on the operator desktop. NEVER runs/passes headless.

# Build / test gates

- `cargo build --manifest-path app/src-tauri/Cargo.toml --lib` => EXIT 0.
- `cargo build` + `cargo test` for `handshake_core` covering `sandbox_runtime`, `production_factory`, `process_ledger` (the fake-adapter suite above).
- The live CH test stays `#[ignore]`.
- Integrate phase owns `app/src-tauri` wiring (thread `sandbox_registry` into the `ProductionModelSessionFactory` constructor + app state). Leave changes UNCOMMITTED; the orchestrator reviews + commits.

</topic>
