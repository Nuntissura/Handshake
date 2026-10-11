---
file_id: BUILDER-BRIEF-HSK-STUDIO-SCORE-PLUGIN
file_kind: builder_brief
updated_at: 2026-10-11T04:40:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-score-plugin
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-SCORE_PLUGIN` / key `score_plugin` / crate `hsk-studio-score-plugin`; foundation MT `MT-35560` (read `implementation_steps` 1-4; depends_on MT-35541 score). Write scope `crates/hsk-studio-score-plugin/**` only.
- Responsibility (verbatim): "Optional native plugin hosting provider; scoped process/lifetime/timeout and foreign resource budgets separate from pure DSP."
- Foundation capability: "Actual selected plugin host processes bounded preallocated blocks, reclaims resources off callback and reports crash/timeout safely."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-score`, `hsk-studio-observe`. Needs `score` => wave 2.
- Ownership boundary (`plugin_owner` in STUDIO_MODULES + `backend-redteam/plugin-platform-decision.json` decision): the Handshake-wide plugin platform (5.1.6/11.8) owns catalog, consent, scheduler, DB; ScorePlugin is ONLY the native-audio domain adapter (CLAP/VST3, thread affinity, bounded discovery/isolation, lifecycle). Do NOT create a Studio plugin registry/catalog/permission store; the shared lifecycle is a planned prerequisite, not existing support - so this crate exposes a host-neutral provider port and takes grants/approved content-hashed paths from the caller.
- Forbidden: handshake_core/native/SurrealDB; WASM runtimes (not this module); in-process loading/probing of foreign plugin code in the host process (PLATFORM-08: "never silently fall back to unsafe in-process probing"); ambient directory scans; raising vendor editor windows (PLATFORM-18); native crates reachable from default features; a second plugin platform.
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[features] default = []`, `[lib] path="src/lib.rs"`, `[[example]]`s, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`.
- POLICY BLOCKER (lead/steward): probe `pure_registry_name` rejects `*-sys` registry names: `clack-host 0.2.0` depends on `clap-sys 0.5.0`, and `libloading` (ISC) is needed for dynamic load => the CLAP backend needs a scoped exemption (the packages are declaration-only/loader crates, but the policy is name-based). Slice 1 is buildable without them.
- Prep refs: `backend-redteam/plugin-platform-decision.json` (`required_controls` PLATFORM-01..19, esp. 04 admission, 08 native discovery = executable code in isolation, 09 realtime rules + versioned state capture, 10 update/uninstall vs borrowed code, 12 spawn/cleanup ownership ("an error result never implies no live child"), 14 native providers need OS isolation + content-hashed approved paths, 15 per-instance/per-phase state outcomes, 17 editor windows separate from DSP), `backend-redteam/plugin-host-research.json` (`selected_approach`; SoundCraft `clap-host/src/ffi.rs`, `vst3-host/src/ffi.rs` findings), `media-team/preparation.json` -> `module_proposals.score_plugin`. Reference source (MIT OR Apache-2.0, case study; uses `libloading` + unsafe FFI directly): `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/soundcraft-eac0edd896cd6bdf27c0cdd0f9f9158c8956248a/crates/{clap-host (3065 LOC), vst3-host (4072), au-host, mix}`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-ARC-021 (l.251): realtime providers version immutable plugin-state snapshots prepared OFF callback, declare complete capture or explicit partial/unavailable state, retire foreign objects on the correct host thread within bounded capacity; export/bounce receipts report exact requested vs actual sample counts and executed save/publication mode; unreported stale plugin state is a defect. STU-ARC-013 (l.221): callbacks never allocate/lock/decode/save plugin state/retire foreign graphs. STU-ARC-008/012: bounded admission incl. process count and device/plugin allocations; permits held until native quiescence; partial acquisition releases before waiting. STU-ARC-015: unavailable provider = typed failure, never silent. STU-QUIET-001/004: no foreground windows; Studio-spawned processes owner-tagged and reclaimed (STU-PAR-005: every spawned process registers with the process-lifecycle ledger and is reclaimed on close/failure/staleness).
- STU-FX-134..137 (l.18436-18454): audio effects as `StudioLiveFilter` stack entries with parameters in the audio category; plugin processors (VST3/AU/CLAP) are an effect source later. Later: STU-IO-189 (.sesx plugin state/effect graph losses), plugin parameter automation, editor embedding (separate host-matrix entry), MT corpus plugin feature MTs (UNVERIFIED).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (default build pure Rust, `#![forbid(unsafe_code)]`; ~900-1200 LOC; native only behind a feature in the WORKER)
Reuse (verified): `hsk_studio_accord::{CancellationToken, ActorContext, DomainId, ValidationError}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`; from `hsk-studio-score` (read once landed): `ChannelLayout`, block/sample-position types.
Architecture decision (smallest real provider that satisfies "separate process + timeout + crash reclaim + no in-process fallback"): the HOST crate never loads foreign code. Foreign plugins run only inside a short-lived WORKER process we spawn ourselves; the host speaks a bounded framed protocol over the child's stdio pipes. Native CLAP lives only in the worker behind feature `clap`.
1. Port + lifecycle (off-callback types): `pub trait PluginProvider: Send + Sync { fn probe(&self, p:&ApprovedPlugin, d:Deadline) -> Result<PluginDescriptor, PluginError>; fn activate(&self, cfg:&ProcessConfig) -> Result<Box<dyn PluginInstance>, PluginError>; }`, `pub trait PluginInstance { fn process(&mut self, input:&Block, out:&mut Block, d:Deadline) -> Result<ProcessReceipt, PluginError>; fn capture_state(&mut self, d:Deadline, max_bytes:u64) -> Result<StateCapture, PluginError>; fn retire(self: Box<Self>) -> RetireReport; }`; `ApprovedPlugin{path, content_sha256:[u8;32], format:Clap|Vst3, arch, abi_version}` (no path without a caller-supplied hash and `PluginGrant`; ambient scan does not exist); `enum LifecycleState{Discovered,Probed,Active,Processing,Draining,Retired}` with checked transitions (`IllegalTransition`). `ProcessReceipt{frames, latency_samples, state_version, provider_identity, completeness:Complete|Partial|Unavailable, safe_output:bool}`; `PluginError::{Unauthorized, HashMismatch, AbiMismatch, ProbeTimeout, Crashed{signal_or_code}, Timeout, StaleState, BudgetExceeded, Canceled, Protocol}`.
2. Framed worker protocol (pure Rust, both sides): header `{magic:u32, version:u16, kind:u8, len:u32}` + payload; `len <= Limits.max_frame_bytes` checked BEFORE allocation; kinds Hello/Probe/Configure/Process/CaptureState/Shutdown + replies; blocks are interleaved/planar f32 with explicit frame/channel counts and `start_sample` (unity-checked like score). Reject unknown kinds, short reads, oversize, version skew.
3. Process-isolated host `IsolatedHost`: `spawn` via `std::process::Command` (piped stdio; on Windows `CommandExt::creation_flags(0x0800_0000)` = CREATE_NO_WINDOW so no console/foreground window; null stdin otherwise), child registered in a host-owned `ProcessLedgerEntry{owner_session, started_at, pid}` (STU-PAR-005 shape; real ledger is host-side), per-call `Deadline` enforced with a reader thread + channel `recv_timeout`; on timeout/crash `kill()` + `wait()` (we own this child - allowed) and report `ProcessReceipt{killed, reaped}`; `Drop` reaps; failed spawn/ledger step after child creation still kills+reaps (PLATFORM-12). Admission: `max_workers`, `max_in_flight_bytes` via RAII permits released only after reap.
4. Reference worker `examples/score-plugin-worker.rs` (or `[[bin]]` if the steward's probe allows; UNVERIFIED): pure-Rust gain processor speaking the protocol, with test flags `--crash-on-block N` (abort), `--hang-on-block N`, `--latency L`; proves protocol/isolation/timeout/reclaim WITHOUT claiming CLAP (MT-35560: "mock host does not prove CLAP/VST3" - this is explicitly the lifecycle proof, labelled so in `DESCRIPTOR`).
5. CLAP backend in the worker, feature `clap` (default OFF): `clack-host 0.2.0` (MIT OR Apache-2.0; deps clack-common, clap-sys; optional libloading; updated 2026-09-12) loads the `.clap` DLL ONLY if its SHA-256 equals `ApprovedPlugin.content_sha256`, activates at the negotiated rate/block, processes ONE f32 block, returns output + latency + state version; compare with an independently known input/output (e.g. gain 0.5 plugin). No CLAP/VST3 plugin is installed on this machine (checked `C:/Program Files/Common Files/{CLAP,VST3}`): the fixture must be a tiny `clack-plugin 0.2.0` gain cdylib in a standalone fixture crate OUTSIDE the workspace (own target dir under the artifact root) - validator-time proof; without it the capability is `NOT_PROVEN`. VST3 (`vst3 0.3.0`, MIT OR Apache-2.0, coupler-rs) is later scope.
`DESCRIPTOR` JSON + `report(...)` into `Observe` (Success/Canceled/Failure(Unavailable|Loss|Validation)) + example host consumer `score-plugin-consumer` (`--descriptor`, `--worker PATH --gain 0.5 --frames 256 [--crash-on N] [--hang-on N] [--deadline-ms 500]`).

</topic>

<topic id="deps" status="active" summary="third-party deps (crates.io API 2026-10-11)">

## Deps
- Slice 1 (cap. 1-4): none (std `process`, `io`, `sync::mpsc`). Pure-Rust.
- Cap. 5 (worker-only, feature `clap`, needs probe exemption + dep-request): `clack-host 0.2.0` (MIT OR Apache-2.0; maintained, updated 2026-09-12; **pure-rust: NO** - FFI to foreign plugin code via `clap-sys 0.5.0` MIT/Apache, declaration crate, updated 2025-01), `libloading 0.9.0` (ISC; deps cfg-if, windows-link; ~127M recent dl; dynamic loading = unsafe FFI), `sha2 0.10.9` force-soft (pinned, pure-rust) for the content hash gate. Later: `vst3 0.3.0` (MIT OR Apache-2.0, deps com-scrape-types), `clack-plugin 0.2.0` for the test fixture only.
- Rejected: in-process hosting, `rack`/`vst-rs` (stale or in-process), WASM runtimes (separate platform decision), any crate that scans plugin directories ambiently.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-score-plugin --tests` (then clippy, focused test; the worker must be built before the host test runs - note it in the test via `cargo build --example` ordering or a `[[bin]]`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`; every spawned child is ours and is verified reaped; nothing audible, no windows)
1. `worker_gain_block_exact`: spawn reference worker, process a known 256-frame stereo block at gain 0.5: output equals `input*0.5` exactly, `frames == 256`, latency echoed, `state_version` bound to the instance identity; second block advances `start_sample` continuity.
2. `crash_and_timeout_are_contained`: `--crash-on-block 1` => `Crashed` with `reaped == true`; `--hang-on-block 1` with 300 ms deadline => `Timeout`, `killed && reaped`, returns within 2x deadline; `Child::try_wait` shows no live child; safe silence output flagged `safe_output`.
3. `unauthorized_and_mismatch_rejected`: path without grant/hash => `Unauthorized` before any process is spawned; wrong SHA-256 => `HashMismatch`; abi/arch mismatch => `AbiMismatch`; oversize/short/garbage frames from a fake worker => `Protocol` with the child killed.
4. `lifecycle_and_stale_state`: illegal transitions (`process` before `activate`, after `retire`) => `IllegalTransition`; `capture_state` returns versioned `Complete/Partial/Unavailable`; a block processed under an older `state_version` is refused (`StaleState`); retire releases all permits (in-flight bytes and workers back to 0) only after reap.
5. `admission_and_cancel`: `max_workers = 1` => second `activate` is `BudgetExceeded` immediately (no wait); canceled token mid-process => child killed+reaped, `Canceled`, no partial output published; `report()` emits one terminal frame per outcome.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No in-process plugin loading, no realtime audio-callback bridge (shared-memory/ring handoff to the worker is the next step and must follow STU-ARC-013), no VST3/AU in slice 1, no editor/UI embedding, no plugin catalog/consent/install/update/uninstall (Handshake-wide platform), no WASM/WIT extensions, no parameter automation, no MIDI.
- Wave: 2 (needs `score`); native CLAP additionally blocked on the probe-policy exemption and a plugin fixture. Report READY_FOR_VALIDATION for what is proven; CLAP stays `NOT_PROVEN` until a real plugin run.

</topic>
