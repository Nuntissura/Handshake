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
| No activation of a quiet window, by anybody | nothing; `SWP_NOACTIVATE` added in `WM_WINDOWPOSCHANGING` does not stop `SetWindowPos`, a show after a hide or a restore (MT-214 W1 probe) | `platform/guard.rs`: a CBT hook of the loop's thread (`SetWindowsHookExW(WH_CBT, .., 0, GetCurrentThreadId())`) refuses `HCBT_ACTIVATE` of a quiet window or one it owns, and of every window of the thread while `place` changes one; installed before winit's first window, removed after the last (learn.microsoft.com: CBTProc, SetWindowsHookExW, UnhookWindowsHookEx) |
| No taskbar button for a quiet window | `AddTab` for every window at creation and after Explorer restarts (`window.rs` 1249, `event_loop.rs` 2456) | `with_skip_taskbar` for the quiet class |
| A hidden maximised window across a scale change | asks for a size, unmaximises with `SW_RESTORE`, which shows and activates, then hides (`event_loop.rs` 2302, `window_state.rs` 364) | no size asked for a minimised or maximised window; the message runs inside the guard |
| A stray `WM_DEADCHAR` | `keyboard.rs` 161 unwraps a key press that is not there and panics; the panic ends the loop. Its look at the queue on a key down (115, `PeekMessageW`) dispatches messages other threads sent, so a sent dead key arrives before it opened the key (124) | `platform/key_gate.rs`: a keyboard message another thread sent never reaches winit (`InSendMessageEx`); of the queued ones, a dead key without an open key is kept from it, following winit's own decision (MT-214 W6, R2-N2) |
| Wakes | `send_event` allocates, ignores a failed `PostMessage`, and its event is held back while any callback runs (`runner.rs` `should_buffer`) | a posted message to a window of ours, numbered and masked per lane, with a pool timer that posts again until one is dispatched; refused posts back off to one per 8 s, and after four unanswered posts the next waits until the loop answers a sent probe or dispatches a wake (R2-N3); a dropped lane's number is used again with a new mask (R2-N4) |
| Host code that opens a dialog or menu | runs inside a callback, so every event and wake is held back meanwhile | callbacks only queue; the host is called from a window procedure |
| Origin, device, time | not reported | `GetCurrentInputMessageSource`, `GetMessageTime` per message |
| Precision touchpad | a fractional wheel delta | the same delta in whole 1/120 detents, and `IMDT_TOUCHPAD` |
| File drop | one path at a time, no position, needs COM | `WM_DROPFILES`: all paths and the position |
| Sleep, session end, display change | not reported | a hidden top-level service window |
| Clipboard, work areas, wheel settings | none | Win32; a write copies what is on the clipboard first and puts it back if its own put fails (R2-N6) |

Not needed in this slice, for the next ones to know: winit's IME is IMM32 only (`ime.rs`, `WM_IME_*`), off until asked;
it has no TSF. AccessKit's Windows adapter subclasses a window handle that was not shown yet, so it attaches through
`NativeWindowToken` on `WindowEvent::Created`.

Costs of winit that stay: one loop per process, so one loop per test binary; it makes a visible top-level window without
extent for itself, which `run` puts at the bottom of the Z order and keeps from activation; it registers for raw input,
which `run` turns off.

Decisions for the Operator to confirm (MT-214 R2-N5, R2-N6): ACTIVATION_AFTER_REMOVAL (`policy.rs`) sends the keyboard,
after a minimise, hide or destroy the Operator approved that left it nowhere, to the removed window's owner, else to the
topmost other Operator window, and never while another program is in front. A clipboard write copies the formats on the
clipboard before emptying it, which makes a program that promised a format render it, as reading does.

Window ghosting is left as the system has it: a hung Operator window can still be moved, minimised and closed through
its ghost. Whether the ghost of a quiet window can surface is not inspected; it needs a real desktop and a hang.

A window a `NativeWindowToken` still names when the loop ends abnormally is hidden and left until the thread ends,
instead of destroyed under its surface (`RunReport::windows_left`).

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

`cargo test --locked` in this folder (validator only). Tests with a window run on a private Win32 desktop that receives
no input. A watch on the Operator's desktop fails the test if a window of the process ever is in the foreground, active
or focused there; it guards the Operator and cannot see the private desktop. A second watch on the private desktop
fails the test if a window the test named is ever the active or focused window of its thread, with a control that it
sees an activation (`tests/support`). The private desktop has no foreground window; the thread's active window stands
for it, so the paths that need one run. One loop per test binary.

| Target | Rule |
| --- | --- |
| unit tests | policy (restore is geometry, activation after removal), authority used once, queue order and loss position, frame ledger, wake slot (lost posts, refused posts back off, a hung loop gets four posts, forged numbers, yield), key gate (sent keyboard messages, stray dead keys), clipboard put back after a failed write, key mapping against winit's own tables, wheel, drop parsing, key events print no key |
| unit tests, `platform/quiet_tests.rs` | the guard against every sequence that activates a plain window (with that control, each one counted as refused), sections, minimise hand-off, lifetime; behind the foreground below its owner chain; topmost foreground; the keyboard after an approved removal |
| `test_geometry` | scale and size arithmetic |
| `test_focus_ban`, `test_public_surface` | source scans, with a control that they find what they look for; the only hook is the guard's |
| `test_wakes` | wakes on the loop's thread, not early, through a full queue, through thrown-away posts, not from forged or sent messages (a genuine post sent back from another thread is refused and counted), through 200 lanes whose number is used again, in a nested loop, in a dialog, during shutdown, none after `finish`, refused after the end |
| `test_menu_loop` | wakes inside the system's menu loop |
| `test_quiet_window` | a quiet window through every command, against each direct call that activated it (each refused by the guard), against foreign dead keys, and against keyboard messages another thread sends |
| `test_operator_window` | activation and clipboard need the Operator, once; a model cannot move, size, restore or remove the Operator's window or the owner of its dialog; behind the foreground; the close request; a shell's minimise hands on to no window of ours; a close the Operator asked for hands the keyboard to the other Operator window |
| `test_scale_change` | logical size exact over many scale changes; a requested frame is due; a hidden maximised window stays so |
| `test_host_panic` | a panicking host leaves no window and is not called again |

Left out, because they cannot run quietly: the move and size loop (needs a pressed button or moves the cursor), real
keyboard, pointer, wheel and touchpad input (needs injection), dragging files in, the Operator's clipboard, real monitor
changes, sleep and session end. They need the Operator (HBR-QUIET-004) or MT-202.
