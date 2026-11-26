# handshake_shell_window

Native windows and the event loop of the kernel-owned shell (WP-KERNEL-012 MT-214 slice 1). Typed commands in, typed
events out. No Vulkan, no accessibility tree: those are the surface and accessibility crates. The input method's
composition arrives here as `WindowEvent::Ime`, with `WindowCommand::AllowIme` and `SetImeArea` (slice 3); the input
crate (`../input`) routes it to text targets, and nothing here keeps it. Windows x64 first; the same code compiles for Windows ARM64. On other systems only the data types
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
| A stray `WM_DEADCHAR` | `keyboard.rs` 161 unwraps a key press that is not there and panics; the panic ends the loop. Its look at the queue on a key down (115, `PeekMessageW`) dispatches messages other threads sent, so a sent dead key arrives before it opened the key (124) | `platform/key_gate.rs`: a keyboard message another thread sent never reaches winit; of the others, a dead key without an open key is kept from it, following winit's own decision and taking it over when winit returns, and so is any dead key that comes while winit still handles a keyboard message of the window (MT-214 W6, R2-N2, E3-F5) |
| A key or wake taken out of the queue, told from one another thread sent | `InSendMessageEx` says "sent" for every message while the thread handles one another thread sent, also for keys a loop inside it dispatches (E3-F5) | `platform/retrieval.rs`: a `WH_GETMESSAGE` hook of the loop's thread (`SetWindowsHookExW(WH_GETMESSAGE, .., 0, GetCurrentThreadId())`) notes the newest message `GetMessage` or `PeekMessage` took out and whether the thread was inside a sent message then; it reads and changes nothing, and sees this thread's queue only (learn.microsoft.com: GetMsgProc) |
| Wakes | `send_event` allocates, ignores a failed `PostMessage`, and its event is held back while any callback runs (`runner.rs` `should_buffer`) | a posted message to a window of ours, numbered and masked per lane, with a pool timer that posts again until one is dispatched; refused posts back off to one per 8 s, and after four unanswered posts the next waits until the loop dispatches a wake or answers a sent probe while no wake post waits (R2-N3, E3-F4: a thread waiting in `SendMessage` to another thread answers probes and takes nothing out of its queue); a dropped lane's number is used again with a new mask (R2-N4) |
| Host code that opens a dialog or menu | runs inside a callback, so every event and wake is held back meanwhile | callbacks only queue; the host is called from a window procedure |
| Origin, device, time | not reported | `GetCurrentInputMessageSource`, `GetMessageTime` per message |
| Precision touchpad | a fractional wheel delta | the same delta in whole 1/120 detents, and `IMDT_TOUCHPAD` |
| File drop | one path at a time, no position, needs COM | `WM_DROPFILES`: all paths and the position |
| Sleep, session end, display change | not reported | a hidden top-level service window |
| Clipboard, work areas, wheel settings | none | Win32; a write keeps the old content only when it is plain text alone (a marker of the clipboard history is a registered format, so it keeps nothing, E4-F4), up to 1 MiB, and puts it back if its own put fails (KEEP_PLAIN_TEXT_OR_NOTHING, `src/clipboard.rs`, R2-N6, E3-F2); the copy is made on a thread of its own on the loop thread's desktop and waited for 250 ms at most, since Microsoft documents no bound on a program's rendering of promised text: a hung program makes the write `clipboard.busy`, never the loop wait (E4-F3); a withdrawal empties only this process's own last write (E3-F3) |
| The Operator's close, told from anybody else's | `CloseRequested` for every `WM_CLOSE`, whoever sent or posted it | `window.rs`, `platform/closing.rs` and `policy::CloseWatch`: only a close message the system sends inside the close command it made of the Operator's own Alt+F4, close button or system-menu double-click on that window approves ACTIVATION_AFTER_REMOVAL (E3-F6): the command sent while the gesture's message is handled, or the one that handling posted, seen in the queue right after it (E4-F2) |

winit's IME is IMM32 only (`ime.rs`, `WM_IME_*`), off until asked; it has no TSF, and TSF input methods reach it through
the system's IMM32 compatibility. It reports a composition's start and end as `Enabled` and `Disabled`. AccessKit's Windows adapter subclasses a window handle that was not shown yet, so it attaches through
`NativeWindowToken` on `WindowEvent::Created`.

Costs of winit that stay: one loop per process, so one loop per test binary; it makes a visible top-level window without
extent for itself, which `run` puts at the bottom of the Z order and keeps from activation; it registers for raw input,
which `run` turns off.

Decisions: ACTIVATION_AFTER_REMOVAL (`policy.rs`, confirmed by the Operator 2026-11-26) sends the keyboard, after a
minimise, hide or destroy the Operator approved that left it nowhere, to the removed window's owner, else to the topmost
other Operator window, and never while another program is in front; a close request another program made approves
nothing (E3-F6). KEEP_PLAIN_TEXT_OR_NOTHING (`src/clipboard.rs`, R3-D1, E3-F2): a write copies the old clipboard only
when it holds plain text alone (no other format, so no history or cloud marker either, at most 1 MiB), within 250 ms
(E4-F3), and otherwise copies nothing and writes all the same, so a write the system refuses after emptying leaves the clipboard
empty and says `clipboard.not_restored`. The other choice, refusing such writes (`clipboard.cannot_keep`), is one
constant in `platform/clipboard.rs` and would make copies fail after the Operator copied in a browser, an office
program, an image editor or a password manager.

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
| IME (S3, done) | winit's IME events (IMM32 only) become `WindowEvent::Ime(ImeInput)` in `translate.rs`, stamped with the origin of the window's newest key (`window.rs` `origin_of`, as for text); `WindowCommand::AllowIme` and `SetImeArea(ClientRect)` call winit's `set_ime_allowed` and `set_ime_cursor_area` (physical, client-relative); `EventTime::from_nanos` and `TypedText::new` are public for recorded sequences and model input, and grant nothing; `InputStamp::model_action` for steered input |
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
| unit tests | policy (restore is geometry, activation after removal, the Operator's close gesture only through the command the system made of it, a gesture that made none lends nothing, a foreign close approves nothing), authority used once, queue order and loss position, frame ledger, wake slot (lost posts, refused posts back off, a hung loop gets four posts, forged numbers, yield), key gate (sent keyboard messages, stray dead keys, dead keys inside a key the library handles), how a message arrived, the clipboard policy against a clipboard that lists formats as the system does (only plain text asked for and kept, markers as registered formats, the limit, the other choice, a copy the clipboard changed since, withdrawal after a restore), key mapping against winit's own tables, every winit IME event to one change, wheel, drop parsing, key and IME events print no key or text |
| unit tests, `platform/quiet_tests.rs` | the guard against every sequence that activates a plain window (with that control, each one counted as refused), sections, minimise hand-off, lifetime; behind the foreground below its owner chain; topmost foreground; the keyboard after an approved removal; a close command in a real queue seen where it waits, and F4 alone through the system's own handling making none, so a close posted after it is nobody's (E4-F2) |
| `test_geometry` | scale and size arithmetic |
| `test_focus_ban`, `test_public_surface` | source scans, with a control that they find what they look for; the only hooks are the loop thread's own two, each for that thread; the only desktop binding in the crate's source is the clipboard keeper thread's, to the loop thread's own desktop |
| `test_wakes` | wakes on the loop's thread, not early, through a full queue, through five thrown-away posts (one more than go out before a sign), not from forged or sent messages (a genuine post sent back from another thread is refused and counted), not from the posts 200 dropped lanes left under the number in use again, four posts and no more while the loop waits in a SendMessage to a hung thread, in a nested loop, in a dialog, during shutdown, none after `finish`, refused after the end |
| `test_menu_loop` | wakes inside the system's menu loop |
| `test_sent_keys` | keys a loop inside a message another thread sent takes out reach the library and are not counted as sent; keys another thread sends are refused and counted |
| `test_clipboard` | on the unnamed window station of the logon session (no elevation; E4-F1), one clipboard test of the session at a time, emptied at the end, the Operator's clipboard sequence number unchanged: a write asks no program to render a promised format, but promised plain text alone; a program that hangs on that request makes the write busy within the bound, not the loop wait (E4-F3); withdrawal takes back only this process's own write |
| `test_quiet_window` | a quiet window through every command, against each direct call that activated it (each refused by the guard), against foreign dead keys, and against keyboard messages another thread sends |
| `test_operator_window` | activation and clipboard need the Operator, once; a model cannot move, size, restore or remove the Operator's window or the owner of its dialog; behind the foreground; the close request; a shell's minimise hands on to no window of ours; a close another program asked for lets a model destroy the window but hands the keyboard nowhere |
| `test_scale_change` | logical size exact over many scale changes; a requested frame is due; a hidden maximised window stays so |
| `test_host_panic` | a panicking host leaves no window and is not called again |

Left out, because they cannot run quietly: the move and size loop (needs a pressed button or moves the cursor), real
keyboard, pointer, wheel and touchpad input (needs injection), and with it the Operator's close gesture and the hand-on
after it inside the loop, dragging files in, the Operator's clipboard (the operating system's clipboard runs on the logon
session's unnamed window station), real monitor changes, sleep and session end. They need the Operator (HBR-QUIET-004) or
MT-202.
