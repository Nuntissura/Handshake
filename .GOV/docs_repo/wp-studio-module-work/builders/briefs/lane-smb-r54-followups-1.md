---
file_id: LANE-SMB-R54-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-11-02T14:03:34Z
wp: WP-KERNEL-STUDIO
lane: SMB-R54-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r54-followups-1.json
crates: [hsk-studio-type (forced line break U+2028; terminator fast path), hsk-studio-layout (marker attribution before overset)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R54 F-R54-3 (U+2028), F-R54-6 and F-R54-4">

## Target

Read `validation/round-R54.json` F-R54-3, F-R54-4 and F-R54-6 (finding, repair_direction, file_line) and the lane notes of SMB-SENTINELS-NATIVE-1 (NATIVE-2) first. Spec: Master Spec v02.229 STU-TYP-207 (:9106-9160): the forced line break is a member of the break catalog.

1. **F-R54-3, forced line break.** Type refuses U+2028 with MissingGlyph. Per STU-TYP-207 the forced line break ends the LINE, not the paragraph: the engine treats U+2028 as a mandatory line break (UAX #14 class BK), never shapes it as a glyph, and keeps the stored code point. Bidi treats it per the spec / UAX #9 (U+2028 is class WS; it does not end the paragraph). Red test: a paragraph with U+2028 breaks the line there in the default and harfrust units; the NOT_PROVEN forced-line-break host test in interop-layout (`tests/sentinels_native_type.rs`) then proves it (turn its NOT_PROVEN into a real assertion).
2. **F-R54-6, marker at a line end before overset.** A marker at the end of a line directly before overset text is attributed to the line before. Fix the attribution so a marker resolves for the line (and page) it sits on; red test from the validator's case (read its probe in `Handshake_Artifacts/WP-KERNEL-STUDIO/validator-r54-probe/`).
3. **F-R54-4, terminator cost.** The public `hsk_studio_type::ends_paragraph` runs a full lookup per character. Measure it (a bounded micro-benchmark or a counted-operations test, no wall-clock assertion) on a long story; if the cost is material, add a fast path (e.g. ASCII and the common BMP range answered without the full lookup) that returns identical results for every code point (exhaustive agreement test over all code points). Record the measurement either way.
4. **MTs.** Claim only what is met: MT-31008 (breaks: forced line break row), MT-31005 / MT-31007 rows touched.

Out of scope: OOXML / ODF mappings; GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: shaping and composition are not mutating commands", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r54-followups-1`, branch `lane/smb-r54-followups-1`, base 96ace3ca)

- type: the engine's line-break / segmentation and validation modules, `src/validate.rs`, `src/lib.rs`, tests.
- layout: the sentinel-resolution and reflow modules (marker attribution only), tests.
- interop-layout: `tests/sentinels_native_type.rs` (turn the NOT_PROVEN case into an assertion).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on C: (`lane-check.sh --owner smb-r54-followups-1`): step 0; clippy over type, layout, interop-layout and direct dependents; nextest WHOLE suites of type (default and harfrust units), layout and interop-layout; doctests if added.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form (re-fetch and rebase on a non-fast-forward).
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>

<topic id="remediation-r55" status="active" updated_at="2026-11-05T07:43:42Z" summary="STUDIO-R55 FAIL_V1 remediation, F-R55-1 and F-R55-2">

## Remediation after STUDIO-R55 (FAIL_V1 at ce644577)

Read `validation/round-R55.json`: the lane's entry in `lanes` (failures, remediation, not_proven), `attributed_failures.probe_failures`, and F-R55-1, F-R55-2 in `findings_not_failures`. The probe source and log are in `Handshake_Artifacts/WP-KERNEL-STUDIO/validator-r55-probe/` (`a3-native.log`, the `r55_probe_native` test file).

1. **The failure: `forced-line-break-without-glyph-before-collapsed`.** `H LS LS I` gives two lines (`break_opportunities` lists Mandatory at bytes 4 and 7, but `opp_after` at `crates/hsk-studio-layout/src/paragraph.rs:409-424` folds a gap's opportunities into one kind, and `:556-558` finishes one line). `LS HI` makes no blank first line. All four composers do this; also `cjk.rs:599, :653-663`. Required: one line per mandatory break inside a glyph gap, and one before the first glyph. A blank line takes the same line metrics as the control `H NL NL I`. Red first from the validator's cases, in the default and harfrust units. V4-06: list every site that reads a gap's opportunities or finishes a line on a Mandatory, fixed or not applicable per site, with the regex used.
   - Re-check the marker rule `line_holding` (`sentinels.rs:591`) now that blank lines exist, and test a marker on a blank line.
   - A trailing forced line break before a paragraph end (probe n3) stays open for the D5 oracle: keep today's behaviour, declare it, raise it as a spec proposal.
2. **F-R55-1, lead decision: follow STU-TYP-130 as written, no spec exception.** Changing shaping semantics requires a new token and a stored document recomposes under its authored token. Add `hsk-type-shaped-runs@3` as `COMPOSITION_VERSION`, name the old one `COMPOSITION_VERSION_V2`, add `Semantics::V3`. U+2028 stays outside shaping under @3 only; under @1 and @2 it behaves as it did before this lane (a font without the glyph refuses MissingGlyph, a font with it shapes it). Every other `Semantics::V2` gate (`engine.rs:466`, `serialize.rs:183`, `provider/harfrust.rs:56`) keeps its V2 behaviour under V3. Red test: U+2028 with Inter refuses MissingGlyph under the @1 and @2 tokens and composes under @3, both providers. No fixture font maps U+2028, so the font-maps-it path stays unproven unless you find a permissive font that does: say so, do not add a font for it. Update only pins of the CURRENT token; never re-bless V1 or V2 outputs. Report whether the recorded, undoable upgrade STU-TYP-130 requires exists for @2 to @3 (a read, not a build). Blank-line layout needs no second gate if gaps with two Mandatory opportunities only arise under @3; verify that and state it.
3. **F-R55-2.** Since this lane U+2028 passes Type in every host. `harness-view src/view/text.rs:420-422, :584-592` classes area and point text from each glyph's source character and U+2028 has no glyph, so a forced line break in a Viewer text layer composes with no break and no refusal. Class the `NonrenderingControl` U+2028 coverage entry as a forced break there; where a path cannot express one, a typed refusal. Red first. No new egui, eframe, winit or wgpu line.
4. **MTs.** Re-claim only rows now met (MT-31008 forced line break; MT-31005 and MT-31007 rows touched).

Owned paths extended by the lead (V4-15): layout `src/paragraph.rs`, `src/cjk.rs`, `src/sentinels.rs`; harness-view `src/view/text.rs`; any test pin of the current token.

Proof: one bundled run on C: (`lane-check.sh --owner smb-r54-followups-1`; no round is running): step 0; clippy over type, layout, interop-layout and direct dependents; nextest WHOLE suites of type (default and harfrust), layout, interop-layout, and harness-view with its round feature unit. Rebase first (the worktree is 7 behind origin), rebase again before the push, `--per-sha`, guarded sha push on `PER_SHA_RESULT OK`, READY_FOR_VALIDATION on the pushed commit. SMB-LAYOUT-ENTRY-TEXT-1 works in the same crate in other files at the same time: whoever pushes second rebases. Do not wait for the lead to release this push.

</topic>
