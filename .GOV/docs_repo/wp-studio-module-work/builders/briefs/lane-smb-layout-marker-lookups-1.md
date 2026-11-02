---
file_id: LANE-SMB-LAYOUT-MARKER-LOOKUPS-1
file_kind: lane_brief
updated_at: 2026-11-02T14:47:56Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-MARKER-LOOKUPS-1
lane_file: builders/lanes/hsk-studio-layout-marker-lookups-1.json
crates: [hsk-studio-layout (marker / anchor line and page lookups in footnotes, TOC, TOC contract and cross-references)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, V4-06)
---

<topic id="target" status="active" summary="V4-06 sweep of the F-R54-6 failure class outside the sentinel module">

## Target

SMB-R54-FOLLOWUPS-1 (pushed at b7066f94) fixed F-R54-6: a marker with no glyph at a line break was attributed to the line before, so it resolved on the wrong page. Its lane notes (`builders/lanes/hsk-studio-r54-followups-1.json`) list four similar lookups outside its paths that it did not check: `src/footnotes.rs`, `src/toc.rs`, `src/toc_contract.rs`, `src/xref.rs`.

1. **Class (V4-06).** A lookup that finds the line / page of a story byte from glyph or line extents, and so attributes a byte that has no glyph at a line boundary to the previous line. Give the search regex you use and list EVERY matching site in layout (these four and any other), each fixed or not applicable with file:line and the reason.
2. **Each site.** For a site in the class, write a red-first test that places the relevant anchor (footnote reference, TOC source paragraph anchor, cross-reference target / text anchor) at a line end directly before a line break and before overset text, run it red, then fix it the way F-R54-6 was fixed (reuse that helper if it is reachable; never a second copy of the rule).
3. **MTs.** Claim only rows this proves (footnotes MT-30780 / MT-30781 rows, TOC MT-30776 / MT-30777, cross-references MT-30780 family as applicable); read each MT first.

Out of scope: any other behaviour; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-layout-marker-lookups-1`, branch `lane/smb-layout-marker-lookups-1`, base b7066f94)

- layout: `src/footnotes.rs`, `src/toc.rs`, `src/toc_contract.rs`, `src/xref.rs` and any other site the sweep finds in the class, tests.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per applicable site, proven red by running it; a pushed red commit must compile the round units of layout and its direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-layout-marker-lookups-1`): step 0; clippy over layout and its direct dependents; nextest WHOLE suites of layout and interop-layout.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit (or, if no site is in the class, record the sweep, push nothing, and report). Never claim PASS.

</topic>
