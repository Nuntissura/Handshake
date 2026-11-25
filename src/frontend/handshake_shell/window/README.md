# handshake_shell_window

Native windows and the event loop of the kernel-owned shell (WP-KERNEL-012 MT-214 slice 1). Typed commands in, typed
events out. No Vulkan, no IME composition, no accessibility tree: those are the surface, input and accessibility crates
of later slices. Windows x64 first; the same code compiles for Windows ARM64. On other systems only the data types
exist.

The contract is in the crate documentation (`src/lib.rs`): how a host uses it, who may do what, wakes, failure modes.

## Files

| Path | Holds |
| --- | --- |
| `src/geometry.rs`, `monitor.rs` | scale, logical and physical sizes, placement |
| `src/event.rs`, `input.rs`, `keys.rs`, `command.rs` | the typed interface |
| `src/origin.rs`, `policy.rs` | who caused something, and who may move focus |
| `src/queue.rs`, `frame.rs`, `wake_slot.rs` | event order and bounds, frame demand, when a wake is posted: decisions without the operating system |
| `src/host.rs`, `wake.rs`, `token.rs`, `clipboard.rs` | `run`, `Control`, wake lanes, native window token, clipboard |
| `src/platform/` | winit and Win32, all unsafe code, nothing public |

## Why winit and Win32

MT-214 S1 prescribes winit 0.30.13 kept private, and asks why raw Win32 was or was not needed. Read in the source of
winit 0.30.13 (`src/platform_impl/windows`), and in `hsk-studio-native-host` at assets `d32d0574`, which uses winit the
same way.

winit does, and is used for: window class and creation (`window.rs` `init`, hidden with `with_visible(false)`),
keyboard layouts, dead keys and text (`keyboard.rs`), pointer, wheel, touch and pen messages, `WM_DPICHANGED` with
per-monitor v2 awareness (`event_loop.rs` 2216, `SWP_NOZORDER | SWP_NOACTIVATE`), cursor shapes, and the outer message
pump (`MsgWaitForMultipleObjectsEx`, no polling).

Win32 is needed beside it:

| Need | What winit 0.30.13 does | Here |
| --- | --- | --- |
| Restore without activation | `SW_RESTORE` (`window_state.rs` 366, 376) | `place.rs`, `SW_SHOWNOACTIVATE` |
| Minimise without moving focus | `SW_MINIMIZE`, which activates the next window (375) | `SW_SHOWMINNOACTIVE` |
| Show at a chosen Z place | `SW_SHOW` or `SW_SHOWNOACTIVATE` (330), no Z order; any later flag change of a shown window shows it again, with `SW_SHOW` when it was created active | `SetWindowPos` with place and no-activate in one call |
| Show with activation | `focus_window` sends an Alt key with `SendInput` and calls `SetForegroundWindow` (`window.rs` 1538-1575): banned | `SW_SHOW` and Z order only, on the Operator's authority; the operating system decides about the foreground |
| `WS_EX_NOACTIVATE` that stays | rewrites the extended style on flag changes; the Studio host had to put it back (`style_restores`) | `WM_STYLECHANGING`, `WM_WINDOWPOSCHANGING` and `WM_MOUSEACTIVATE` in front of winit's procedure |
| Wakes | `send_event` allocates, ignores a failed `PostMessage`, and its event is held back while any callback runs (`runner.rs` `should_buffer`) | a posted message to a window of ours, with a pool timer that posts again until it is dispatched |
| Host code that opens a dialog or menu | runs inside a callback, so every event and wake is held back meanwhile | callbacks only queue; the host is called from a window procedure |
| Origin, device, time | not reported | `GetCurrentInputMessageSource`, `GetMessageTime` per message |
| Precision touchpad | a fractional wheel delta | the same delta in whole 1/120 detents, and `IMDT_TOUCHPAD` |
| File drop | one path at a time, no position, needs COM | `WM_DROPFILES`: all paths and the position |
| Sleep, session end, display change | not reported | a hidden top-level service window |
| Clipboard, work areas, wheel settings | none | Win32 |

Not needed in this slice, for the next ones to know: winit's IME is IMM32 only (`ime.rs`, `WM_IME_*`), off until asked;
it has no TSF. AccessKit's Windows adapter subclasses a window handle that was not shown yet, so it attaches through
`NativeWindowToken` on `WindowEvent::Created`.

Costs of winit that stay: one loop per process, so one loop per test binary; it makes a visible top-level window without
extent for itself, which `run` puts at the bottom of the Z order; it registers for raw input, which `run` turns off; a
stray `WM_DEADCHAR` without a key before it panics (`keyboard.rs`), which ends the loop.

To replace winit: `platform/app.rs`, `platform/translate.rs` and the handle in `platform/window.rs`. No other file names
it (`tests/test_public_surface.rs`).

## Extension points

| For | Where |
| --- | --- |
| Surface (S2) | `NativeWindowToken` on `Created`; `FrameDue` and `frame_done(Presented)`; `FrameProgress::waiting` is the present-progress signal |
| IME (S3) | winit's IME events in `translate.rs` become new variants of `WindowEvent` (non-exhaustive); `InputStamp::model_action` for steered input |
| AccessKit (S4) | `NativeWindowToken` on `Created`, before the first `Show` |
| Heartbeat (MT-215) | a `WakeLane` behind `EventLoopWake`; `on_wake` only with a `&DispatchedWake` |

## Failure codes

`Refusal::code`, `CreateFailure::code`, `LoopError::code`, `WakeRefused::code`, `ClipboardError::code`: `window.*`,
`window.create.*`, `loop.*`, `wake.*`, `clipboard.*`. They carry no content. `Event` and commands print without titles,
typed text and paths.

## Tests

`cargo test --locked` in this folder (validator only). Tests with a window run their loop on a private Win32 desktop that
receives no input, while a watch on the Operator's desktop fails the test if a window of the process ever is in the
foreground, active or focused there (`tests/support`). One loop per test binary.

| Target | Rule |
| --- | --- |
| unit tests | policy, queue order and bounds, frame ledger, wake slot, key and wheel mapping, drop parsing, placement |
| `test_geometry` | scale and size arithmetic |
| `test_focus_ban`, `test_public_surface` | source scans, with a control that they find what they look for |
| `test_wakes` | wakes on the loop's thread, not early, through a full queue, in a nested loop, in a dialog, during shutdown, refused after the end |
| `test_menu_loop` | wakes inside the system's menu loop |
| `test_quiet_window` | a quiet window through every command and against direct calls |
| `test_operator_window` | activation and clipboard need the Operator; control that an activation is seen |
| `test_scale_change` | logical size exact over many scale changes; a requested frame is due |
| `test_host_panic` | a panicking host leaves no window and is not called again |

Left out, because they cannot run quietly: the move and size loop (needs a pressed button or moves the cursor), real
keyboard, pointer, wheel and touchpad input (needs injection), dragging files in, the Operator's clipboard, real monitor
changes, sleep and session end. They need the Operator (HBR-QUIET-004) or MT-202.
