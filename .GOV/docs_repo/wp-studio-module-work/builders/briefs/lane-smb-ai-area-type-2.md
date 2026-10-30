---
file_id: LANE-SMB-AI-AREA-TYPE-2
file_kind: lane_brief
updated_at: 2026-10-30T04:05:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-AREA-TYPE-2
lane_file: builders/lanes/hsk-studio-ai-area-type-2.json
crates: [hsk-studio-harness-view]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Show path and area text in Studio's window">

## Target

SMB-AI-AREA-TYPE-1 (READY at c951bbf7, lane file `builders/lanes/hsk-studio-ai-area-type-1.json`) made composite draw live path text and area text from Folio records, but only when the host shapes the story: `hsk_studio_composite::VectorAssets::shaped_story` (`crates/hsk-studio-composite/src/vector.rs:1063`, default `None`, the layer then reports `vector_text_not_provided`). The viewer's impl (`crates/hsk-studio-harness-view/src/view/vector.rs:792`, `impl VectorAssets for EntryAssets`) does not implement it, so the window shows no path/area text.

1. Slice 1: implement `shaped_story` in the viewer's `EntryAssets`, shaping with the same Type path the viewer already uses for text layers (`src/view/text.rs`); fonts, features and errors exactly as text layers do (no second font resolution). A story that cannot be shaped stays `None` and the report stays visible in the viewer's existing report surface.
2. Slice 2 (only if slice 1 is green with time left): expose the AREA-TYPE-1 Create Outlines proposal for point/area/path text as one typed, undoable viewer command where the viewer already lists vector/text commands; reuse the existing command registry and descriptor conventions (no modifier-less shortcut collisions: the key-uniqueness test exists).

Read the AREA-TYPE-1 lane file notes (gaps, `expand_vectors` limitation) first.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths

- `crates/hsk-studio-harness-view/src/view/vector.rs`, `src/view/text.rs` (only to share the shaping helper), NEW `tests/viewer_vector_text.rs`.
- Do NOT touch `src/view/timeline*` (SMB-AU-MULTITRACK-1), `src/view/design/**` (SMB-R40-FOLLOWUPS-1), engine crates (read-only; report missing APIs to the lead), `RUN-UI.md` (list lines in notes).

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Viewer tests: a document with a path-text binding and an area-text binding renders text pixels inside the container / along the path and reports no `vector_text_not_provided`; an unshapeable story keeps the report. Red on c951bbf7 (the default `None`).
- One bundled lane-check on C: over the WHOLE harness-view and harness suites with the round's viewer features (`.config/round-features.toml`, rule 4e).
- Illustrator behaviour stays UNVERIFIED (D5). GUI lane 2 of max 2.

</topic>
