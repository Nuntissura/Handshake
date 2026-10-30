---
file_id: LANE-SMB-FIG-GRID-3
file_kind: lane_brief
updated_at: 2026-10-30T17:55:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIG-GRID-3
lane_file: builders/lanes/hsk-studio-fig-grid-3.json
crates: [hsk-studio-harness-core (one additive Run step), hsk-studio-harness-view (design/auto_layout.rs)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Auto-layout edits on frames with text children">

## Target

SMB-FIG-GRID-2 (READY at 825598c6) made the window's auto-layout panel and carried imported auto-layout from `.fig` files into the window. Read its lane file `builders/lanes/hsk-studio-fig-grid-2.json` first.

But every auto-layout frame in the Operator `.fig` fixtures contains text. Edits there are refused with `auto_layout_text_layer_unsupported`, because moving a text child changes the text entry, and that must happen in the same undo step as the design-record change.

1. **harness-core.**
   - Add the smallest additive `Run` step that commits the design-record change and the text-entry change as ONE history step. The precedents are `restructure_and_entry` and `submit_sequence_edits_and_entry`.
   - Keep the redo history intact on a refused write. Reuse `Run::keeping_redo_without_new_step` (SMB-R42-FOLLOWUPS-1).
   - Add a RUN_COMMANDS row. The source-reading corpus check will fail without one.
   - Test: one step, undo and redo restore both entries, and a refused write leaves no split step.
2. **harness-view.**
   - The auto-layout panel moves and resizes text children through that step.
   - A text child whose size would change is resized only when its text frame kind allows it, such as point text that moves without resizing. Otherwise it is refused with a typed report naming the child.
   - Remove the blanket `auto_layout_text_layer_unsupported` refusal.
   - Test with the `fixture-button` frame: change the spacing, and the text child moves to the solved position, in one undo step.
3. Restyling, recolouring and moving a text child keeps its grid anchor. Extend the GRID-2 anchor tests if needed.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-fig-grid-3`, base 825598c6)

- harness-core: `src/harness.rs` (the new step and its helper use), `src/action.rs` (the corpus row), and a new test.
- harness-view: `src/view/design/auto_layout.rs`, plus the text-move helper wherever text layers are moved today (read first; name the file).
- Do not touch:
  - pigment, folio and composite (SMB-R43-FOLLOWUPS-1);
  - type, layout and interop-layout (SMB-ID-FRAME-AUTOSIZE-2);
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Write tests red first. Pushed red commits must compile.
- Do one bundled run covering the whole suites of harness-core, harness-view and harness, with desktop + audio-batch (rules 4a and 4e). Name the log per RUN LOGS.
- Do not start the bundled run until the lead messages you with the disk and the base. Folio and type lanes are in flight, and both sit in your closure.
- Figma behaviour stays UNVERIFIED (D5).
- GUI lane 1 of 2.

</topic>
