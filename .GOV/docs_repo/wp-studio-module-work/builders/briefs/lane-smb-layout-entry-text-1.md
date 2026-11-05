---
file_id: LANE-SMB-LAYOUT-ENTRY-TEXT-1
file_kind: lane_brief
updated_at: 2026-11-05T07:43:42Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-ENTRY-TEXT-1
lane_file: builders/lanes/hsk-studio-layout-entry-text-1.json
crates: [hsk-studio-layout (derived text of TOC, running text and cross-references; note hosts)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Text derived from a story carries no stream sentinels and ends where the paragraph ends">

## Target

Read `validation/round-R55.json` F-R55-5, F-R55-9, F-R55-11 (finding, repair_direction, file_line, evidence) and the probes in `Handshake_Artifacts/WP-KERNEL-STUDIO/validator-r55-probe/` (n7a-d, l1e, l3). Spec: Master Spec v02.229 STU-TYP-207 (:9106-9160): a marker sentinel carries no text; composition shows the resolved value without rewriting the stream. All three predate SMB-LAYOUT-MARKER-LOOKUPS-1.

1. **F-R55-5, sentinels in derived text.** TOC entries (`generate_toc`, `generate_table_of_contents`, `TocEntryOut.text`, the generated story), running text (`running.rs:117`) and `<fullPara/>` cross-references (`xref.rs`) copy stream sentinels. Through native Type an index marker becomes an orphan marker, a current-page marker in a heading resolves against the TOC story's own page instead of the heading's (n7c: a wrong value), and direct Type over the entry text refuses MissingGlyph. One helper for all of them, not a copy per site:
   - a marker with no value at the source (index, anchor, note reference and the like) does not appear in derived text;
   - a value-bearing marker (page number, section name and the like) appears as its value resolved at the SOURCE location, never the location of the generated text;
   - if that value is not available to the call, a typed outcome, never a wrong value and never a silent drop.
   Declare which members of the STU-TYP-207 catalog fall in which group, with file:line. V4-06: list every site that copies story text into generated or derived text (`git grep -nE "clean_keeping|entry_text|full_para|fullPara"` in layout, then widen by reading callers), fixed or not applicable per site.
2. **F-R55-9, terminators in entry text.** The TOC trims only U+000A, U+000D and U+2029, not the 12 terminators of `hsk_studio_type::ends_paragraph`. A heading ended by a break sentinel keeps it (U+FDDA): the generated story says one paragraph while the partition says two, and through native Type the entry's page label lands after the page break and is not placed (n7d). Trim with `ends_paragraph` in `clean_keeping` (`toc.rs:187`), `entry_text` (`toc_contract.rs:673`) and the xref full text (`xref.rs:377`). One test per terminator, all 12.
3. **F-R55-11, note hosts.** An endnote or sidenote host with `StoryContent::Stored` keeps a gap reference unplaced because `NoteHost` has no text (`endnotes.rs:31`, `:72`, `:196`; typed today, no silent loss). `NoteHost` gains the resolved text so these place like footnotes. Add the missing tests: `layout_footnotes` with stored content (`footnotes.rs:270`) and `index.rs:205`.
4. **MTs.** Read the TOC and cross-reference MTs named in round-R55 `mt_judgments` first; claim only rows this proves. Do not edit MT files.

Hostile input (V4-01/02): no new allocation unbounded by the existing limits. Vendor parity UNVERIFIED (D5): InDesign's handling of markers in a TOC entry has no oracle; raise it, do not assert it.

Out of scope: U+FFFC with native Type (F-R55-13, recorded separately); paragraph.rs and cjk.rs (SMB-R54-FOLLOWUPS-1 holds them now); any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: layout returns typed outcomes, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: generated text is not a mutating command", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-layout-entry-text-1`, branch `lane/smb-layout-entry-text-1`, base ce644577)

- layout: `src/toc.rs`, `src/toc_contract.rs`, `src/running.rs`, `src/xref.rs`, `src/endnotes.rs`, `src/footnotes.rs`, `src/index.rs`, one new helper module, `src/lib.rs` exports and DESCRIPTOR, tests.
- interop-layout: a new native-Type test file for items 1 and 2.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of layout and its direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-layout-entry-text-1 --disk d`): step 0; clippy over layout, interop-layout and layout's direct dependents; nextest WHOLE suites of layout and interop-layout; doctests if added.
- Push when green, no release needed: rebase, `--per-sha`, guarded sha push on `PER_SHA_RESULT OK`. SMB-R54-FOLLOWUPS-1 works in layout at the same time in other files: whoever pushes second rebases and reruns the layout suite if the rebase touches it.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
