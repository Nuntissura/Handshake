---
file_id: LANE-SMB-LAYOUT-SENTINELS-1
file_kind: lane_brief
updated_at: 2026-11-01T12:58:23Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-SENTINELS-1
lane_file: builders/lanes/hsk-studio-layout-sentinels-1.json
crates: [hsk-studio-layout (resolve marker, break and flush-space sentinels at composition)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R52 F-R52-1: layout resolves the v02.228 stream sentinels at composition">

## Target

Read `validation/round-R52.json` F-R52-1 first. Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-TYP-207 and the v02.228 SD-29 sentinel assignment (:9106-9160), with the layout clauses they point to (STU-LAY-014/015 page numbering and markers, STU-LAY-108 sections and the page-number enumeration, STU-LAY-018/020 threading and flow, STU-LAY-021 columns). The type crate (SMB-TYPE-SENTINELS-REGISTRY-1, PASS_V1 in R52) stores and inserts the sentinels, ends paragraphs at break sentinels in its own engine, and the shaper refuses marker and flush sentinels (MissingGlyph) by design: resolving them is layout's job. Nothing in layout reads U+FDD0..U+FDDD today.

1. **Markers resolve at composition.** Every marker sentinel (current / next / previous page number, section marker and the other marker members the spec assigns) is replaced, for shaping only, by the text it resolves to for the page / section where the containing line lands: page numbers in the section's numbering style, section marker text from the section record. The stored story keeps the sentinel (one code point; byte offsets of the stored text never change). A marker whose target cannot be resolved (e.g. next page beyond the last page) resolves as the spec says, or to a typed unresolved outcome if the spec is silent (record that). Re-composition after a page move updates the resolved text.
2. **Break sentinels direct the flow.** Column break moves the following text to the next column, frame break to the next threaded frame, page break to the next page, odd-page / even-page break to the next page of that parity (inserting the blank page only if the spec says so; otherwise a typed outcome). A break at the end of the last frame overflows into overset, never silently dropped. Paragraph partition in layout splits at break sentinels exactly as type does (no InvalidBidi refusal for layout-built requests).
3. **Flush space.** On a fully justified last line, a flush-space sentinel absorbs all remaining justification slack (STU-TYP-207 / SD-29); elsewhere it behaves as the spec states. It is never shaped as a glyph.
4. **Tests.** Red first per item: each marker member on a multi-page, multi-section document (including after re-composition), each break member with its flow target and the overset case, flush space on a justified last line vs a non-justified line, stored bytes unchanged after composition.
5. **MTs.** Claim only what is met: MT-31005 (closure_unit "resolved at composition"), MT-31007 (markers), MT-31008 (breaks), MT-31009 (flush space).

Hostile input (V4-01/02): marker counts and resolved text lengths are budgeted. Vendor parity UNVERIFIED (D5).

Out of scope: IDML / OOXML / ODF import-export mappings of sentinels (a later interop-layout lane), anchored-object marker placement (U+FFFC) beyond keeping it unshaped, any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit; no change to the type crate (report anything type must change).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composition is not a mutating command", "jobs": "none: composition stays a bounded call", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-layout-sentinels-1`, branch `lane/smb-layout-sentinels-1`, base 99836457)

- layout: the composition / reflow / flow / justification modules that need the change, a new sentinel-resolution module, `lib.rs` exports and DESCRIPTOR, their tests and new test files.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of layout and its direct dependents (RED COMMITS).
- One bundled run on C: (`lane-check.sh --owner smb-layout-sentinels-1`): step 0; clippy over layout and its direct dependents; nextest WHOLE suites of layout and its direct dependents that pin composition output (4a, 4b); doctests if added.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form (other lanes may push meanwhile; re-fetch and rebase on a non-fast-forward).
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
