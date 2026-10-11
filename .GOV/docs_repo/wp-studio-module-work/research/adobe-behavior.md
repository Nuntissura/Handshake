---
file_id: STUDIO-ADOBE-BEHAVIOR-DELTA
file_kind: research_note
updated_at: 2026-10-11T04:00:00Z
wp: WP-KERNEL-STUDIO
lane: adobe-behaviour (researcher, Sonnet)
scope: GAP/DELTA note after coordinator scope change; prose context, not authority
legend: "[V] verified this session at cited source | [C] community/third-party, not Adobe | [U] UNVERIFIED | [O] resolve by black-box oracle probe"
---

<topic id="prep-coverage" status="active" summary="What existing prep already covers (do not redo) and what it lacks">

## 1. Already covered by prep (file refs under `.GOV/reference/studio_app_feature_research/` = SFR)
- Feature/tool/leaf inventories: SFR `01-photoshop-feature-map.md`, `03-indesign-feature-map.md`, `20-illustrator-feature-map.md`, leaf indexes 06/07/22, count method 28-33, UI export playbook 32.
- Primitives/commands/slice: `05-studio-primitive-map.md`, `10-studio-command-contracts.md`, `12-cross-app-parity-matrix.md`, `13-layer-graph-vertical-slice.md`. Note: slice `layer_node.required_fields` has opacity + blend_mode but NO fill-opacity, isolation/pass-through, knockout, blend-if, clipping-base field [V: 13 lines 74-75]; spec modules 14-04/05/06/08 mention fill opacity/knockout (42 grep hits, not re-read).
- Behaviour model (greenroom spec modules, `_greenroom_20260903/spec_update/modules/`): 14-27 STU-CMP-020 (38 blend members; six non-ordinary: stencil/silhouette alpha+luma, alpha add, luminescent premul), STU-CMP-021 (declared blend space + linearity), STU-CMP-022 (straight vs premultiplied), 14-08 STU-COL-162 (blending space); 14-26 STU-MOT-030a/031/033/034/035/036 (independent in/out interpolation, 5 members, `{speed,influence}`, default influence 1/6, roving, spatial tangents; DECLARED GAP STU-MOT-036b/141); 14-07 STU-TYP-135..140 (two-axis composer + registry, determinism); 14-04 parameter tables (levels/curves/exposure names, ranges, units).
- Original-suite audits `docs_repo/studio-collaboration-2026-10-09/adobe-original-team/*.json`: documented semantics only; own `unproved_acceptance` lists "Original-app-produced reference files/traces not acquired" and "Native Adobe GUI ... not executed" (original-suite-workflow-audit.json).
- Offline harvesters `_tools/greenroom/` (adobe-install-harvest.py, photoshop-parameter-surface.py, lightroom-develop-parameters.py, ae_scripting.py, indesign-dom-extract.py, uia-crawl.py, adobe-com-capture.py + `jsx/`).
- GAP 1: no numeric formulas in spec modules (my grep hits for Porter/W3C/Knuth/Newton were all "32000" false positives [V]). GAP 2: zero Adobe-produced reference outputs. GAP 3: existing exporters emit inventories only (see black-box-oracle-plan).

</topic>

<topic id="engine-formulas" status="active" summary="Missing engine-level formulas for first builder slices, with evidence status">

## 2. Blend / compositing (crates: composite, pigment, prism)
Source-over with blend, non-premultiplied colours [V: W3C Compositing 1, https://www.w3.org/TR/compositing-1/ s5.1, s9.1.4, s10]:
```
Cs' = (1-ab)*Cs + ab*B(Cb,Cs)      co = as*Cs' + (1-as)*ab*Cb      ao = as + ab*(1-as)      Cr = co/ao
algebra (mine): Cr = (1-as/ao)*Cb + (as/ao)*Cs'   (= PDF/ISO 32000 form; PDF text NOT fetched: Adobe addendum URL 404, ISO PDF >10MB) [U for PDF wording]
```
Separable B(Cb,Cs) [V: W3C s10.1]: multiply Cb*Cs | screen Cb+Cs-Cb*Cs | overlay=HardLight(Cs,Cb) swapped | darken/lighten min/max | colour-dodge 0 if Cb==0, 1 if Cs==1, else min(1,Cb/(1-Cs)) | colour-burn 1 if Cb==1, 0 if Cs==0, else 1-min(1,(1-Cb)/Cs) | hard-light Cs<=.5: Cb*2Cs else screen(Cb,2Cs-1) | soft-light Cs<=.5: Cb-(1-2Cs)*Cb*(1-Cb); else Cb+(2Cs-1)*(D(Cb)-Cb), D=((16Cb-12)Cb+4)Cb if Cb<=.25 else sqrt(Cb) | difference |Cb-Cs| | exclusion Cb+Cs-2CbCs.
Non-separable [V: W3C s10.2]: `Lum=.3R+.59G+.11B`; `SetLum(C,l)`: d=l-Lum(C), C+=d, ClipColor; ClipColor: L=Lum, n=min, x=max; if n<0: C=L+(C-L)*L/(L-n); if x>1: C=L+(C-L)*(1-L)/(x-L); `Sat=max-min`; SetSat scales mid by (mid-min)*s/(max-min), max=s, min=0 (all 0 if max==min). hue=SetLum(SetSat(Cs,Sat(Cb)),Lum(Cb)); saturation=SetLum(SetSat(Cb,Sat(Cs)),Lum(Cb)); color=SetLum(Cs,Lum(Cb)); luminosity=SetLum(Cb,Lum(Cs)).
Groups [V: W3C s8.2]: isolated group = transparent-black initial backdrop; non-isolated = group backdrop. Photoshop "Pass Through" ~ non-isolated, "Normal" group ~ isolated: mapping [U] [O]; Illustrator OMV exposes `PageItem/Layer.isIsolated` + `artworkKnockout` {Unknown,DISABLED,ENABLED,INHERITED} [V: omv.xml] (Illustrator blend set = exactly the 16 PDF modes [V: omv BlendModes]). W3C removed knockout (changelog) [V]; knockout formula source = ISO 32000 s11.4.6 [U, not read].
Photoshop-only modes (no public Adobe formula found): common definitions a=base b=blend [C, Wikipedia/Pegtop; hard-to-confirm] - linear burn a+b-1; linear dodge a+b; linear light a+2b-1; vivid light b<.5: 1-(1-a)/(2b) else a/(2(1-b)); pin light b<.5: min(a,2b) else max(a,2b-1); hard mix (a+b>=1)?1:0; subtract a-b (Wikipedia is internally inconsistent on this) ; divide a/b; darker/lighter colour = compare luminance sum, pick whole pixel; dissolve = pseudo-random dither on alpha. ALL [O].
Soft light divergence: Wikipedia cites a Photoshop variant `b<.5: 2ab+a^2(1-2b) else 2a(1-b)+sqrt(a)(2b-1)` (Pegtop-sourced, "as of 2012", not Adobe) vs W3C/PDF above [C] -> which one current Photoshop uses is [O]. Open PSD display-mismatch reports (START_LOG) make this first-priority.
PSD keys [V: https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/]: pass norm diss dark mul idiv(colour burn) lbrn dkCl lite scrn div(colour dodge) lddg lgCl over sLit hLit vLit lLit pLit hMix diff smud(exclusion) fsub fdiv hue sat colr lum; layer opacity u8 0-255, clipping u8 (0 base/1 non-base), flags (transp-protected, visible, pixel-data-irrelevant), blending ranges (blend-if) per channel src/dest pairs, `lsct` divider (0 normal,1 open,2 closed,3 bounding) with own blend key, `knko`, `infx` (blend interior as group), `clbl` (blend clipped as group), `lrFX/lfx2`, `vmsk`, `SoLd/SoLE`. `iOpa` (fill opacity) not confirmed in fetched excerpt [U]. Fill vs opacity: fill scales pixels only, not layer-style effects; 8 special modes react to fill differently [C: petapixel/photoshoptrainingchannel] [O].
Blend space: Photoshop default blends gamma-encoded document-RGB numbers; Edit > Color Settings > More Options "Blend RGB Colors Using Gamma" (1.00 = colorimetric/linear) is off by default [C: PetaPixel 2015-03-20, cgcookie; Adobe Help returned HTTP 403] [U for Adobe wording]. Community reports Pass Through groups misbehave under gamma 1.00 in PS 26.10 [C] -> engine special path [O]. 16-bit/32-bit rounding and Lab/CMYK behaviour: not found [U][O].

## 3. Adjustments (crate: pigment/develop). Parameter names [V: Photoshop `Required/UXP/com.adobe.photoshop.adjustments-panel/data/presets.json`]
levels{adjustment[]}, curves{adjustment[{channel,curve[{horizontal,vertical}]}],presetKind}, hueSaturation{adjustment[],colorize}, colorBalance{shadow/midtone/highlightLevels,preserveLuminosity}, vibrance{vibrance,saturation,useLegacy}, brightnessContrast{brightness,contrast,useLegacy}, photoFilter{color,density,preserveLuminosity}, blackAndWhite{red,yellow,green,cyan,blue,magenta,useTint,grain}, invert. ACV curves: 2-19 points, u8 pairs (output first), master then R,G,B [V: PSD spec]. No Adobe formula found for any adjustment (Adobe Help 403) -> baselines below are hypotheses, all [U][O]: Levels `out=ol+(oh-ol)*clamp((in-il)/(ih-il))^(1/gamma)`; Curves = spline through points (type unknown); Channel Mixer = 3x3+const matrix; Gradient Map = luminance -> gradient lookup; exposure = `(in*2^ev+offset)^(1/gamma)` in linear light.
Develop (crate develop): process version and slider vocabulary already harvested; preset sample `Color - Vivid.xmp` shows `crs:ProcessVersion="10.0"`, `*2012` tone names (Contrast/Highlights/Shadows/Whites/Blacks/Clarity), Vibrance, HSL x8 colours, SplitToning*, `ToneCurveName2012`, `CameraProfile` [V: C:/ProgramData/Adobe/CameraRaw/Settings/Adobe/Presets/Color/]. Order of operations and per-slider math: [U][O] (no public source fetched).

## 4. Keyframes / time (crates: motion, pulse)
Verified API semantics [V: https://ae-scripting.docsforadobe.dev/property/property/ and /other/keyframeease/]: types LINEAR, BEZIER, HOLD; separate in/out types; temporal continuous/auto-bezier only when both BEZIER; spatial continuous/auto-bezier/roving only on spatial props; first/last key cannot rove; `KeyframeEase(speed, influence)` influence 0.1..100, speed units "depend on keyframe type"; ease array length 2 (TwoD) / 3 (ThreeD) / otherwise 1 -> a SPATIAL position key has ONE path-speed ease, not per-component (STU-MOT-033 says "per component": check [O]); `valueAtTime(t, preExpression)`; `expression/expressionEnabled/expressionError`.
Proposed evaluation (derived, mine; consistent with STU-MOT-033 and Jam3 example influence 16.666666667 -> x 0.1667 [C: github.com/Jam3/ae-temporal-to-cubic-ease, MIT]) [U][O]:
```
seg [t0,t1], dt=t1-t0; out(s0,i0) at k0, in(s1,i1) at k1 (i as fraction 0..1)
P0=(t0,v0) P1=(t0+i0*dt, v0+s0*i0*dt) P2=(t1-i1*dt, v1-s1*i1*dt) P3=(t1,v1)   // (time,value) space
x(u) monotone for control x in [0,1]: solve x(u)=t (Newton + bisect fallback), value=y(u)
```
Community found AE 33% influence not exactly equal to CSS cubic-bezier(.33,0,.66,1) [C: Adobe community thread 2015] -> exact match is [O], and sampling via `valueAtTime` needs no render.
Drop-frame [V: https://en.wikipedia.org/wiki/SMPTE_timecode]: labels 0,1 skipped at start of each minute except minutes divisible by 10 (29.97 only in that article; 59.94 drops 4 [U]); 17982 labels per 10 min (arithmetic from rule). Label-only algorithm (derived): `D=f/17982, M=f%17982, f'=f+18*D+(M<2?0:2*((M-2)/1798))` then split f' at 30 fps; pulse stays integer ticks, DF is formatting only.

## 5. Paragraph composer (crates: layout, type)
Adobe behaviour [V via search snippets of helpx.adobe.com/indesign/using/composing-cjk-characters.html; Adobe pages 403 to fetch]: Paragraph Composer evaluates a network of break points over the whole paragraph, weighs penalties for letter spacing, word spacing, hyphenation; Single-line composer is line-at-a-time; hyphenation dialog trades spacing vs fewer hyphens; paragraph composer recomposes the entire paragraph per edit; inline-object text wrap can silently force single-line behaviour [C: CreativePro]. Illustrator OMV `ParagraphAttributes` confirms inputs [V]: desired/min/max word spacing, letter spacing, glyph scaling, hyphenation (minimumHyphenatedWordSize, minimumBeforeHyphen/AfterHyphen, maximumConsecutiveHyphens, hyphenationZone, hyphenationPreference), singleWordJustification, `composerEngine` {latinCJKComposer, optycaComposer, adornment}, `everyLineComposer`. Adobe's exact algorithm and weights are not public; "InDesign = Knuth-Plass" is NOT stated in the Wikipedia article [V negative] -> treat as analogy [U].
Reference algorithm [V: bramstein/typeset linebreak.js, MIT, as implementation evidence of Knuth-Plass 1981]: `r`=adjustment ratio (slack/stretch or slack/shrink; r<-1 infeasible); `badness=100*|r|^3`; `demerits=(L+badness)^2 (+pen^2 if pen>=0, -pen^2 if -inf<pen<0)`; `+flagged*f1*f2` consecutive flagged (100); `+3000` if adjacent fitness classes differ by >1 (classes: r<-.5 tight, <=.5 normal, <=1 loose, else very loose); tolerance 2, retry with larger. Mapping min/desired/max word spacing -> glue shrink/natural/stretch is my proposal [U][O].

</topic>

<topic id="first-slice-recommendations" status="active" summary="What each crate should reproduce first">

## 6. First-slice recommendations (hardest-to-regress first)
- composite/pigment: W3C separable + non-separable set with non-premultiplied maths; blend space parameter (encoded vs linear) as an explicit input; group `isolated` flag; golden = analytic vectors, then oracle ramps for Photoshop-only modes and soft-light variant.
- prism: gamma-encoded vs linear blend switch is a document-level setting; store in render receipt (matches STU-CMP-021).
- pigment adjustments: Levels, Curves (point list 0..255, 2-19 pts), Channel Mixer first; hold exposure/hue-sat/vibrance/selective colour until oracle data exists.
- motion: segment evaluator above + HOLD/LINEAR/BEZIER, independent in/out; test against `valueAtTime` samples from oracle; spatial = single path-speed ease.
- pulse: integer ticks + DF label function (test vector: frame 1800 at 29.97 -> 00:01:00;02).
- layout/type: Knuth-Plass core with Adobe-style inputs (word/letter/glyph spacing min/desired/max, hyphenation limits) behind a `composer` registry; single-line = first-fit; overset detection; determinism per STU-TYP-139.

</topic>

<topic id="installed-references" status="active" summary="Documented human-readable references present on disk (read-only)">

## 7. Installed references [V: listed this session]
- Illustrator: `C:/Program Files/Common Files/Adobe/Scripting Dictionaries CC/Illustrator 2026/omv.xml` (OMV 1.0, 757 KB, 299 classdefs: PathItem fill/stroke/dash/cap/join/miter/evenodd, PathPoint anchor/left/rightDirection/pointType, Gradient/GradientStop midPoint/rampPoint/opacity, TextFrameItem previousFrame/nextFrame/columns, Export*/ImageCapture/Rasterize options). Startup: `Common Files/Adobe/Startup Scripts CC/Illustrator 2026/`.
- Photoshop: `Required/UXP/com.adobe.photoshop.adjustments-panel/data/presets.json` (62 KB descriptor JSON); `Required/UXP/com.adobe.cai.uxp.panel/**/*.d.ts` (internal panel typings only, not the public DOM); `TypeLibrary.tlb` (registered COM typelib; prep walks it); `Presets/{Curves .acv, Levels .alv, Exposure .eap, Channel Mixer .cha, Hue and Saturation .ahu, Black and White .blw, HDR Toning .hdt, 3DLUTs .cube/.3dl/.look}` (formats partly in PSD spec; LUT licences [U], do not copy into assets).
- Lightroom/ACR: `C:/ProgramData/Adobe/CameraRaw/Settings/Adobe/Presets/**/*.xmp` (crs: documents), same tree under `Lightroom Classic/Resources/Settings`, `Resources/xmp_schema.json` (asset-payload XMP mapping), `Resources/{CameraProfiles,LensProfiles}`, `*.lrplugin` sample plug-ins (Flickr, AdobeStock) as SDK structure examples.
- InDesign: `Documentation/en_US/InDesign 2026 Scripting Read Me.pdf` NOT_INSPECTED (no PDF text tool); `Presets/default/Predef*.iddx` NOT_INSPECTED; `Common Files/Adobe/Startup Scripts CC/Adobe InDesign/`; `.hyp` hyphenation dictionaries under `Resources/Dictionaries` are proprietary binaries: do not use, use open patterns. DOM reference otherwise online (SFR `_source_snapshots/indesign-scripting-jina.md`, `indesign-uxp-dom-api-jina.md`).
- After Effects: `Startup Scripts CC/Adobe After Effects/aftereffects-26.0.jsx`; scripting reference online (ae-scripting.docsforadobe.dev, Property/KeyframeEase fetched [V]). Prep notes AE has never been run on this host (`_tools/greenroom/ae_render.py` docstring) [C: prep claim].
- Not found locally: standalone `.omv.xml`/`.d.ts` public DOM for Photoshop, InDesign, AE.

</topic>

<topic id="black-box-oracle-plan" status="active" summary="Plan to extend installed export scripts to render-reference exports under Operator-approved foreground session">

## 8. Black-box oracle plan (decision: no decompile; observe/test as licensed user per lead citing EU Software Directive art. 5(3); I did not verify the directive or the Adobe EULA text, lead/Operator own the legal call)
Existing `SFR/_installed_export_scripts/` [V: read]: `photoshop-export-inventory.jsx` (reflect props/methods of `app`+`activeDocument` + 51 seeded command names), `indesign-export-inventory.jsx` (menuActions, scriptMenuActions, menus, panels, toolBoxTools, reflect app), `illustrator-export-inventory.jsx` (reflect app/activeDocument + ~50 menu-command seeds), `normalize-adobe-installed-exports.py`. All three write to Desktop (portability defect) and end with `alert()` (modal, foreground). None exports pixels, layout or timeline data. `_tools/greenroom/indesign-live-capture.py` documents that the InDesign COM bridge wedges after a handful of calls (page small batches, restart/resume).
Extend (new `*-render-reference.jsx`, same `safe()`/JSON conventions; output under `Handshake_Artifacts/WP-KERNEL-STUDIO/oracle/<app>/`, manifest with app version, locale, colour settings, input hash, output hash; no `alert`; Illustrator `UserInteractionLevel.DONTDISPLAYALERTS` [V: omv]; Photoshop `DialogModes.NO`, InDesign `NEVER_INTERACT` [U]):
- Inputs = our own synthetic files only (Rust-generated 16-bit ramps, 17^3 colour lattice, known-text paragraphs, synthetic linear-DNG); outputs = numeric files (16-bit TIFF/PNG/EXR, JSON). Never ship Adobe presets/LUTs/binaries into assets.
- Photoshop: (P1) backdrop x source 256x256 grids per blend mode, 8/16-bit, opacity {100,50}, fill {100,50}; (P2) Normal vs Pass Through groups with nested blend; clip mask/knockout/blend-if on ramps; (P3) one adjustment layer per kind with parameter sweeps on gray ramp + lattice -> fits Levels/Curves/ChannelMixer/Exposure/HueSat/ColorBalance/Vibrance; (P4) Color Settings gamma toggle = GLOBAL setting change: record, flip, restore, needs explicit Operator OK; export via Save-As TIFF/PNG or UXP batchPlay [U exact calls].
- Lightroom/ACR: synthetic linear DNG + single-slider ramps via Camera Raw filter (descriptor `cameraRawFilter`) or LrC SDK plug-in (SDK is a separate download [U]); export TIFF; derive WB -> exposure -> tone -> HSL order by pairwise slider interactions; XMP round-trip of every `crs:` field.
- Illustrator: PathItem stroke cap/join/miter/dash/evenodd, gradient midpoint, blend/opacity/isolated/knockout via `exportFile` PNG24/TIFF with `antiAliasing`; note OMV exposes one fill + one stroke per PathItem, multi-fill appearance stacks need Actions/ArtStyles [U].
- InDesign: paragraph with width sweep x composer {Paragraph, Single-line} x hyphenation on/off x spacing limits -> line-break indices + per-glyph x from PDF export parsed by OUR parser; IDML export for structure; threading/overset via DOM frame links [U names].
- After Effects: `valueAtTime` dense sampling over (s0,i0,s1,i1,dt,dv) grid, 2D spatial vs non-spatial, hold/linear mixes, roving, expressions on/off -> no render needed; render probes (blend modes on solids, precomp collapse, time remap) via `aerender` if present [U]. First launch writes per-user templates/prefs (side effect, announce).
Foreground caveat: launching any Adobe app shows splash/sign-in/welcome windows and steals focus (GLOBAL-BUILD-QUIET). Run only in an Operator-approved session: one launch per app, batched probes, dialogs suppressed, offline/no cloud features, settings restored, then close what we opened (GLOBAL-INSPECT-014: never stop processes we did not start). UIA passive crawl (`uia-crawl.py`) is the only no-input observation path.
FLAG for lead: prep tools `ae_render.py`/`ae_scripting.py`/`adobe-install-harvest.py` extract embedded JSON/string tables and Eve layout sources from Adobe DLLs (e.g. PluginSupport.dll, BEE.dll, AfterFXLib.dll). That is string extraction, not disassembly, but sits near the "never read binaries" line you set; I used none of it. Confirm policy.

</topic>

<topic id="sources" status="active" summary="Sources opened this session">

## 9. Sources
- [V] https://www.w3.org/TR/compositing-1/ (blend formulas, groups; first 100k chars read)
- [V] https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/ (PSD keys, ACV; first 100k chars)
- [V] https://ae-scripting.docsforadobe.dev/property/property/ and /other/keyframeease/
- [V] https://en.wikipedia.org/wiki/Blend_modes ; https://en.wikipedia.org/wiki/SMPTE_timecode ; https://en.wikipedia.org/wiki/Knuth%E2%80%93Plass_line-breaking_algorithm (negative: no formulas, no Adobe mention)
- [V] https://raw.githubusercontent.com/bramstein/typeset/master/src/linebreak.js ; https://github.com/Jam3/ae-temporal-to-cubic-ease (README only)
- [C] web-search snippets: PetaPixel gamma article, cgcookie, Adobe community gamma threads, CreativePro composer paradox, helpx composing-cjk-characters
- FAILED: helpx.adobe.com blending-modes + color-settings (403); Adobe PDF blend addendum (404); PDF 1.7 reference (>10MB).
- Local files: paths in installed-references, prep files in prep-coverage.

</topic>
