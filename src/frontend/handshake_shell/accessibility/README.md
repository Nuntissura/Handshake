# handshake_shell_accessibility

Accessibility of the kernel-owned shell (WP-KERNEL-012 MT-214 slice 4, AC214-7): the host's semantic tree of each
window reaches UI Automation with node ids stable across frames, roles, names, values, bounds, text runs, focus and
actions; what assistive tools and models ask for comes back as typed requests with their origin; reading a tree never
activates its window; every window has its own tree. The interface carries AccessKit core types only (AC214-1), plus
the window and input crates' data types. The contract is in the crate documentation (`src/lib.rs`).

## Files

| Path | Holds |
| --- | --- |
| `src/frame.rs` | the host's frame: `NodeKey`, `SemanticNode`, `SemanticFrame`, relations and selections by key, the checks, `text_run` |
| `src/tree.rs` | one window's AccessKit tree: ids by key, the scale as the root's transform, updates of what changed |
| `src/request.rs` | `AccessRequest`, `RequestData`, the bounded queue and `ActionSink` (the adapter's action handler) |
| `src/hub.rs` | `Accessibility`: every window's tree, its adapter, `TreeSource` (the adapter's activation handler), the report |
| `src/platform.rs` | Windows only: the asset adapter and the crate's only unsafe code (two reads), nothing public |

## Bindings

- Window crate: `Accessibility::attach(&Control, WindowKey)` on `WindowEvent::Created`, before the first `Show`, with the
  window's `NativeWindowToken` (the extension point the window crate's README names for AccessKit). The adapter keeps
  the token, so `Destroy` and `finish` are refused until `Accessibility::close`. `feed` follows `ScaleChanged`,
  `Resized` and `Destroyed`. No change to the window crate.
- Input crate: `SemanticNode::text_target` names the text target a node shows; `SemanticFrame::focus_on_text` takes
  `InputTracker::text_focus` (added, read only); a focus request gives `AccessRequest::text_target` and
  `AccessRequest::focus_by`, which is `FocusBy::Model` or `FocusBy::OtherProgram` (added: like a model, it yields to the
  Operator's composition), for `InputTracker::focus_text`.
- Requests: `set_notifier` with a wake of the loop (`LaneWaker::wake`); `take_requests(Control::now)` on the loop's thread.

## The asset adapter (GLOBAL-ASSETS, HBR-MODULE-LIFECYCLE)

`accesskit_windows` 0.29.2 is the Operator's asset copy, `Kumihimo/product/third-party/accesskit_windows-0.29.2` at
assets rev `8eadc5b57dc20d96bf29340f835505ad8ff7cbc8`, the revision `../handshake_native/Cargo.toml` patches in; the
workspace root patches it here too (`[patch.crates-io]`, accesskit_windows only). Against crates.io 0.29.2 it changes
`src/node.rs` `ProviderOptions` alone (and adds a test): the root of a window of this process with `WS_EX_NOACTIVATE`
reports `ProviderOwnsSetFocus`, so a UI Automation `SetFocus` on it goes to the provider (a queued `Focus` request for
`AccessTarget::Window`) instead of focusing the window. Everything else is upstream. Its closure is accesskit 0.21.1,
accesskit_consumer 0.31.0, hashbrown 0.15, static_assertions and windows 0.61.3: no eframe, egui or other AC214-1 crate
(`tests/source_scan.rs`, cargo tree).

Wired as accesskit_winit 0.29.2 wires it on Windows (`platform_impl/windows.rs`): a `SubclassingAdapter` on the window
before it is first shown, `update_if_active`, then `QueuedEvents::raise` with nothing borrowed. The adapter's procedure
sits above the window crate's comctl32 subclass and answers `WM_GETOBJECT` only, noting `WM_SETFOCUS`/`WM_KILLFOCUS`
for the focus it reports. The plain `Adapter` would need the window crate to forward `WM_GETOBJECT` and the focus
messages through a new hook, a larger change to that crate; it was not taken.

The adapter panics instead of failing in three places, which the crate keeps from the loop: a second adapter on a
window (one per window, refused as `AlreadyAttached`), a window already shown (`AlreadyShown`), and a failed install
or event raise (caught: `AttachRefusal::Adapter`, `AccessReport::raise_failed`). Its text code unwraps and slices text
runs on the threads tools call it from, so the crate refuses frames with runs it cannot read (`TextRunFault`), and
frames whose next-on-line or previous-on-line relations make a circle or lead to a node that is no text run, since the
consumer walks them without end (`FrameRefusal::LineCycle`, `LineNotRun`). A window shown once is refused even when it is
hidden again (`AlreadyShown`, from the events fed to `Accessibility::feed`; a Show and a Hide within one handler, before
the loop reports either, cannot be told from a window never shown, and only its visibility is read then). A failed
install leaves the adapter's window property (`AccessKitAdapter`), set before the procedure, pointing at freed state:
the host removes it after the failure (`platform.rs` `guarded`), which is safe because it first refuses a window that
already has the property. The asset should set the property after the procedure instead (asset request RQ-AKW-01); until
then the removal is the host's, and a failure of `RemovePropW` itself is not reported. Requests an assistive tool queued
for a window are dropped when the window closes, is destroyed, or the loop ends.

## Decisions and open items

- A key keeps its id while it is in every frame; a key that comes back after a frame without it gets a new id, so an
  id a tool kept from a node that went never names another node.
- The window's scale is the root's transform; the host gives logical bounds, and a scale change sends the root alone.
  `ScrollToPoint` from a tool (physical, window-relative) reaches the host in logical pixels.
- A request through the adapter is `Origin::OtherProgram`: the operating system does not tell an assistive tool from
  automation, and neither has the Operator's authority. A model's action through the steering route
  (`Accessibility::model_action`) is `Origin::ModelAction`; Argus reads `Accessibility::snapshot` in process, so no
  message reaches the window.
- S4-Q1, UNVERIFIED: whether UI Automation tries to focus the hosting window for a `SetFocus` on a node below the root.
  The asset sets `ProviderOwnsSetFocus` on the root only. On a quiet window the window crate's guard refuses any such
  activation (counted in `RunReport::activations_refused`); on an Operator window it would not. Not exercised by the
  tests (they `SetFocus` the root only); a real client on the x64 PC decides it, and if it does, it is a request to
  the asset owner, not a change here.
- S4-Q2: a node's UI Automation focus shows only while the window has the keyboard (AccessKit's host focus); a quiet
  window never has it, so its focus is visible to tools only in a frame snapshot.

## Failure codes

`AttachRefusal::code` (`a11y.attach.*`), `FrameRefusal::code` (`a11y.frame.*`), `ActionRefusal::code`
(`a11y.action.*`). `SemanticNode`, `SemanticFrame` and `RequestData::Value` print without names, values or text.

## Tests

`cargo test --locked -p handshake_shell_accessibility` from `..` (validator only).

| Target | Rule, and the mutation it catches |
| --- | --- |
| unit `tree_tests` | every update applied to accesskit_consumer 0.31.0 (what the adapter reads) and compared with the whole tree: ids stay with keys across 400 random frames and are never given twice (ids by position, reused ids); an update without a new or moved node's parent or with a removed node panics there; one change sends one node, none sends nothing; bounds at 1.0, 1.25, 1.5 and 2.0 through the root alone (nodes scaled one by one, or not at all); every refusal leaves the tree; host-set ids cleared and relations by key; focus by key, by text target, or the window; `text_run` read back by the consumer (byte or UTF-16 lengths) |
| unit `request_tests` | through the same `ActionSink` the adapter gets: two windows that both number their button 2 give requests with their own window and key (one id map, routing by id); stale, undeclared, closed-window and excess requests dropped and counted (unbounded queue); data in keys and logical pixels, values not printed; a model's action is the model's and checked the same way; a focus request moves the input crate's text focus as another program, and the tree's focus follows `text_focus` |
| unit `hub` | an activation while a frame is taken gets a placeholder, and the next update is whole |
| `source_scan` | Win32 and the adapter only in `src/platform.rs`, which makes nothing public; no toolkit; no call that activates, orders, shows or focuses a window or injects input, tests included (a Win32 `SetFocus`, not a provider's); no public constructor or field of a request; loops and client threads on a private desktop; the adapter is the asset's revision; no AC214-1 crate locked; each with a control |
| `uia_client` | real loop, quiet windows on a private desktop, a UI Automation client (`IUIAutomation`, MTA) on another thread of that desktop: two windows' trees read with names, automation ids, control types and screen bounds at the window's scale; same keys, separate trees; no activation tried, no quiet window broken, no focus or display change, Z order kept, and both desktop watches clean; the frame's focus shown while the window has the keyboard; bounds at a doubled scale; Invoke and a root `SetFocus` back as typed requests of another program, with no activation tried; attach refused on a shown window and twice; `Destroy` refused until `close` |

Left for the x64 PC and a real desktop: Narrator or another screen reader against an Operator window, a real
per-monitor DPI change across monitors (the test changes the tree's scale only), S4-Q1, and the timing of
`WM_GETOBJECT` while the host runs a modal loop.
