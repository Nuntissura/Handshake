---
file_id: BUILDER-BRIEF-LANE-SMB-PS-SOFT-PROOF-1
file_kind: builder_brief
updated_at: 2026-10-24T01:17:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-PS-SOFT-PROOF-1
lane_file: builders/lanes/hsk-studio-ps-soft-proof-1.json
crates: [hsk-studio-prism]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip 1e3ae3d4); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules (+ prism-eng), builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, parity rows, expected gain, model">

## Target

- App: Photoshop (proof setup is also Illustrator/InDesign provenance per STU-COL-022; their rows NOT_INSPECTED). Row PS-28 "Soft proofing and gamut warning" is MISSING, rank 1 of the Photoshop gaps (parity-20261023-adobe-print.json apps.photoshop.top_gaps_remaining; spec STU-COL-022 :12434, STU-COL-240 :11064; "lane: none").
- Expected gain: PS-28 MISSING -> BUILT_NOT_VALIDATED when the auditors accept an engine-only proof pipeline (+1.4 pp Photoshop engine, 36 rows; +2.8 pp once validated), else PARTIAL (+0.7 pp) because the document-scoped proof state and the on-screen toggle are follow-ups (see gaps). 9-app engine pool +0.08 to +0.16 pp.
- Model / effort: opus / high (spec parameter table STU-COL-244, the six-field proof record STU-COL-241, ICC chain on the wide path).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-soft-proof-1`)

- NEW `crates/hsk-studio-prism/src/proof.rs` and `src/proof/**` (suggested `condition.rs`, `chain.rs`, `cvd.rs`, `gamut.rs`).
- `crates/hsk-studio-prism/src/lib.rs`: `mod proof; pub use proof::...;` and the DESCRIPTOR text for the new API (no other hunk; no READY lane touched prism).
- NEW `crates/hsk-studio-prism/tests/soft_proof.rs` (+ `tests/soft_proof_support/` if needed); reuse profiles `tests/profiles/sRGB-v4.icc`, `DisplayP3-v4.icc` (CC0) and `tests/oracle/profiles/hsk-synth-cmyk-v2.icc` (project-made synthetic CMYK); add new corpus files only, never rename.

## Do NOT touch

- `src/wide.rs`, `src/wide_*.rs`, `src/bpc.rs`, `src/icc_eval.rs` public types (no field added to `ClipReport`, `WideReceipt` or any pub struct: dependents build them by literal or pin them); call them, or add `pub(crate)` helpers in new files only. No other crate: the Folio proof-state record and the window toggle are follow-ups (Folio is owned by SMB-PR-MULTICAM-1 tonight, harness-view by the two GUI lanes).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-COL-022]`: "Studio MUST simulate a chosen output condition on screen using a device `StudioColorProfile` and a rendering intent WITHOUT converting document values, toggleable on/off, and MUST include color-blindness simulation profiles as proof conditions. The proof state is inspectable via Argus per [STU-COL-OBLIG-001]".
- `[STU-COL-023]`: "Rendering intents MUST include perceptual, relative colorimetric, saturation, and absolute colorimetric, with an independent black-point-compensation toggle, selectable both when converting to a profile ([STU-COL-004]) and when soft-proofing ([STU-COL-022])".
- `[STU-COL-240]`: "Soft-proofing simulates an output condition on screen WITHOUT converting document values. It is toggleable, its state is document-scoped and inspectable, and it never alters authority values ([STU-COL-164])."
- `[STU-COL-241]`: "A proof condition carries SIX fields: `proof_profile` (the device being simulated), `proof_intent` (document -> proof device), `display_intent` (proof device -> display), `black_point_compensation` (bool), `simulate_paper_color` (bool) and `simulate_black_ink` (bool). The TWO intents are separate ... a single-intent proof is wrong. `simulate_paper_color` and `simulate_black_ink` are separate booleans and are not implied by each other."
- `[STU-COL-242]`: "A proof condition is nameable, savable and selectable. Studio ships at least: current CMYK working space; current RGB working space; legacy uncompensated RGB; and the accessibility conditions of [STU-COL-243]."
- `[STU-COL-243]`: "Studio MUST ship colour-vision-deficiency proof conditions as first-class proof profiles, at minimum protanopia and deuteranopia, and MUST allow an operator-supplied condition. These are proof conditions, not a filter: they use the same pipeline, the same toggle and the same inspection state as any other proof."
- `[STU-COL-244]`: "An overlay marking every pixel or object outside the target gamut, with a configurable warning colour and warning opacity. It is computed from the clipping report of [STU-COL-138] and MUST be queryable as structured state, at minimum as an out-of-gamut area fraction and a bounding region list." Parameter table row: "| `gamut_warning.opacity` | 0 | 100 | 0 | 100 | 100 | percent | 0 |".
- `[STU-COL-138]`: "For an out-of-gamut value under `relative_colorimetric` or `absolute_colorimetric`, the engine MUST clip and MUST be able to report that clipping occurred per pixel or per object, because that report is what drives gamut warning ([STU-COL-244]). Under `perceptual` and `saturation` the engine uses the profile's own mapping tables and MUST NOT substitute a clip."
- `[STU-COL-245]`: "Where a picked colour is out of gamut, the picker MUST offer a one-click substitute that is the nearest in-gamut colour under the ACTIVE intent, and MUST show the delta. The substitution is an explicit operator or model action, never automatic."
- Accessibility row (grep `acc.gamut_coverage_readable`): "Gamut-warning coverage is readable as a number by a headless model".

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- Lane files `builders/lanes/hsk-studio-prism-eng6.json`, `hsk-studio-prism-eng7.json`, `hsk-studio-prism-eng8.json`, `hsk-studio-prism-view-transform-2.json` (wide path, BPC, absolute scaling, clip detection, view transform).
- Code: `src/wide.rs:1-11` (module doc: Prism-evaluated source stage -> PCS (BPC or absolute media-white applied once) -> destination stage), `:614 enum ClipMethod {NotComputed, LinearDevice, Unavailable}`, `:625 struct ClipReport` (aggregate counts, matrix/Gray destinations only: "table destinations cannot reveal clipping from their clamped output"), `:742 WideTransformRequest`, `:755 WideReceipt`; `src/lib.rs:177 enum Intent` (four intents); `src/bpc.rs`; `src/view_transform.rs` (display stage); `src/working.rs` (working profiles for the "current CMYK/RGB working space" presets).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; push per green bundled run)

1. Proof condition record and presets (STU-COL-241/242): `ProofCondition { name, proof_profile: ProofProfile, proof_intent: Intent, display_intent: Intent, black_point_compensation, simulate_paper_color, simulate_black_ink }`, `ProofProfile::{Icc(profile input), ColourVision(kind), OperatorSimulation(matrix)}`; validation with typed refusals (hostile profile bytes through the existing admission, V4-01/02); shipped presets: current CMYK working space, current RGB working space, legacy uncompensated RGB (Studio reading, declared: display stage is identity, device values shown unmanaged), protanopia, deuteranopia. Tests: `proof_condition_has_six_independent_fields`, `presets_include_the_spec_minimum`, `hostile_proof_profiles_are_refused_before_allocation`.
2. Proof chain (STU-COL-022/023/240): `soft_proof(document pixels, document profile, condition, display profile) -> ProofPreview` = document -> proof device (proof_intent, BPC) -> display (display_intent); `simulate_paper_color` = absolute media-white on the display side, `simulate_black_ink` = no black-point compensation on the display side (Studio readings of two independent booleans; Photoshop numerics UNVERIFIED); toggle off = the ordinary document -> display transform; the input buffer is never mutated and the receipt names both intents. Tests: `soft_proof_never_changes_document_values`, `toggle_off_equals_the_plain_display_transform`, `two_intents_are_applied_separately`, `paper_and_black_ink_flags_are_independent`, `proof_is_deterministic_across_threads`.
3. Colour-vision conditions (STU-COL-243): protanopia and deuteranopia as proof profiles in the SAME chain (linear-light simulation matrices written from the published paper, e.g. Machado, Oliveira, Fernandes 2009, severity 1.0; cite in the module doc), plus an operator-supplied 3x3 linear-light condition with bounds (finite, rows within a declared range). Tests: `cvd_conditions_run_through_the_proof_chain`, `cvd_neutral_axis_is_preserved`, `operator_condition_bounds_are_enforced`.
4. Gamut warning and in-gamut substitute (STU-COL-244/245/138): per-pixel out-of-gamut mask for the proof device: matrix destinations from the linear-device clip test; table (CMYK) destinations by round trip document -> proof -> PCS and a declared delta-E threshold (Studio reading: STU-COL-244 declares no tolerance); under perceptual/saturation proof intents the gamut check runs under relative colorimetric (declared reading of STU-COL-138). Structured state: out-of-gamut area fraction, bounded bounding-region list (declared cap), warning colour and `gamut_warning.opacity` 0..=100 default 100 integer (table row). `in_gamut_substitute(colour, condition) -> (substitute, delta_e)` under the active intent, explicit call only. Tests: `gamut_mask_marks_exactly_the_clipped_pixels`, `table_destination_gamut_check_uses_round_trip`, `gamut_opacity_bounds_follow_the_parameter_table`, `area_fraction_and_regions_are_bounded`, `substitute_is_in_gamut_and_reports_delta`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Pure engine tests (no GUI). Analytic expectations where the math is closed-form (matrix profiles, CVD matrices, neutral axis); for CMYK use the synthetic profile and assert invariants (in-gamut round trip under threshold, out-of-gamut above), never invented vendor numbers.
- NOT_PROVEN (`NOT_PROVEN:` lines): equality with Photoshop View > Proof Colors / Gamut Warning output (no proof case in the D5 Photoshop corpus); equality with LittleCMS proofing transforms (`cmsCreateProofingTransform` with gamut check) unless a checked-in oracle file is added (new file only).

## Bundled lane-check (rule 4a whole prism suite; clippy covers prism's direct dependents automatically)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-ps-soft-proof-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-ps-soft-proof-1 --disk d -- cargo nextest run -p hsk-studio-prism --no-fail-fast
```

- Rule 4a: prism only (public API additive; dependents do not pin new names, so their suites are not required; state it). Rule 4b: no Folio field -> package suite not required. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None. moxcms 0.9.1 is already pinned; no new crates.io dependency (CVD matrices are numbers from a paper, no code import).

## UNVERIFIED (D5)

- Photoshop proof-colour numerics for paper-white and black-ink simulation, Photoshop gamut-warning threshold and default warning colour, Illustrator/InDesign proof behaviour.

## Spec gaps found (Operator decides Master Spec edits)

- STU-COL-244 declares no out-of-gamut tolerance for table-based proof profiles and no default warning colour; STU-COL-242 does not define "legacy uncompensated RGB". The lane uses declared Studio readings and lists them for the HANDOFF spec-delta list.

## Gaps left for follow-up lanes (state in lane notes)

- Document-scoped proof state (STU-COL-240 "its state is document-scoped"): a Folio record + rule 4b package suite; not tonight (Folio is in SMB-PR-MULTICAM-1).
- On-screen toggle, proof preset menu and gamut overlay in the window (harness-view; not tonight).

## Risks and conflicts

- No READY lane touches prism. lane-check clippy spans prism's many direct dependents (composite, pigment, develop, render-cpu, interop-psd, harness...): a long hold on D:; expect it.
- The four lanes share the D: build lock with R39 off-disk on C:; runs serialise.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes; a push needs its green bundled run, so while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`) instead of pushing an unverified commit.

</topic>
