---
file_id: LANE-SMB-LAYOUT-BLANK-LINE-LOOKUPS-1
file_kind: lane_brief
updated_at: 2026-11-05T14:24:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-BLANK-LINE-LOOKUPS-1
lane_file: builders/lanes/hsk-studio-layout-blank-line-lookups-1.json
crates: [hsk-studio-layout (TOC, index and cross-reference line lookup)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="An index mark or cross-reference destination on a blank line is placed, not lost">

## Target

STUDIO-R56 findings F-R56-5 (MEDIUM) and F-R56-9 (LOW): read both in `S/validation/round-R56.json`, evidence PROBE-R56-ENTRY-1, and the notes of `builders/lanes/hsk-studio-layout-entry-text-1.json`. `toc::line_of` misses the blank line of a paragraph with no glyph, because that line's source span is empty and sits at the paragraph's end. Measured: an index mark in a marker-only or empty paragraph counts unplaced and its entry is left out (`generate_index` gives 'Index', unplaced 1), though `compose_with_sentinels` places the marker on page 2; a cross-reference destination there reads Unplaced for `<pageNum/>`. `toc::blank_line_of` exists and is read only for the TOC decision and gap reasons. A blank line made by U+2028 inside a paragraph with glyphs is found correctly.

1. **Readers.** List every caller of `line_of` (at base at least `toc.rs`, `index.rs:209`, `xref.rs:315`, `on_hidden_layer`), and for each say what a blank line must answer. Read the spec rows for TOC, index and cross-references in `.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md` and quote them in `notes.spec_basis`. Where the spec is silent, write a declared reading and a `notes.spec_proposals` entry, labelled UNVERIFIED (D5).
2. **Table test, red first.** One table over mark and destination kinds against paragraph shapes: empty, marker-only, spaces only, U+2028 only, with glyphs; placed, overset, hidden layer. Prove it red at base.
3. **Fix.** Marks and destinations resolve their blank line. An index entry and a `<pageNum/>` answer the page the composer placed them on. Before, a blank styled paragraph was not a running-text occurrence: that behaviour does not change by accident. Either keep it with a test, or change it with its own proof and a stated reason.
4. **Hidden and overset readings (F-R56-9).** A marker-only heading on a hidden layer with `include_hidden` off counts `omitted_no_text` because `on_hidden_layer` goes through `line_of`; it should count hidden if the layer is what removes it. With `include_overset` on, a placed empty heading lists an empty entry where with it off it counts overset; a whitespace-only heading lists its spaces with a label. Decide from the spec or a declared reading, with one rule for the empty cases. Every omission stays typed and counted.
5. **Consumers.** Check readers of these answers in `interop-layout` (`tests/longdoc_pdf.rs` and the outline and index exports): an entry that now appears must export.
6. **MTs.** Read the rows under the TOC, index and cross-reference anchors; claim only rows this proves. Do not edit MT files.

Hostile input (V4-01/02): no new unbounded allocation; a document of only empty paragraphs at the existing limits stays bounded (one test). Vendor parity UNVERIFIED (D5).

Out of scope: `adobe_composer.rs` (SMB-LAYOUT-COMPOSER-DEPTH-1 is open at the same time); section numbering in TOC and cross-references; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: layout returns typed answers and omission counts, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composition and generation commit nothing", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-layout-blank-line-lookups-1`, branch `lane/smb-layout-blank-line-lookups-1`, base 091c4d1b)

- layout: `src/toc.rs`, `src/index.rs`, `src/xref.rs`, `src/derived_text.rs`, `src/index_contract.rs` as the table proves, `lib.rs` DESCRIPTOR, `tests/`.
- interop-layout: `tests/` only.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-layout-blank-line-lookups-1 --disk d`): step 0; clippy over layout and its direct dependents; nextest WHOLE suites of layout and interop-layout; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-layout-blank-line-lookups-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
