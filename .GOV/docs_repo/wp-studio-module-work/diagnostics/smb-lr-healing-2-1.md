---
file_id: diagnostics-smb-lr-healing-2-1
file_kind: diagnostics
lane: SMB-LR-HEALING-2
crate: hsk-studio-harness-view
input: lane file builders/lanes/hsk-studio-lr-healing-2.json (runs 1 at 80ea5c88 and 2 at 19516258, same failure); Handshake_Artifacts/WP-KERNEL-STUDIO/smb-lr-healing-2/nextest/default/junit.xml (run 2)
measured_at: lane HEAD 19516258 (static analysis of the lane code and egui 0.33.3 sources; no diagnostic run, counter at its limit)
updated_at: 2026-10-24T09:40:00Z
---

<topic id="cause" status="current" summary="develop panel widens every frame: push_id buttons in wrapping rows never wrap">

Cause (<=120 chars): develop panel rows put each button in ui.push_id; egui never wraps them, so the right panel widens every frame.

PRODUCT layout defect (visible in the real window: the develop panel creeps from 300 px towards ~1200 px over the first
~10 frames after opening a raw/DNG and squeezes the canvas), not a test-criterion defect. The settle helper is right:
the preview rect really moves every frame (x shrinks as the panel grows), monotone, not an alternation.

</topic>

<topic id="evidence" status="current" summary="code path, egui mechanism, numeric reproduction of x=290 at frame 8">

Code (lane worktree, HEAD 19516258):
- `src/view/develop.rs:1247-1269` develop buttons row and `:1270-1299` family row (Upright modes, lens, lateral CA,
  grain seed, vignette style, Select subject/sky/person: ~13 buttons, ~1200 px unwrapped), `:1332-1342` export row:
  every button is `ui.push_id(w(name), |ui| ui.button(label))` inside `ui.horizontal_wrapped`.
- `src/view/develop/spot.rs:185-193` (mode row) and `:294-308` (spot rows): same pattern.
- `src/view/mod.rs:964-965`: the develop panel is `SidePanel::right(..).resizable(true).default_width(300.0)` with a
  vertical `ScrollArea` (auto-shrink: its width follows the content).

egui 0.33.3 mechanism (`~/.cargo/registry/src/*/egui-0.33.3/src`):
- `ui.rs:2505-2517` `scope_dyn` (what `push_id` calls): the child Ui gets `max_rect = available_rect_before_wrap()` and
  the parent only advances by the child's `min_rect`; the parent's wrap logic (`layout.rs:536-557`, `next_frame`) never
  runs for it, and inside the fresh child no wrap happens because `cursor.left == max_rect.left` (`layout.rs:547`).
- `layout.rs:464-513`: once the row overflows, `available_rect_before_wrap` beyond the row end has zero width.
- `atomics/atom_layout.rs:171-194` + epaint line breaking (falls back to break-anywhere): a Button in a Wrap-mode
  layout shrinks its text to the available width, so a zero-width button becomes one glyph wide (~26 px with
  padding and spacing) and the row overflows past the panel edge.
- `containers/panel.rs:249-257, 300, 344`: a SidePanel's width is the previous frame's content rect width; the
  overflow is stored as the new width, so the panel grows each frame until the family row fits unwrapped.

Numeric reproduction (family row widths estimated from labels, egui default 12.5 pt font, 8 px spacing): content
width C per frame 284 (default 300) -> 526 -> 711 -> 850 -> 936 -> 1014 -> 1074 -> ~1108 at frame 8, i.e. preview
x = 1400 - (C + 16) + 8 = ~284..290; observed x = 290 at frame 8 (`junit.xml`, all three tests). The panel was
still growing, so no two consecutive frames matched within the helper's 8 frames.

Run 1 (drawn-list settle) failing too is consistent: the bottom panels in the central column shrink every frame as
the right panel grows (UNVERIFIED detail; same cause).

Related: F-TL-1 (viewer toolbar ~800 px tall, `hsk-studio-timeline-ui-1.json`) is the same egui mechanism in the
toolbar (sliders inside `push_id` in wrapping rows staircase); not this lane's code (`view/mod.rs`).

</topic>

<topic id="distinct-from" status="current" summary="differs from the builder's alternation / scrollbar guess">

Distinct from the builder's UNVERIFIED hypothesis (rect alternating between two layouts, e.g. the right panel's
ScrollArea scrollbar toggling): egui 0.33 scroll bars are floating by default and allocate 0 width
(`style.rs:583-587, 640-656`), so they cannot move content; the motion is a monotone creep of the panel width.
Settling "on a rect repeated within N frames" or more frames would hide a real panel creep; not done.

</topic>

<topic id="fix" status="current" summary="row allocates each item at its unwrapped size, item keeps its stable id">

Fix direction: every item of a wrapping row is allocated by the row itself at its unwrapped size
(`ui.allocate_ui(size, ..)`, which runs the row's wrap logic) and drawn inside under its stable `push_id` id; rows
wrap at the panel width, the panel width converges in a few frames (remaining fixed-width rows: control rows,
receipt grid). Applied in `develop.rs` (develop buttons, family and export rows) and `develop/spot.rs` (mode and
spot rows) through one helper `wrap_item` in `develop.rs`. Settle criterion unchanged (two consecutive identical
preview rects within 8 frames); the helper now reports the per-frame rect trace when it fails.

</topic>

<topic id="proof" status="current" summary="one focused probe, one confirming bundled run">

Cheapest proof: one focused probe on C: (the three failing tests, lane-check, unchanged settle criterion), then the
lane's bundled run (rule 4a). If the probe fails, the lane stops with the trace from the settle panic.

Result: probe 3/3 PASS at 46b5bcdd; bundled 430/430 at fd02e21b; after a second rebase (SMB-FIG-EDIT-2 42b304c7)
435/435 at 09b4f637 (exit 2 only from lane-check.sh being rewritten mid-run, fdbe825c); pushed 09b4f637.

</topic>
