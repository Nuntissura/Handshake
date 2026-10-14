---
file_id: STUDIO-APP-INSPECT-ADOBE
file_kind: research_note
updated_at: 2026-10-14T16:35:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-APP-INSPECT (read-only static inspector)
scope: Static data in installed Photoshop 27.11 / After Effects 26.5 / shared Adobe colour settings that bear on research/adobe-behavior.md and the D5 oracle case list; prose context, not authority
legend: "[V] read in an installed file at the cited path | [U] UNVERIFIED inference | [O] still needs the D5 black-box oracle"
---

<topic id="method" status="active" summary="Inputs, tools, limits">

## Method

- Versions, from file metadata (Get-Item VersionInfo; nothing executed) [V]:
  - Photoshop.exe 27.11 (20260929.r.26 8c16e56)
  - AfterFX.exe 26.5
  - InDesign.exe 21.6
- No app was launched.
- Prep harvesters: the docstrings of `_tools/greenroom/ae_render.py` and `ae_scripting.py` state they read only, and a grep found no subprocess, exec or COM dispatch in `ae_render.py`, `ae_scripting.py`, `ae_common.py` or `ae_panels.py`.
- `adobe-install-harvest.py` imports `win32com.client.makepy`/`gencache` (COM typelib generation, which writes a gencache). It was NOT run.
- Instead of rerunning the full harvesters, this lane used targeted byte and string reads (`strs.py`, `u16strs.py`, `csf.py` in the session scratch). The earlier harvest payloads (`_greenroom_20260903/installed_exports/**`) are listed in the manifest but absent on disk [V].
- Byte-pattern counts are not disassembly: no code was decoded.

</topic>

<topic id="color-settings" status="active" summary="Blend RGB Colors Using Gamma: shipped defaults">

## Shipped colour-settings presets (.csf)

- Location: `C:/Program Files (x86)/Common Files/Adobe/Color/Settings/{Recommended,ExtraSettings}/*.csf`.
- Format: an ICC-like tag table (`AsCs`, big-endian {sig, offset, size}) [V].
- Every preset read has `bge`=0 and `bgv`=100 [V]: Monitor Color, North America General Purpose/Newspaper/Prepress/Web Internet, Europe General Purpose 3 and Europe Prepress 3.
- `bge`/`bgv` = "Blend RGB Colors Using Gamma" enabled flag and value ×100 (1.00) [U: tag-name reading; strong].
- Other common values: `kpc`=1 (black-point compensation [U]), `cInt`=1 (Newspaper 0) (rendering intent [U]), `dith`=1, `mcp`=20 (monitor desaturation percent [U]).
- Consequence for `research/adobe-behavior.md`:
  - The community claim that the gamma blend is OFF by default (blending in gamma-encoded document RGB) now has shipped-file support for every factory preset.
  - The active user setting lives in the user's preferences and was not read. The D5 scripts must still read and restore it, as D5 decided.

</topic>

<topic id="photoshop" status="active" summary="Photoshop static facts">

## Photoshop 27.11

- **PSD tagged-block keys**: byte-reversed little-endian 4CC immediates (counts) in `Photoshop.exe` [V]:

  | Key | Count |
  |---|---|
  | `iOpa` | 4 |
  | `infx` | 4 |
  | `clbl` | 4 |
  | `knko` | 4 (+1 forward) |
  | `lsct` | 3 |
  | `lfx2` | 3 |
  | `brst` | 7 |
  | `SoLd` | 4 |
  | `lspf` | 4 |
  | `luni` | 4 |
  | `tsly` | 4 |
  | `vmsk` | 12 |

  `iOpa` has the same pattern as the documented keys. This supports the fill-opacity tagged block that `research/adobe-behavior.md` line 36 marks [U]. It is still not a spec citation.
- **Exposure presets** `Presets/Exposure/{Minus 2.0, Minus 1.0, Plus 1.0, Plus 2.0}.eap` are 14 bytes each: u16 version 1, then three big-endian f32 [V]:
  - exposure ±1/±2
  - offset 0.0
  - gamma 1.0

  Field order is inferred from the values: only the first float varies across presets [U, strong]. Good D5 case 6 inputs, but they say nothing about linear light [O].
- **Levels presets** `Presets/Levels/*.alv`: u16 version 2, then per-channel records of five u16 (input black, input white, output black, output white, gamma×100). For example, `Lighter.alv` = input 0..230, output 0..255, gamma 1.00, consistent with the PSD spec Levels block [V]. Composite-vs-channel order (case 2) [O].
- **Hue/Saturation presets** `Presets/Hue and Saturation/*.ahu` (v2, colorize flag) and **Curves** `Presets/Curves/*.acv` (9 presets incl. `Strong Contrast (RGB)`, already decoded by the lead) give exact parameter inputs for cases 3/4. Interpolation and clamping [O].
- Other preset families present for future oracle inputs: Black and White (12 `.blw`), Channel Mixer (6 `.cha`), Duotones, HDR Toning [V: `Presets/` listing].

</topic>

<topic id="after-effects" status="active" summary="After Effects static facts">

## After Effects 26.5

- **`BlendingMode` scripting enum**: 38 names, in a contiguous string table in `Support Files/Plug-ins/Keyframe/Scripting.aex` [V]:
  - `ADD, ALPHA_ADD, CLASSIC_COLOR_BURN, CLASSIC_COLOR_DODGE, CLASSIC_DIFFERENCE, COLOR, COLOR_BURN, COLOR_DODGE, DANCING_DISSOLVE, DARKEN`
  - `DARKER_COLOR, DIFFERENCE, DISSOLVE, DIVIDE, EXCLUSION, HARD_LIGHT, HARD_MIX, HUE, LIGHTEN, LIGHTER_COLOR`
  - `LINEAR_BURN, LINEAR_DODGE, LINEAR_LIGHT, LUMINESCENT_PREMUL, LUMINOSITY, MULTIPLY, NORMAL, OVERLAY, PIN_LIGHT, SATURATION`
  - `SCREEN, SILHOUETE_ALPHA` (AE's own spelling) `, SILHOUETTE_LUMA, SOFT_LIGHT, STENCIL_ALPHA, STENCIL_LUMA, SUBTRACT, VIVID_LIGHT`

  The count equals STU-CMP-020's 38 members, including the six non-ordinary modes. An importer must accept AE's `SILHOUETE_ALPHA` spelling.
- **Keyframe scripting surface** [V: same file]: `KeyframeEase(speed, influence)`, `setTemporalEaseAtKey`, `keyIn/OutTemporalEase`, `setSpatialTangentsAtKey`, `setTemporalContinuous/AutoBezierAtKey`, `setRovingAtKey`, internal `_getTemporalEaseDimensionRet`, `_getSpatialDimensionAndThrowIfNotSpatial`. Diagnostics:
  - `LStr/0135`: setting ease requires BEZIER interpolation and is refused on properties that forbid it (Marker).
  - `LStr/0130`: spatial calls require `TwoD_SPATIAL`/`ThreeD_SPATIAL`.
- Whether a spatial property takes ONE path-speed ease or one per component (`adobe-behavior.md` line 44 vs STU-MOT-033) is NOT stated in any string [O]. `valueAtTime` sampling in the D5 session answers it without rendering.
- **Effect match names**: 346 `.aex` under `Support Files/Plug-ins`, with about 344 distinct `ADBE ...` identifiers [V]. Examples: `ADBE Gaussian Blur 2`, `ADBE Drop Shadow`, `ADBE Levels`/`ADBE Levels2`, `ADBE Curves`, `ADBE Exposure`/`ADBE Exposure2`, `ADBE HUE SATURATION`, `ADBE Brightness & Contrast 2`, `ADBE Posterize`, `ADBE Threshold2`, `ADBE Glo2`, `ADBE Fill`, `ADBE Tint`. The full catalogue is what prep `ae_effects.py` produces; rerun it (read-only) when a lane needs the table.

</topic>

<topic id="d5-static" status="active" summary="Which D5 cases static data answers">

## D5 cases vs static data

Case numbers follow `diagnostics/adjustments-advanced-blending.md` and the effects lanes.

| Case | Question | Static answer? |
|---|---|---|
| setup | Gamma blend default | PARTLY: every shipped .csf has it off, value 1.00 (`bge`/`bgv`). The user's active setting is still read and restored by the D5 scripts. |
| 1 | Adjustment alpha on semi-transparent backdrop | No [O] |
| 2 | Levels composite vs channel order | No; preset encoding only [O] |
| 3 | Curves interpolation/clamp | No; preset points known (`.acv`) [O] |
| 4 | Hue/Sat | No; preset inputs only [O] |
| 5 | Brightness/Contrast modern vs legacy | No [O] |
| 6 | Exposure linear light | No; preset inputs `.eap` decoded [O] |
| 7-8 | Threshold metric / Posterize | No [O] |
| 9-10 | Blend-If ramp / mask density, feather | No [O] |
| 11 | Knockout shallow/deep | Key `knko` present; semantics [O] |
| 12 | Drop-shadow angle sign | No [O] |
| 13-17 | Effects details (glow jitter etc.) | No [O] |
| 18-27 | Blend-If with clipping/effects | No [O] |
| 28-29 | Masked / fill on Pass Through group | No [O] |

Net: no D5 case is fully answerable from installed files; the D5 session stays required. Statically settled side facts:
- shipped gamma-blend defaults
- `iOpa` handled as a tagged-block key
- the AE 38-mode enum, which matches STU-CMP-020
- exact preset encodings to use as oracle inputs

</topic>
