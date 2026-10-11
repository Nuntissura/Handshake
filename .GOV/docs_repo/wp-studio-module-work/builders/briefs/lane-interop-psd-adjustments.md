---
file_id: BUILDER-BRIEF-LANE-INTEROP-PSD-ADJUSTMENTS
file_kind: builder_brief
updated_at: 2026-10-11T05:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-psd
lane: SMB-INTEROP-PSD-ADJ-1
depends_on: [folio ff92dea (StudioRasterRecords)]
decision: diagnostics/adjustments-advanced-blending.md
---

<topic id="scope" status="active" summary="PSD adjustment descriptors, blending ranges, mask parameters, knockout and lfx2 into folio raster records">

## Scope
Decode the following into `folio::StudioRasterRecords`, next to the existing document proposal. Keep every source block byte-stable in preservation, and remove each mapped key from the `adjustment_layer_not_applied` / `layer_effects_not_rendered` loss classes (`src/loss.rs:109-114`) only once the mapping is complete.

1. **Adjustment layers** (first set). For each, emit a `StudioLayer { kind: "adjustment", payload: PrimitiveReference { schema_id: "hsk.studio.adjustment@1", object_id: <its SLYR id> } }` plus a `StudioRasterLayerRecord.adjustment`.
   - `levl`: version u16 = 2, then 29 records of 5 x u16 (input floor 0..253, input ceiling 2..255, output floor, output ceiling, gamma x100 in 10..999). Record 0 is composite; records 1-3 are R/G/B for RGB documents. The optional `Lvls` tail carries extra records: preserve it, do not map it. Divide points by 255 into `normalized_0_1` (STU-RAS-152).
   - `curv`: both reference parsers read a leading extra byte before version u16 (1 or 4). psd-tools reads `BHI`; ag-psd reads a u8 then u16 == 1. Each curve is a u16 point count (2..19) followed by (output, input) u16 pairs. The `Crv ` extra block is v4 with u32 count and a u16 channel index per curve. Map composite/R/G/B; preserve the fifth/reserved curve and CMYK channels.
   - `hue2` (and `hue `): version 2, colorize u8, pad, colorization H/S/L int16, master H/S/L int16, then 6 x (4 x int16 range + 3 x int16 settings). Photoshop writes 136 bytes, 36 more than documented (psd-tools `psd/adjustments.py:513-518`); preserve the tail. Bands go into `bands`.
   - `brit`: int16 brightness, int16 contrast, int16 mean, u8 Lab-only. Read signed: psd-tools reads unsigned (`3HBx`), which is wrong for negative values. `CgEd` (descriptor v16 with `Vrsn`, `Brgh`, `Cntr`, `means`, `Lab `, `useLegacy`) takes precedence over `brit` when present. Map `use_legacy` from `useLegacy`.
   - `expA`: version u16 = 1, then f32 exposure, f32 offset, f32 gamma (big-endian). `nvrt`: no data. `thrs` / `post`: u16 value then 2 pad bytes.
2. **Blending ranges** (layer record): u32 length, then 4-byte ranges `[black0, black1, white0, white1]` (u8 each, ascending; default `[0, 0, 255, 255]`). The composite gray source is followed by the composite gray destination, then per channel source then destination. Map gray, R, G, B into `StudioAdvancedBlending.blend_ranges`; preserve other channels. Sources: ag-psd `psdReader.ts:523-541`, pytoshop `blending_range.py`, ag-psd fixture `test/read/blend-if`.
3. **Mask parameters**: when the mask flags have bit 4 set, a parameters byte follows. Bit 0 is user density (u8), bit 1 user feather (f64), bit 2 vector density, bit 3 vector feather. Both reference parsers read the real-mask block BEFORE the parameters (psd-tools comment at `layer_and_mask.py:934`). Map the user density to `density = byte / 255 * 100` (percent) and the feather to `feather` (px, f64). Bit 1 of the flags (disabled) maps to `StudioMask.enabled`; bit 2 (invert, obsolete) maps to `inverted`. Vector-mask parameters are preserved, not mapped (spec gap).
4. **`knko`**: one byte plus 3 pad. psd-tools reads it as `Knockout {NONE=0, SHALLOW=1, DEEP=2}` (issue 707: a Photoshop 2026 deep-knockout group stores 2); ag-psd collapses it to bool. Map 0 to `Disabled` and 1 or 2 to `Enabled`. Keep the raw byte in preservation and add a loss entry `knockout_depth_not_represented` when it is 2 (spec gap). `infx`/`clbl`/`tsly` (bool + 3 pad) map to `blend_interior_effects` / `blend_clipped_effects` / `transparency_shapes_layer`. `channel_blend` comes from the layer record's channel-blend flags only where they are verified; otherwise all true, recorded as a loss entry.
5. **`lfx2` / `lmfx`**: object-effects version u32 = 0, then descriptor version 16. Top-level keys are `Scl `, `masterFXSwitch`, `DrSh`, `IrSh`, `OrGl`, `IrGl`, `ebbl`, `SoFi`, `GrFl`, `patternFill`, `ChFX`, `FrFX`. The multi-instance lists are `dropShadowMulti`, `innerShadowMulti`, `solidFillMulti`, `gradientFillMulti`, `frameFXMulti` (ag-psd `descriptor.ts:1155-1184`). Merge each single slot and its multi list into ONE ordered list (STU-RAS-156), keeping both. Per-effect keys are `enab`, `Md  `, `Opct`, `Clr `, `uglg`, `lagl`, `Dstn`, `Ckmt`, `blur`, `Nose`, `AntA`, and `TrnS` for the contour (`MpgS` for satin). The result goes into `StudioEffectStack`. It is not rendered: keep the `layer_effects_not_rendered` loss entry until the effects wave. `lrFX` (legacy) is only preserved.

</topic>

<topic id="constraints" status="active" summary="Hostile input rules">

## Constraints (hostile bytes; workflow v4 topic `v4-rules`)
- V4-01: every new decoding module carries `#![deny(clippy::arithmetic_side_effects)]` and uses checked arithmetic with a typed error on overflow.
- V4-02: check the declared length against the remaining block and the running aggregate before allocating. Point counts must be within 2..19, and the 29+N levels records within the block length. Each decoder gets a `dims_u32_max` / oversized-count test and an `aggregate_over_budget` test.
- Descriptor parsing is bounded in depth, item count and string length (Accord budgets), so an unknown descriptor key is preserved, not an error.
- Byte-stable round trip of every touched block; unknown blend keys are never defaulted to Normal.
- V4-03: Photoshop display parity of the mapped records is `NOT_PROVEN` until a D5 oracle exists. Tests never read machine-local paths.

</topic>

<topic id="acceptance" status="active" summary="Acceptance">

## Acceptance (synthetic byte fixtures built in the test; at most 5 tests)
1. A `levl` block with composite (10, 245, 0, 255, gamma 150) produces `input_black 10/255`, `input_white 245/255` and `gamma 1.5`. A truncated block gives a typed error.
2. A `curv` block with the leading byte, v1, the RGB bitmap and two points `(out 255, in 0), (out 0, in 255)` produces a composite inverted curve. A point count of 20 is refused.
3. Blending ranges with gray source `[50, 100, 200, 240]` produce `source_black [50, 100]` and `source_white [200, 240]`. A length shorter than 8 bytes is refused.
4. Mask parameters density 128 produce `density = 50.196...`, and feather f64 12.5 produces `12.5`. `knko` = 2 produces `Enabled` plus a `knockout_depth_not_represented` loss entry, and the raw byte is preserved.
5. `lfx2` with `dropShadowMulti` [2 items] and a single `DrSh` produces a 3-instance stack in source order. The `validate_raster_records` result is `Ok`.

</topic>

<topic id="verification" status="active" summary="Cargo form">

## Verification
Use the studio cargo form with `-p hsk-studio-interop-psd --tests`. Commit with a pathspec, push, and update the lane file.

</topic>
