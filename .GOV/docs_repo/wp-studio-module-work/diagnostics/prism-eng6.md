---
file_id: diagnostics-prism-eng6
file_kind: diagnostics
lane: SMB-PRISM-ENG-6
crate: hsk-studio-prism
input: builders/lanes/hsk-studio-prism-eng5.json (11 findings, T0/T1 oracle at baeecf3)
moxcms: 0.9.1 (latest on crates.io 2026-10-11; GitHub master trc.rs unchanged for the pure-gamma path); pxfm 0.1.30 (latest)
updated_at: 2026-10-11T05:02:14Z
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
