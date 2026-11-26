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
| Shutdown drains (the heartbeat beats through it), settles claims, and only a settled one is marked in the ring and acknowledged by the watcher | `ProgramApp::ended`, `src/outcome.rs` (`shutdown_settled`, `may_acknowledge`, `conclude`) |
| Crash behaviour untouched: no panic hook, exception filter or process exit in this crate | `tests/source_scan.rs` |

Exit codes: 0 clean, 2 usage, 3 unsupported, 4 device unavailable, 5 no window, 6 loop failed, 7 no device authority, 8
shutdown unsettled, 9 watcher not acknowledged, 10 heartbeat refused (`ExitKind`).

Missing from other MTs (nothing was invented): `device.gpu.shell` in the capability registry (MT-219 is not published);
the kernel issuer's reviewed trust-port binding (NATIVE-RENDER-BOOT-007 defines semantics, no API): `IssuedAuthority` holds
an uninhabited type until it exists; Core's installation-wide quota ledger (the test binaries use their own policy); a
toolkit-free Palmistry launcher (`palmistry_launch.rs` is in the egui host), so the watcher seam `ShutdownWatcher` is
passed `NoWatcher` and the ring is a v1 writer from `--diag-ring`, without the launcher's v2 nonce.

Not yet (MT-214 S6): more than one window, device loss and rebind, the readable unavailable state, the Operator-class
root window and launch activation. Windows with `--diag-ring` and `NoWatcher` show `watcher=not_connected` in the end line.

Tests: `tests/startup_gate.rs` (no device, any machine), `tests/source_scan.rs`, `tests/shell_program.rs` (real device,
private desktop, own process; skips without a device unless `HANDSHAKE_REQUIRE_VULKAN_GPU` is not 0).
