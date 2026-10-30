---
file_id: LANE-SMB-ID-FRAME-AUTOSIZE-2
file_kind: lane_brief
updated_at: 2026-10-30T17:50:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-FRAME-AUTOSIZE-2
lane_file: builders/lanes/hsk-studio-id-frame-autosize-2.json
crates: [hsk-studio-type (two read-only metric accessors), hsk-studio-layout, hsk-studio-interop-layout]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Finish InDesign frame auto-size and first baseline: real metrics, refusal code, IDML mapping">

## Target

SMB-ID-FRAME-AUTOSIZE-1 is READY at 2f8e13d5. Read its lane file `builders/lanes/hsk-studio-id-frame-autosize-1.json` first, especially its notes on decisions 1 and 2 and its follow-ups. Then close these gaps, pushing after each green slice:

1. **Type metrics.**
   - Add `Run::cap_height_pt()` and `Run::x_height_pt()` to `hsk-studio-type`, both returning `Option<f64>`. Source them from the face's OS/2 `sCapHeight` / `sxHeight` through the existing private provider; read how it parses OS/2 today.
   - `None` when the table or version lacks the field. No estimate from glyph outlines unless type already does that elsewhere; if it does, name where.
   - Layout's `from_type` fills cap height and x-height from them, so hosts no longer need `with_face_metrics` for the six first-baseline kinds.
   - Test against a fixture face whose OS/2 values you read from the font bytes in the test: hand-decoded big-endian i16 at the table offset, not through type's own parser.
2. **Refusal code.**
   - Switch the reflow's missing-metric refusal from `unsupported_composition` to `first_baseline_metric_unavailable:<kind>`.
   - Update the interop-layout pin `tests/layout_reflow_folio_export.rs:268-271` to the new code. List it in the notes as a change to another lane's test.
3. **IDML mapping.** Map IDML text-frame auto-size attributes to the folio settings AUTOSIZE-1 lays out, in `hsk-studio-interop-layout`'s IDML import, plus export if an IDML writer exists. Read IDML element and attribute names from the IDML fixtures the importer already tests against; never guess them.
   - Unknown values are a typed loss.
   - Documents without the attributes import byte-identically.

Out of scope:
- a folio record for the six STU-LAY-118 rows folio does not store (it needs a folio decision; SMB-R43-FOLLOWUPS-1 is changing folio now);
- the window committing `reflow_autosize` bounds (GUI).

List both under follow-ups.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-frame-autosize-2`, base ff4ca080)

- type: the `Run` accessor and the provider read path, plus a new test.
- layout: `from_type`, the reflow refusal site, and tests.
- interop-layout: the IDML import/export modules for frames, `tests/layout_reflow_folio_export.rs` (the pin only), and a new test.
- Do not touch:
  - folio, pigment and composite (SMB-R43-FOLLOWUPS-1);
  - harness-view and interop-fig* (SMB-FIG-GRID-2);
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof and order">

## Proof and order

- Write tests red first. Pushed red commits must compile.
- Do one bundled run covering the whole suites of type, layout and interop-layout, plus clippy over their dependents (rule 4a). Name the log per RUN LOGS.
- Type sits upstream of composite and layout sits downstream of folio, so this lane and SMB-R43-FOLLOWUPS-1 each change the other's closure. The lead orders the runs.
- Do not start the bundled run until the lead messages you with the disk and the base.
- InDesign behaviour stays UNVERIFIED (D5).

</topic>
