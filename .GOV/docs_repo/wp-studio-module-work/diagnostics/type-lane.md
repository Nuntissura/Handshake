---
file_id: WP-STUDIO-MODULE-WORK-DIAG-TYPE-LANE
file_kind: lane_diagnostic
updated_at: 2026-10-11T01:16:00Z
wp: WP-KERNEL-STUDIO
mt: MT-35536
crate: hsk-studio-type
---

# Type lane diagnostic (MT-35536, approved fix round 4)

<topic id="root-cause" status="active" mt="MT-35536" summary="FAIL_V3 refusal stage isolated to provider plan entry gate">

## Root cause

Cause: Resolved Common reaches provider plan gate admitting only Latn/Arab/Hani; ShapePlan::new refuses UnsupportedProfile.

Entry chain for original `mixed_ltr_isolate_and_trailing_space` piece 2..5 (U+2067 RLI, declared `Zyyy`, `und`, font 0 Inter-Regular), candidate 55c10969:

| Stage | Evidence (file:line at 55c10969) | Result |
|---|---|---|
| font coverage | `engine.rs:336` `fonts::covers` | pass (V3 counts show refusal later) |
| resolver | `engine.rs:586-623` `resolved_script` | returns `Zyyy` (all chars pinned default-ignorable) |
| script map | `engine.rs:350` `b"Zyyy" => rb::script::COMMON` | COMMON |
| plan entry | `engine.rs:377` -> `provider/rustybuzz/hb/ot_shape_plan.rs:57-63` | **refuses UnsupportedProfile** (only LATIN/ARABIC/HAN) |
| shape entry (next) | `provider/rustybuzz/hb/shape.rs:58-64` `shape_with_plan` | would refuse same profile |
| alternate entry | `provider/rustybuzz/hb/shape.rs:18-26` `shape` | same gate (no caller in crate) |

Downstream chain verified compatible with COMMON (no further gate):
- `ot_shaper.rs:156` categorize -> `DEFAULT_SHAPER` (same shaper as Latin).
- `tag.rs:241-296` COMMON -> `zyyy`; `ot_layout.rs:156-160` select_script falls back to `DFLT`.
- Inter-Regular (sha 529be850...) GPOS `DFLT` has `kern`, no legacy `kern` table (fontTools 4.62.1 read) -> kerning gate `ot_shape.rs:265-266` not hit.
- Inter cmap lacks U+2067/U+2069 -> `.notdef` (`ot_shape_normalize.rs:238`); `init_unicode_props` `buffer.rs:234-236` flags ignorable; `hide_default_ignorables` `ot_shape.rs:972-990` captures source groups then deletes with REMOVE flag.
- `capture_source_groups` `buffer.rs:1136-1165` keeps cluster 2 with `surviving_glyphs=0` -> engine `NonrenderingControl` coverage `engine.rs:540-547`; reference expects run `Zyyy/und`, 0 glyphs, coverage 2..5 `nonrendering_control`.

distinct_from:
- V1 `buffer.rs:1812` terminal `foreach_group` group_end at count (fixed 4c3a234f).
- V2 `engine.rs:587-611` Zyyy resolved to None before shaping (fixed 55c10969: resolver returns Zyyy -> COMMON).
- V3 (this): the 55c10969 COMMON value is refused one layer deeper, at both provider entry guards; V3 finding left "exact current refusal stage not isolated".

</topic>

<topic id="fix" status="active" mt="MT-35536" summary="Approved changed approach implementation">

## Fix (approved changed approach, one round)

- `ot_shape_plan.rs`: plan gate admits `COMMON` (default shaper only).
- `shape.rs`: both entries (`shape`, `shape_with_plan`) admit `COMMON` only via `admit_common`: non-empty buffer whose every scalar is a pinned default-ignorable (`hb::unicode::CharExt::is_default_ignorable`), else `UnsupportedProfile`.
- `shape_with_plan` post-condition: a COMMON buffer must end with zero surviving glyphs, else `UnsupportedProfile` (glyph-bearing Common keeps typed refusal).
- Unchanged: resolver, engine, fixtures, reference, admission/work/cancel bounds, REMOVE_DEFAULT_IGNORABLES flag gate.

Result: see topic `round-result`.

</topic>

<topic id="strategy" status="active" mt="MT-35536" summary="Provider strategy assessment; recommendation only, not implemented">

## Strategy: is the vendored rustybuzz subset the root-cause lane?

- Evidence: V1/V2/V3 all failed in the bounded adaptation layer added to the fork (helper bounds, resolver, Latn/Arab/Hani entry gates), not in upstream shaping logic. Each new script/profile means lifting gates in a 61.8k LOC private fork and re-proving it.
- Upstream: rustybuzz is archived; its README says to switch to HarfRust (github.com/harfbuzz/rustybuzz). crates.io: rustybuzz 0.20.1 (2024-11-12, matches HarfBuzz 10.1.0).
- HarfRust 0.14.0 (crates.io 2026-10-04): tracks HarfBuzz 14.5.1, mostly passes the HarfBuzz test/fuzz suites, `#![forbid(unsafe_code)]`, `no_std` via `default=["std"]`, parses fonts with read-fonts 0.45 (pinned source in Handshake_Artifacts/.../research/harfrust-0.14.0). It ships all complex shapers (arabic, indic, khmer, myanmar, USE, hangul, hebrew, thai). The MT's own independent reference oracle was produced with HarfRust 0.14.0.
- Field stacks: parley_engine 0.12 = harfrust + skrifa + icu_segmenter/icu_properties/icu_normalizer; cosmic-text 0.19 = harfrust + skrifa + swash + unicode-bidi + unicode-linebreak.
- Why the fork exists (MT research_basis, type-bounded-route-comparison.json): per-loop work metering, charged allocation, cancellation. HarfRust's shaper-font model uses Arc/Once and unmetered allocation. Per-allocation accounting inside HarfRust is not available (UNVERIFIED beyond that file).
- Bidi: unicode-bidi 0.3.18 (Unicode 16, already used as source) is fine. ICU4X icu_properties `bidi` feature gives a `BidiClassAdapter` for `unicode_bidi::BidiDataSource` if one Unicode data owner is wanted.
- Segmentation: icu_segmenter 2.3.0 (UAX#14/#29, Unicode-org maintained, 2026-08) preferred over unicode-linebreak 0.1.5 (last release 2023-07).
- swash 0.2.10: its shaper is not a HarfBuzz port. Use skrifa (fontations) directly for outlines/hinting/metrics, with swash/zeno only for rasterization if needed.

Recommendation:
1. Close MT-35536 on the current fork (in-flight pin; this round already approved). Do not switch provider mid-validation.
2. Open a separate MT/decision: replace the vendored rustybuzz + ttf-parser with pinned crates.io `harfrust =0.14.0` + `read-fonts`/`skrifa` as a separately selected provider leaf behind the existing Type engine contract. Enforce bounds at the boundary: input/glyph/run caps before the call, cancellation between runs, output allocation charged after the call, and optionally an isolated worker for hard limits. The cost is coarser in-shaper work metering, which needs explicit Operator acceptance against STU-ARC bounds.
3. Gain: the Type lane stops owning HarfBuzz parity (archived upstream, gate-by-gate script expansion), and the oracle equals the provider by construction.

Sources: https://github.com/harfbuzz/rustybuzz ; https://github.com/harfbuzz/harfrust ; crates.io API (harfrust, rustybuzz, read-fonts, skrifa, swash, unicode-bidi, unicode-linebreak, icu_segmenter, parley_engine, cosmic-text) ; https://doc.servo.org/icu_properties/bidi/struct.BidiClassAdapter.html

</topic>

<topic id="round-result" status="active" mt="MT-35536" summary="Fix round 4 implementation evidence; validator verdict pending">

## Round result (implementation evidence, not a verdict)

- Commit `e603db86031feb860039f59658273d8fe17d2b30` (tree `1de50152...`), pushed to `codex/handshake-studio-assets`; 2 files, +36/-3.
- check `--tests`: exit 0 (pre-existing dead-code warnings only). clippy `--tests`: exit 0, no findings on changed lines.
- Predecessor first: `mixed_script_bidi_font_runs --exact`: 1 passed (4.88 s), unchanged fixture and reference (sha 6579f9a9...).
- Bundle `--no-fail-fast`: Type `font_axis_missing_cancel_stale` 1 passed; Observe `foundation_contract` 19 passed.
- Tree note: shared edit tree (plus uncommitted steward root-manifest metadata), not an export of the pushed SHA. The independent round must re-run on an export.
- Not run: workspace-probe closure extension (rustc tool; no manifest change in this diff); independent consumer/allocation acceptance.
- MT-35536 lifecycle: READY_FOR_VALIDATION, blocked_on null; validator_verdict still FAIL_V3 until the independent round.

</topic>
