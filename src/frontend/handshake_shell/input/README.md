# handshake_shell_input

Typed input of the kernel-owned shell, with its origin and time, and the input method's composition as one event stream
per focused text target (WP-KERNEL-012 MT-214 slice 3; AC214-5, AC214-6). Its only dependency is the window crate
(`../window`); it has no unsafe code and calls no operating system function. The contract is in the crate documentation
(`src/lib.rs`).

## What it does

| Part | Rule |
| --- | --- |
| Inputs (`InputTracker::feed`) | Every input event of the window loop becomes one `InputEvent`: key, text, modifiers, pointer moved, entered, left, button, wheel (`WheelSource`: detents, finer wheel, precision touchpad), touch and pen contact, input method. Other events give none; the tracker reads windows, scales and focus from them. |
| Origin | `InputEvent::origin` is the loop stamp's (Operator device, model action, other program, not said), with its time and the stamp itself for `CommandSource::from_input`. Fields are private, so only this crate makes an event. A model's input (`InputTracker::model_input`) is stamped with `InputStamp::model_action`: never the Operator's. An Operator stamp older than one already seen is a copy: passed on as not attributed, without the stamp (`InputReport::stale_stamps`). |
| Keys | First press `Press`, every further press until the release `Repeat` (with or without the operating system's mark), `Release`. Dead keys (`KeyInput::is_dead`) type nothing; the next key's text carries the accent, as one `Text` with that key's stamp. When the window loses the keyboard, held keys are released; when events are lost (`Event::EventsLost`) or the window goes, held keys and pointer buttons are. A release this crate makes carries the press's origin, no stamp, and `InputEvent::synthetic`. |
| Input method | See below. |
| Model | A model may not move a window's text focus, turn composition off for its target, or send keys or text, while the Operator composes there (`OperatorComposing`, GUI-DESKTOP-041). Pointer and wheel input stays allowed. |

### The input method (AC214-6)

The window reports what the operating system's input method did (`WindowEvent::Ime`, from winit 0.30.13's IMM32
handling: a composition's start, its preedit with a byte cursor, its commits, its end). The tracker routes it to the
text target with the window's text focus (`InputTracker::focus_text`) as `ImeEvent`s, numbered per composition:
`Enabled`, `Preedit { text, cursor }`, `Commit`, `Cancel`, `Disabled`.

- Composed text reaches only the target where its composition began.
- A composition that showed preedit text ends in exactly one `Commit` or one `Cancel`, then `Disabled`. A composition
  cleared without a result string is a cancel (Microsoft Learn, WM_IME_COMPOSITION: "If none of the GCS_ values are set,
  the message indicates that the current composition has been canceled"); winit commits the empty result string there,
  which is left out (`ime_empty_commits`).
- Focus loss ends an open composition at once: the window loses the keyboard, the host moves the text focus, the target
  stops taking composed text, events are lost, the window goes, or a new composition begins. It sends `Cancel` when the
  preedit was not committed, then `Disabled`, never also a commit: nothing of that composition reaches the target
  afterwards. What the input method goes on with begins again as a new composition at the target that has the focus then
  (`Synthetic::Rebound`), or is left out when none takes composed text (`ime_dropped`).
- The candidate area follows the caret: the host reports the caret, viewport and scroll (`move_caret`, `scroll_text`)
  in logical pixels; on a change, a scroll or a scale change (`Resized`, `ScaleChanged`) the tracker asks for
  `WindowCommand::SetImeArea` with the caret in physical client pixels, kept inside the viewport when scrolled out, and
  asks again when a composition begins. `WindowCommand::AllowIme` follows whether the focused target takes composed text.
  The host carries out `InputTracker::take_requests` with `ImeRequest::command`.
- The composition itself is Kumihimo's (MT-198 KG-03): the tracker keeps which composition is open for which target,
  whose it is and whether its preedit is uncommitted, never its text, cursor or anchor (`tests/source_scan.rs`).

## Decisions and open items

- IME-FOCUS-CANCEL: on focus loss the tracker cancels; it cannot commit, since it keeps no composed text. Chromium asks
  the input method to complete instead (`imm32_manager.cc` `CleanupComposition`: `ImmNotifyIME(NI_COMPOSITIONSTR,
  CPS_COMPLETE)` when the window is deactivated while composing). If the input method commits before the focus message,
  the target gets that commit and no cancel. Asking for completion would be a window crate call (`ImmNotifyIME`), not made
  here; it needs a real input method on the Operator's desktop to prove (HBR-QUIET-004). Open for the Operator.
- The one event type that MT-216 and MT-217 consume is to be agreed with the Kumihimo text port (MT-198 KG-03). `ImeEvent`
  is the operating system's side of it; whether Kumihimo adopts it or maps it is the asset owner's decision.
- The origin of an input method message is that of the window's newest key, as for text (`window/src/platform/window.rs`
  `origin_of`): the operating system says nothing of `WM_IME_*`. A change made with the mouse in the candidate window
  is attributed to that key too.
- UNVERIFIED: whether an input method leaves a stale result string that winit reads again at the composition's end after
  a cancel; it would show as a commit of old text. Needs a real input method.

## Research basis

winit 0.30.13 source in the Cargo home (`platform_impl/windows/event_loop.rs` `WM_IME_STARTCOMPOSITION`,
`WM_IME_COMPOSITION`, `WM_IME_ENDCOMPOSITION`; `ime.rs`; `keyboard.rs` dead keys and repeat bit; `event.rs` `Ime`):
followed for the event mapping, the start and end as enabled and disabled, the clearing preedit before a commit, the
commit at the end for the Korean input method, and byte cursors. Microsoft Learn: WM_IME_COMPOSITION (a message with no
GCS flags is a cancel), ImmNotifyIME (CPS_COMPLETE, CPS_CANCEL). Chromium `ui/base/ime/win/imm32_manager.cc`: complete on
deactivation, candidate window `CFS_EXCLUDE` around the caret rectangle (winit does the same in `set_ime_cursor_area`).
TSF: winit has none; TSF input methods reach IMM32 applications through the system's compatibility layer.

## Tests

Validator only: `cargo test --locked -p handshake_shell_input` in `src/frontend/handshake_shell`.

| Target | What it proves, and the mutation it fails for |
| --- | --- |
| unit `ime_tests` | sequences as winit's source writes them: a commit (no cancel), Esc (one cancel, empty commit left out), the Korean end, clause commits; focus loss mid-composition (one cancel, a late commit reaches nobody); focus loss after a commit (no cancel); focus moved (ends at the old target, goes on at the new); composition turned off; events lost and the window gone (one end); a composition over an open one; nothing without keyboard or target; a model refused while the Operator composes, not when the composition is a model's; cursors off the text; the candidate area at 100 % and 150 %, scrolled out, on scroll and scale change only, again at a composition's start; 400 generated interleavings of all of these keeping one target, one beginning, one end, a cancel only for uncommitted preedit and never a commit after it; the kept state prints no composed text. Fails if the cancel is dropped or doubled, a late commit is passed on, events go to an unfocused target, the area does not follow, or a model is not refused. |
| unit `tracker_tests` | every input kind is one typed input with its stamp's origin, time and stamp; what is no input gives none; wheel sources; an older Operator stamp is not the Operator's; a model's input is always the model's, and refused for keys and text while the Operator composes; press, repeat with and without the mark, release; dead keys with the accent typed once by the next key; held keys released once with their press's origin when the keyboard goes, and not twice after winit's own release; buttons and keys of every window on lost events and a window gone; the hold limit; the input method through the tracker with its requests; a key prints no key. |
| `source_scan` | no window library, Win32 type or unsafe code in the code, the window crate the only dependency, no loop or native window passed on; nothing injects input or a composition (`SendInput`, `keybd_event`, `mouse_event`, `Inject*`, `ImmSetCompositionString*`, `ImmNotifyIME`) or moves focus or the cursor, tests included; `InputEvent` has no public field and no public constructor; the input method state holds no text; every loop of a test on a private desktop; no banned crate locked. Each with a control that it finds what it looks for. |
| `real_loop` | on a private desktop, posted to a quiet window through winit and the window crate: a space bar the system translates, its repeat and the focus going give one press, one repeat, one release and two texts; with the input method allowed for a target, start, emptied preedit, focus lost and the input method's own end give enabled, preedit and one synthesized end; a composition the host moves the focus from goes on at the new target as a new one; with a target that takes no composed text, `AllowIme(false)` stops the window's input method events. |

Needs the Operator's desktop and x64 hardware (HBR-QUIET-004, MT-202): real typing on real layouts (dead keys, AltGr),
a real input method (Japanese, Chinese, Korean) and its candidate window placement at 100 %, 150 % and across monitors,
focus loss with the input method's own completion, precision touchpad and pen.

## Failure codes

`FocusRefusal::code`, `ModelRefusal::code`, `Synthetic::code`: `input.*`, `input.model.*`, `input.synthetic.*`. They
carry no content. Events print without key codes, key meanings and text.
