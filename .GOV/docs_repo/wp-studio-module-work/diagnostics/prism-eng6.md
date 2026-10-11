---
file_id: diagnostics-prism-eng6
file_kind: diagnostics
lane: SMB-PRISM-ENG-6
crate: hsk-studio-prism
input: builders/lanes/hsk-studio-prism-eng5.json (11 findings, T0/T1 oracle at baeecf3)
moxcms: 0.9.1 (latest on crates.io 2026-10-11; GitHub master trc.rs unchanged for the pure-gamma path); pxfm 0.1.30 (latest)
updated_at: 2026-10-11T05:45:35Z
---

<topic id="summary" status="current" summary="one root architecture cause plus one prism policy defect">

All numeric findings come from evaluating ICC math through moxcms 0.9.1 transforms: a fast pow without a zero
guard, a type-0 parametric curve that maps negatives to 0, 14-bit truncated curve lookups, 16-bit table inverses,
and re-baked 33^3 / 17^4 grids. No moxcms release fixes them (0.9.1 is the newest; `dirty_powf` unchanged on
master), so option (b) is unavailable. Fix route (a): Prism evaluates ICC math itself (f64, deterministic
basic-op pow/log) and uses moxcms only as the profile parser (`ColorProfile` fields are public). The missing
intent table refusal is a Prism policy defect against ICC.1:2022 8.10.2 c). No vendor/patch request needed.

</topic>

<topic id="findings" status="current" summary="cause, evidence, owner, fix per finding">

| Finding | Cause (<=120 chars) | Evidence | Owner | Fix |
|---|---|---|---|---|
| PRISM-CURV-GAMMA-ZERO-001 | curv count=1 uses dirty_powf: log2(0)=-127, exp2 exponent wraps in pow2if -> garbage (g1.8 -1.3e8, g2.0 -4) | moxcms trc.rs:1403-1405,1485-1497; pxfm powf.rs:829-841, log2f.rs:357-372, common.rs:306-308 | moxcms (+pxfm) | Prism TRC evaluator, pow(0)=0 |
| PRISM-RGB-BPC-CURV-GAMMA-001 | same garbage at vertex (0,0,0) -> source black L* -1.5e11 | bpc.rs darkest_l over moxcms leg; same lines | moxcms | same fix |
| PRISM-GRAY-BPC-001 | same; gray gamma 2.0 gives Y=-4.0 exactly (pow2if(-254) = 0xC0800000) -> L* -3613 | same lines | moxcms | same fix |
| PRISM-SOURCE-XYZ-NEGATIVE-CLAMP-001 | 2nd leg linearises companion para type 0: ParametricCurve type 0 has c=0, x<0 -> 0 | moxcms trc.rs:239-246,296-302; wide.rs companion `para` g=1 | prism companion + moxcms | no companion leg; PCS stays f64 unclamped |
| PRISM-HDR-SOURCE-CLAMP-001 | mAB M/B curves evaluated through 0..1-domain lookup tables; PCS above 1 is clipped | moxcms mab.rs:311-420 (build_linearize_table) | moxcms | Prism mAB evaluator: parametric curves unbounded |
| PRISM-LUT-DEST-ACCURACY-001 | PCS->CMYK re-baked on a 33^3 grid sampled linear in XYZ/D50; shadows interpolated in linear XYZ | moxcms lut_transforms.rs:560-579 (create_rgb_lin_lut of companion) | moxcms bake + prism companion | exact per-pixel B2A evaluation |
| PRISM-BPC-CMYK-BLACK-001 | local-black probe runs through the 17^4 re-baked A2B grid; 0.08 L* interpolation error | moxcms lut_transforms.rs:385,473; bpc.rs source black probe | moxcms bake | exact A2B evaluation in BPC probes |
| PRISM-LUT-SOURCE-REBAKE-001 | 7^4 source CLUT re-baked to 17^4 then 4D tetrahedral, differs from C-linear x 3D tetrahedral | moxcms lut_transforms.rs:385,473 | moxcms bake | exact CLUT (first-input linear, 3D tetrahedral) |
| PRISM-CURV-TABLE-INTERP-001 | f32 curve lookup index = trunc(x*16383) (14-bit), no interpolation | moxcms transform.rs:360-365; rgbxyz_float.rs TransformShaperFloatScalar | moxcms | linear interpolation of table entries |
| PRISM-TABLE-TRC-INVERSE-001 | destination curv table inverted to a u16 table (invert_lut/lut_inverse_interp16) | moxcms trc.rs:836-945 | moxcms | exact inverse of the piecewise-linear table |
| PRISM-MISSING-INTENT-TABLE-REFUSAL-001 | Prism refuses when A2B2 absent; ICC.1:2022 8.10.2 c) says use A2B0/B2A0 when the intent tag is absent | wide.rs:1094-1104 intent_table_check; ICC.1:2022 p.30-31 | prism | ICC 8.10.2 precedence: intent tag, else tag 0, else matrix/TRC |

Extra finding while reading: the second leg re-quantises nothing for matrix pairs, but every wide conversion runs
two moxcms legs plus a per-chunk PCS buffer; the develop lane measured ~110 ns/px.

</topic>

<topic id="fix-plan" status="current" summary="what Prism builds">

1. `src/icc_eval.rs`: deterministic f64 `ln`/`exp`/`pow` from IEEE basic operations (no libm, no FMA), ICC curves
   (curv 0/1/n, para 0-4) forward and inverse, CLUT interpolation (1 linear, 2 bilinear, 3 tetrahedral, 4+ linear in
   the first input over the rest, as lcms2), mft1/mft2/mAB/mBA pipelines with XYZ, legacy-v2 and v4 Lab encodings.
2. Plan = source stage -> Prism PCS map (BPC/absolute) -> destination stage, all f64; moxcms only parses.
3. Matrix/Gray pairs fuse into one affine 3x3 (`A = Mdst^-1 diag(D50) map diag(1/D50) Msrc`), identity TRCs skipped;
   `WideEngine::set_matrix_fusion(false)` keeps the staged path for the equivalence test.
4. Intent tables follow ICC 8.10.2 (intent tag, tag 0, matrix/TRC); `MissingIntentTable` only when none exists.
5. Receipts: `engine_name`/`engine_version` become the Prism wide evaluator constants; `engine_build` names the
   exact-f64 path; pinned determinism digests re-pinned (numeric path changed on purpose).

Cheapest proof: T0/T1 KNOWN_FINDINGS entries start failing-as-fixed; delete each, thresholds untouched.

</topic>

<topic id="status" status="current" summary="every PRISM finding fixed; proof per finding">

All 11 fixed (commits 3926709, fdd57ee, dc42e34, d9a8ce8 on codex/handshake-studio-assets, pushed). Proof: the
KNOWN_FINDINGS entry of each flipped ("no longer reproduces") and was deleted; thresholds untouched; full suite
120/120 at 987af00. T0 registry is empty.

| Finding | Proof |
|---|---|
| CURV-GAMMA-ZERO, RGB-BPC-CURV-GAMMA, GRAY-BPC | T0 curv_gamma_* / srgb_curv18_* / gray_v21_*; T1 src_leg_prophoto, m_prophoto/m_srgb_to_prophoto BPC; interop_psd_gray_bpc_repro_is_fixed (BPC on == off, black L* 0, scale 1) |
| SOURCE-XYZ-NEGATIVE-CLAMP | T0 p3_para3_to_xyz_d50; T1 src_leg_image_p3/acescg/synth_rgb_v2_table |
| HDR-SOURCE-CLAMP | T1 src_leg_rec2100_hlg203 passes the LUT gate against lcms2, whose PCS Y reaches 4.92 |
| LUT-DEST-ACCURACY | T1 dst_leg_swop/fogra39/synth_cmyk_v2 and every RGB/CMYK->CMYK relative/perceptual/saturation pair |
| BPC-CMYK-BLACK | T1 l_swop/fogra39/gracol2006_to_srgb relative BPC (black 16.589 = lcms2 16.59) |
| LUT-SOURCE-REBAKE | T1 l_synth_cmyk_to_srgb saturation |
| CURV-TABLE-INTERP, TABLE-TRC-INVERSE | T0 curv_table_to_linear, curv_table_dense_gray_scan, lab_to_srgb_curv_table1024_clipped |
| MISSING-INTENT-TABLE-REFUSAL | T1 l_rec2100_hlg203_to_srgb saturation; wide_profiles A2B0-only profile: relative == perceptual |

Found while fixing (not an accuracy defect): lcms2 interpolates the 3D CLUT of a Lab-PCS output table trilinearly
(`_cmsReadOutputLUT` -> `ChangeInterpolationToTrilinear`); exact tetrahedral differed from lcms2 by 0.12 magenta on
the 13^3 synthetic B2A1, trilinear reproduces lcms2 to 1e-6. Prism adopted the lcms2 convention (decision to
acknowledge). Residual T1 combos re-attributed with evidence: 8 RGB->CMYK absolute -> DIV-ABS-V2-DISPLAY-WTPT-001
(with the source wtpt forced to D50 max 0.0055); SWOP->FOGRA39 perceptual/saturation BPC ->
DIV-CMYK-PERCEPTUAL-BPC-BLACK-001 (BPC off max 0.36/0.73; engine black 0.86 vs lcms2 0.0); HLG saturation BPC on ->
DIV-V4-PERCEPTUAL-BLACK-001 (same numbers as perceptual: A2B0 fallback). Only DIV-* decisions remain.

Fusion: tests/wide_fusion.rs, 7 pairs (linear, para, curv 1.8, BPC, absolute, Gray): fused == staged 100% bit-identical,
identical clip reports. Release: linear pair 34.5 ns/px (develop measured ~110 on the old legs), sRGB->P3 187 ns/px.

</topic>

<topic id="r5-review-a" status="current" summary="adversarial review R5 group A items for this crate">

1. fdd57ee unpushed: pushed (origin contains every prism commit through 987af00).
2. clippy approx_constant at icc_eval.rs:31-32: fixed in d9a8ce8 (fdlibm ln2 split as `f64::from_bits`, std `LOG2_E`).
3. Hard-coded C: profile paths and silent green skips (SMB-PRISM-ENG-5): fixed in bac4790. `tests/oracle/corpus.json`
   names the external root (`../../../../Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/icc` from the crate);
   `tests/oracle/populate_corpus.py` copies SHA-256-verified profiles from the fixture's system directories (tool
   reads only); tests read only the corpus root (plus `HSK_PRISM_ORACLE_PROFILE_DIRS`); a missing profile prints
   `NOT_PROVEN: <test> <case> reason=...` on uncaptured stderr and fails under `HSK_PRISM_REQUIRE_CORPUS=1`.
   Negative proof run: ProPhoto hidden -> 4 NOT_PROVEN lines, exit 0; with the flag exit 101; restored.
4. Admission-ledger hook (D13 option b): `AdmittedWideEngine` in e7cf81f (see lane notes for the charging model);
   develop's owner consumes it.

</topic>

<topic id="v4-rules" status="current" summary="workflow v4 applied to this lane">

V4-01/02 in 987af00: `#![deny(clippy::arithmetic_side_effects)]` on icc_eval, wide, wide_admitted, wide_plan, bpc,
admitted (123 sites fixed); tests/wide_hostile.rs covers u32-max tag offset/size/count, a u32::MAX `curv`, 255^4 mft2 and
255^3 mAB CLUTs, a >2 MiB profile, and an aggregate over the ledger budget refused before any parse. V4-03 markers and
corpus root as above; V4-06 site lists (class, regex, every site) in builders/lanes/hsk-studio-prism-eng6.json
notes.v4_06_site_lists; V4-07 absolute claims cite tests or were weakened; V4-09 commit via `git commit -F`.

</topic>
