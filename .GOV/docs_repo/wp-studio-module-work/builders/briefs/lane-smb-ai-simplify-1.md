---
file_id: LANE-SMB-AI-SIMPLIFY-1
file_kind: lane_brief
updated_at: 2026-10-30T11:00:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-SIMPLIFY-1
lane_file: builders/lanes/hsk-studio-ai-simplify-1.json
crates: [hsk-studio-nib]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules
---

<topic id="target" status="active" summary="Illustrator Simplify (STU-VEC-114) as a nib geometry operation">

## Target

Engine only. Base: origin c719b1dc. AI-09 ("Offset path, simplify, outline stroke, expand, expand appearance") has every part VALIDATED except Simplify (`research/parity-matrix-20261020.json` note: "no Simplify command").

Read first:
- nib `src/freeform.rs`: `fit_freehand` (iterative RDP), `smooth_curvature`, `AnchorError`.
- `src/anchor_path.rs`: `AnchorPath`, `EditAnchor`, `Mirroring`, `to_owned_path` / `from_owned_path`.
- `src/edit.rs` `offset_path`, for the validation style.
- `src/shape.rs` `sin_cos_deg`.
- The nib DESCRIPTOR determinism rule: IEEE-exact operations only (+ - * / sqrt, floor/ceil/round, min/max); no platform trig, hypot or pow. Write your own IEEE-exact implementation.
- `kurbo 0.13.1` (already a nib dependency, Apache-2.0 OR MIT) has an experimental `simplify` module. It uses `atan2`/`hypot`. Read it as reference only; calling it needs a lead decision, because it breaks the determinism rule.

Push after each green slice, after the lead's go (see paths).

1. **`nib::simplify`** (new `src/simplify.rs`).
   - Parameters: `SimplifyParams {curve_precision, angle_threshold, smoothing, convert_to_straight_lines, show_original_path, preview}`. `Default` is the STU-VEC-114 table: 0, 0, 0, false, false, true.
   - API: `simplify(&AnchorPath, &SimplifyParams) -> Result<(AnchorPath, SimplifyReport), AnchorError>` over every contour, open and closed. The report gives anchors before and after per contour, plus the effective tolerance.
   - Studio reading (Illustrator's algorithm is UNVERIFIED, D5):
     - Sample each contour and split it at protected corners.
     - Reduce and refit each run with cubics within a tolerance derived from `curve_precision`:
       - 100 means tolerance 0. Only exactly redundant anchors are removed: collinear anchors on lines, zero-length segments, and anchors whose removal reproduces the curve within 1e-9.
       - Lower values raise the tolerance in proportion to the contour's bounding-box diagonal.
     - Define the mapping for EVERY finite value. STU-VEC-100: an unknown hard bound means accept any finite value, never clamp the parameter, never invent a limit. STU-VEC-102: the soft range is UI only. Declare your reading, for example: above 100 behaves as 100, below 0 continues the line, the parameter is echoed unclamped.
     - `angle_threshold`: an anchor whose interior angle is below the threshold is kept unchanged as a corner. Raising the threshold must protect a superset of corners. Compare angles with dot/cross products against `sin_cos_deg`.
     - `convert_to_straight_lines`: the output has zero-handle anchors only.
     - `show_original_path` and `preview` are display-only. They MUST NOT change the result (STU-VEC-114).
     - Precision 2: parameters are carried at 0.01. Declare the rounding rule.
     - Non-finite values are `AnchorError::NonFinite`. Existing anchor budgets apply.
   - Tests in new `tests/vector_simplify.rs`, red-first:
     - (a) the anchor count never increases;
     - (b) every original sample lies within the reported tolerance of the result, at precision 0, 50 and 90;
     - (c) precision 100 returns a path without redundant anchors byte-identically, and removes exactly the planted collinear and duplicate anchors;
     - (d) 150, -20 and 200 deg are accepted, echoed unclamped, and give a defined result;
     - (e) raising `angle_threshold` keeps a superset of corners;
     - (f) `convert_to_straight_lines` gives zero handles;
     - (g) the display-only flags do not change the output bytes;
     - (h) repeat runs are byte-identical;
     - (i) a closed contour stays closed with the same start anchor, and the contour count is kept.

2. **Smoothing.**
   - STU-VEC-114 `smoothing` is percent, precision 2. The smooth tool (STU-VEC-007) shares it, with a captured default of 20.
   - `simplify` applies `smoothing` after the fit.
   - Add a standalone `smooth(&AnchorPath, contour, smoothing)` and `SMOOTH_TOOL_DEFAULT = 20.0`.
   - Studio reading: non-corner handles move toward `smooth_curvature`'s Catmull-Rom tangent by the smoothing fraction. Protected corners are untouched. Out-of-range values are accepted per STU-VEC-100.
   - Tests:
     - 0 is the identity;
     - 100 equals `smooth_curvature`'s handles on non-corner anchors;
     - handle-angle discontinuity shrinks monotonically with smoothing;
     - corners are untouched.

3. **`OwnedPath` entry and descriptor.**
   - Add `simplify_owned(&OwnedPath, &SimplifyParams)` through `AnchorPath::from_owned_path`, so composite and interop callers can use it.
   - Add a `simplify` entry to `src/descriptor.rs` (parameters, readings, errors, test file).
   - The module rustdoc lists every Studio reading and marks Illustrator parity UNVERIFIED (D5).
   - Test: an `OwnedPath` with a quad segment survives simplify at precision 100 within 1e-9.

4. **(Optional, lead go only, no row gain) Blend geometry.**
   - Clauses: STU-VEC-032 (:4831) and STU-VEC-173 (:4836).
   - Build `nib::blend::blend_steps(a, b, spacing)` for the specified-steps and specified-distance spacings: anchor-count matching, then linear interpolation.
   - Smooth colour needs paint and is a typed refusal.
   - The `blend_spacing_mode` integers and the easing members were NOT recovered (STU-VEC-173). Use Studio identifiers and leave the captured values unknown.
   - A live blend record and rendering need Folio and composite: out of scope.
   - New `src/blend.rs` and `tests/vector_blend.rs`.

</topic>

<topic id="spec" status="active" summary="Spec basis (module 14 v02.225)">

## spec_basis

- STU-VEC-114 (:3216): the Simplify four-parameter table, display-only flags, and the smoothing shared with STU-VEC-007 (default 20).
- STU-VEC-100 (:2969): unknown bounds — accept any value, MUST NOT clamp, MUST NOT invent a limit.
- STU-VEC-102 (:2990): soft bounds are UI only.
- STU-VEC-103 (:2997): derived step = 10^-precision; record it in the descriptor as DERIVED.
- STU-VEC-106 (:3037): observed values are never bounds.
- STU-VEC-007 (:3270): smooth tool.
- Correction to `research/parity-20261023-adobe-print.json`, which says "Blend-tool clause NOT_FOUND by grep": STU-VEC-032 and STU-VEC-173 exist in v02.225.

</topic>

<topic id="paths" status="active" summary="Owned paths, do-not-touch, coordination">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-simplify-1`)

- nib:
  - NEW `src/simplify.rs` (and `src/blend.rs` in slice 4);
  - `src/lib.rs` (one `pub mod` line per new module);
  - `src/descriptor.rs` (`simplify` / `blend` entries only);
  - NEW `tests/vector_simplify.rs` (and `tests/vector_blend.rs`).
- Do NOT touch:
  - other nib modules. If you need a private helper from `freeform.rs` or `edit.rs`, a one-line `pub(crate)` hunk is allowed; name it in the notes and copy no code;
  - composite;
  - harness-view: the vector tools in `src/view/vector.rs` belong to SMB-R41-FOLLOWUPS-1, and a window Simplify command is a later GUI lane;
  - folio;
  - root `Cargo.toml`, `Cargo.lock`, `tools/`, `.config/`.
- SMB-AE-MASK-FEATHER-1 calls nib's `offset_path` read-only. Keep every existing nib signature unchanged.
- Coordination: nib is a dependency of layout and harness-view, which SMB-R41-FOLLOWUPS-1 is running. Push only after the lead's go.

</topic>

<topic id="proof" status="active" summary="Proof and tier">

## Proof

- **Bundled run, per push.** One bundled lane-check on C: over the WHOLE nib suite: `--disk ssd --owner smb-ai-simplify-1 --tests <slice tests> -- cargo nextest run -p hsk-studio-nib`.
  - The API is additive only (new modules and items), so the dependents' suites are not required. lane-check's clippy covers nib's dependents.
  - If an existing pub signature changes, stop and ask the lead.
- **Rules 4c-4e.** Run `--step0-only` first. Doc examples mean `cargo test --doc -p hsk-studio-nib`. nib has no features, so rule 4e does not apply.
- **No Folio change.** Rule 4b is not triggered.
- **Test style.** Red-first. Print `NOT_PROVEN:` lines for Illustrator Simplify parity (no D5 oracle case) and for the missing window command.
- **Model tier.** opus/high: spec parameter table plus declared geometric readings.

</topic>

<topic id="gain" status="active" summary="Estimated parity gain">

## Estimated parity gain

- AI-09, Illustrator (32 rows): PARTIAL -> BUILT_NOT_VALIDATED (+0.78 pp AI engine); VALIDATED after a round PASS (+2.34 pp).
- Pooled 9-app engine: +0.08 to +0.24 pp.
- Slice 4 (Blend) adds 0 pp. AI-27 stays PARTIAL because image trace, repeat and intertwine are absent.
- GUI: none.

</topic>
