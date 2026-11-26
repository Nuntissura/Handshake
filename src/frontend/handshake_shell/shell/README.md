# handshake_shell

The shell crate of MT-214 AC214-12: composition of the kernel-owned shell's crates and the toolkit-free host ports, with
no widget, pane or domain logic of its own. Windows x64 first; on other systems only the watches compile.

So far it holds MT-215 slice 2, `ShellHeartbeat`: the UI heartbeat port (`../../handshake_host_ports/ui_heartbeat`)
bound to the window crate's wake lanes and the surface crate's present watches. A frozen UI thread or a present stall
reaches the diagnostics ring that the internal diagnostics and the standalone Palmistry watcher read. The manual is the
crate and module documentation (`src/lib.rs`, `src/heartbeat.rs`): how a host uses it, what a beat proves, failure
modes.

## Files

| Path | Holds |
| --- | --- |
| `src/heartbeat.rs` | `ShellHeartbeat`: the port on one wake lane, the lane as the port's loop |
| `src/watches.rs` | the present watches the heartbeat reads, bounded (`MAX_WATCHED_WINDOWS`) |
| `tests/heartbeat_loop.rs` | a real loop without a window: beats, foreign lanes, thrown-away posts, a blocked thread |
| `tests/heartbeat_present.rs` | the presenter on this machine's device: a revoked claim stalls the present, minimising recovers |
| `tests/source_scan.rs` | one heartbeat publisher, the crate's dependencies, the focus ban, loops on private desktops |

## Proof that needs native x64 and a real desktop

The tests run on private desktops; that drivers present there, real monitors, occlusion, throttling, sleep and resume,
and Palmistry attached to the real process are proven on the x64 PC (MT-215 HP215-RUNTIME, MT-213 S9).
