# handshake_shell

The shell crate of MT-214 AC214-12: composition of the kernel-owned shell's crates and the toolkit-free host ports, with
no widget, pane or domain logic of its own. Windows x64 first; on other systems only the watches compile.

It holds the shell program of MT-214 S5 (below) and MT-215 slice 2, `ShellHeartbeat`: the UI heartbeat port (`../../handshake_host_ports/ui_heartbeat`)
bound to the window crate's wake lanes and the surface crate's present watches, one per loop. A frozen UI thread
reaches the diagnostics ring as a stale heartbeat; a stalled present as a typed stall event beside a heartbeat that
stays fresh (AC215-6). The internal diagnostics and the standalone Palmistry watcher read that ring. The manual is the
crate and module documentation (`src/lib.rs`, `src/heartbeat.rs`): how a host uses it, what a beat proves, what the
beat path waits for, failure modes.

## Files

| Path | Holds |
| --- | --- |
| `src/heartbeat.rs` | `ShellHeartbeat`: the port on one wake lane, the lane as the port's loop, one per loop |
| `src/watches.rs` | the present watches the heartbeat reads, bounded (`MAX_WATCHED_WINDOWS`) |
| `tests/heartbeat_loop.rs` | a real loop without a window: beats, a second heartbeat refused, foreign lanes, thrown-away posts, a blocked thread, an allocation-free beat path |
| `tests/heartbeat_present.rs` | the presenter on this machine's device: a revoked claim stalls the present while beats go on, one stall event, minimising recovers |
| `tests/source_scan.rs` | over every workspace member read from the manifests: one heartbeat publisher, one port, one ring writer; the crate's dependencies, the focus ban, loops on private desktops, ring files removed by their guard |

Every test keeps its own time limit and removes its ring file however it ends, through the port's shared test limits
(`../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs`).

## Proof that needs native x64 and a real desktop

The tests run on private desktops; that drivers present there, real monitors, occlusion, throttling, sleep and resume,
and Palmistry attached to the real process are proven on the x64 PC (MT-215 HP215-RUNTIME, MT-213 S9).

## The shell program (MT-214 S5)

`src/program.rs` composes the five crates into one process: the window crate's loop, the surface crate's presenter on the
central GPU owner, the input tracker, the accessibility hub and the heartbeat. It draws nothing of its own and holds no
widget, pane or layout (`tests/source_scan.rs` scans for it). Binary: `handshake_shell` (`src/main.rs`; a working name,
no canonical product name is verified). Test binary for a manual run: `examples/shell_program.rs`.

| Rule | Where |
| --- | --- |
| The device opens only under `device.gpu.shell` and the kernel issuer; without them a typed refusal, exit code 7, nothing opened | `src/authority.rs` (`run_product`), `src/main.rs` |
| A test or example binary says `product=false device_authority=none` in its first line | `DeviceAuthority::receipt`, `examples/shell_program.rs` |
| Missing or rejected device: typed end `device_unavailable` (4) with the owner's key | `src/program.rs` (`run_program`) |
| Shutdown drains (the heartbeat beats through it), settles claims, and only a settled one of a run with no typed failure is marked in the ring and acknowledged by the watcher | `ProgramApp::ended`, `src/outcome.rs` (`shutdown_settled`, `may_acknowledge`, `conclude`) |
| Crash behaviour untouched: no panic hook, exception filter or process exit in this crate | `tests/source_scan.rs` |

Exit codes: 0 clean, 2 usage, 3 unsupported, 4 device unavailable, 5 no window, 6 loop failed, 7 no device authority, 8
shutdown unsettled, 9 watcher not acknowledged, 10 heartbeat refused (`ExitKind`).

Missing from other MTs (nothing was invented): `device.gpu.shell` in the capability registry (MT-219 is not published);
the kernel issuer's reviewed trust-port binding (NATIVE-RENDER-BOOT-007 defines semantics, no API): `IssuedAuthority` holds
an uninhabited type until it exists; Core's installation-wide quota ledger (the test binaries use their own policy).

## The Palmistry session (MT-214 S7, AC214-11)

`src/palmistry.rs` is the shell's side of the watcher; the launcher is `palmistry_launch` (MT-215). The product binary
(after the gate, receipt first) calls `palmistry::start`: session id, lease (`HANDSHAKE_HOST_SCOPE_ID` from the
environment), the one version 2 ring (`--diag-ring PATH`, else the launcher's default path for the session), the watcher's
exe (`HANDSHAKE_PALMISTRY_EXE` or a sibling `palmistry.exe`), the one signed launch, and the `PalmistryHandshake` marker.
It prints one `HSK_SHELL_PALMISTRY state=... reason=... session=...` line, then runs the program with the ring and the
watcher, and drops the lease only after the program has returned. A refused start (exit 7) runs none of this. Any failure
of the session is a degrade (reasons `no_host_scope`, `no_lease`, `no_ring`, `exe_not_found`, `spawn_failed`, or state
`handshake_unconfirmed` for a running watcher whose signed handshake was not confirmed): the program runs without a
confirmed watcher, which is supplementary. `--diag-ring` now also names the ring a launched watcher reads.

| What | Behaviour |
| --- | --- |
| Clean, settled close | the ring's `Shutdown` marker, then the watcher is told (signed) and reaped; exit 0 when it acknowledged or none was connected (an unconfirmed watcher is reported not connected), 9 when a confirmed watcher did not acknowledge |
| Endings that skip the acknowledgement: no device (exit 4), a refused window (exit 5), a failed loop (exit 6), a refused heartbeat (exit 10), an unsettled close (exit 8), a panic | the watcher is left untold and records the parent's end as unexpected (Operator decision WP012-MT214-S7-NO-GPU-STOP-20261128: a crash record) |
| Before any window | `palmistry::start` runs before the device opens and before any window exists, on the thread that starts the program, and its signed launch waits for the watcher's handshake, bounded by the launcher's overall deadline of about 9 s (`palmistry_launch`, `HANDSHAKE_OVERALL_DEADLINE`); a slow or unresponsive watcher delays the first window by up to about 9 s, then the start degrades: the watcher that did not confirm is kept, reaped when the program ends and reported not connected (`NotConnected`), and the program runs without a confirmed watcher |
| Before the loop starts (device open, window creation) | not watched before the first heartbeat: Palmistry reads a process with no heartbeat yet as healthy, so a hang there is not detected |
| Crash | no panic hook, exception filter or crash client in this crate; the OS's handler stays; the watcher records the end without a Shutdown |
| `ManuallyDrop` | an adapter dropped without a question never lets the launcher's handle send a clean Shutdown |

Proof here is shape and fakes: the mapping, the order and the one-launch rules are tested without a process
(`tests/palmistry_watcher.rs`, `tests/palmistry_session.rs`, `tests/source_scan.rs`). The real handshake, acknowledgement
and the orphan check are live proofs by a separate agent (P8) and are not supplied by these tests.

Root window (WP012-SHELL-ROOT-WINDOW-20261127, "Operator window, closes app"): the product binary opens its root with
`ProgramConfig::operator_root`: `WindowClass::Operator`, activated once with the launch authority (`Authority::Launch`)
when first shown, and closing it ends the process. Detached panes and Settings stay quiet. Tests and examples keep the
quiet root (`run_program` makes a `TestBinary` quiet whatever its config says); a source scan fails otherwise. The launch
activation itself needs a real desktop. A start without a confirmed watcher shows `watcher=not_connected` in the end line.

Tests: `tests/startup_gate.rs` (no device, any machine), `tests/source_scan.rs`, `tests/shell_program.rs` (real device,
private desktop, own process; skips without a device unless `HANDSHAKE_REQUIRE_VULKAN_GPU` is not 0).
