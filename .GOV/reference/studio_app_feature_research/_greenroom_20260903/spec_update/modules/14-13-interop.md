---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-13"
section_id: "14.13"
title: "14.13 Import/Export & File-Format Compatibility"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "14.13 in .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md lines 2724-2888"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-13-interop.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.13 Import/Export & File-Format Compatibility

This sub-section is the normative interoperability contract for Studio. v02.205 defined the two primitives, the five-artifact promotion gate, the forty-seven-row format matrix, the preservation-blob law and the receipt law. It did NOT define what an export SETTING is, and every export surface in Studio is built out of settings. This module supplies that: the export parameter model, the settings record shapes for each output class, the enumerated value domains, and the bounds.

### 0. Baseline, supersession and disposition

**[STU-IO-100] Baseline preservation and supersession.** Clauses [STU-IO-001] through [STU-IO-014] of v02.205 remain in force in full: the native-format declaration, the four-part support contract, the colour-and-unit explicitness rule, the five-artifact promotion gate, the preservation-blob law, the forty-seven-row matrix and its row-law, the NRT family list, the fonts/profiles/links contract, the unified export-surface list, the fixture law, the recovery-and-diagnostics law, the provider-adapter posture, and the model-steerability obligation. Clauses [STU-IO-100] and above add the parameter-level contract. Explicit corrections:

| v02.205 clause | Disposition | Replacement |
|---|---|---|
| [STU-IO-010] "PDF export: option groups for preset/standard, general, compression, output, security" | EXTENDED | The PDF export setting surface is 149 named settings in ten groups with declared value domains ([STU-IO-125] through [STU-IO-133]). |
| [STU-IO-010] "Video/animation render: timeline to encoder presets or image sequences" | EXTENDED | The video export parameter model is [STU-IO-140] through [STU-IO-148]; it carries a twelve-member parameter type system and a tick-based frame-rate encoding. |
| [STU-IO-010] "Single-target export ... per-document, per-layer, per-artboard" | EXTENDED | Node-render settings are five closed record shapes ([STU-IO-111] through [STU-IO-116]). |
| [STU-IO-006] row 46 "Web / markup output" | BOUND | Web output is produced under the contract of [STU-WEB-120]; the matrix row is unchanged, its writer is now specified. |

**Reserved numbering gaps in this module.** The ranges 118-124, 149, and 155-159 are RESERVED and deliberately unused, so each section of this module owns a contiguous band: 101-117 the parameter model and node-render settings, 125-133 document PDF export, 134-139 still-image export, 140-148 professional video export, 150-154 preset and library import, 160-167 obligations. No clause has been retired from those ranges; nothing is missing.

---

### 1. The export parameter model

**[STU-IO-101] `StudioExportRecipe` is a parameter set, not a switch.** A recipe is `{recipe_id, name, target_format, parameters{}, source_scope, output_targets[]}`. `parameters` is a map from parameter identifier to a typed value under the model below. A recipe MUST be fully declarative: given a recipe and a document revision, the output is determined ([STU-IO-163]).

**[STU-IO-102] Export parameter record (normative shape).** Every export parameter Studio exposes carries:

| Field | Required | Semantics |
|---|---|---|
| `identifier` | yes | stable machine identifier, never reused |
| `label` | yes | human label |
| `description` | no | tooltip text |
| `type_code` | yes | a member of [STU-IO-103] |
| `facet` | yes | a member of [STU-IO-104] |
| `group_path` | yes | ordered path of container identifiers naming where this parameter sits in its surface |
| `hard_min` | yes or explicitly `unknown` | the value the encoder will accept |
| `hard_max` | yes or explicitly `unknown` | |
| `soft_min` | yes or explicitly `absent` | the range the control presents |
| `soft_max` | yes or explicitly `absent` | |
| `default` | yes | |
| `unit` | yes or explicitly `dimensionless` | the real unit token, never a guess |
| `precision` | yes | declared decimal places |
| `step` / `coarse_step` / `fine_step` | yes | scrub increments |
| `enumerated_values` | when `type_code` is enumerated | ordered value list |
| `is_slider` | yes | whether the control presents as a slider |
| `is_hidden` | yes | whether the parameter is hidden in the default surface |
| `used_by_formats` | yes | the container formats that carry it |

**[STU-IO-103] Hard bounds and soft bounds are SEPARATE fields and MUST NOT be collapsed.** `hard_min`/`hard_max` are what the encoder accepts; exceeding them is an error. `soft_min`/`soft_max` are what the control presents by default; a user or a model may type past them but not past the hard bounds. Where a source declares only one pair, Studio MUST emit both fields and mark the undeclared pair explicitly `absent` — NEVER equal to the declared pair. Collapsing them is irreversible: once written as one range the distinction cannot be recovered without re-deriving it from the source captures. Where neither is declared, both are `unknown` and Studio MUST NOT invent a bound; a parameter with unknown hard bounds is passed to the encoder unclamped and its error is surfaced from the encoder, not pre-empted by a fabricated clamp.

**[STU-IO-104] Export parameter type-code enumeration (normative, closed, twelve members).**

| Code | Token | Meaning |
|---|---|---|
| 1 | `group_header` | a container with no value of its own |
| 2 | `signed_integer` | |
| 3 | `floating_point` | |
| 4 | `boolean` | |
| 5 | `string` | |
| 6 | `enumerated_integer` | an integer chosen from a constrained list |
| 7 | `action` | a button; performs an action, stores no value |
| 8 | `tab_group` | a tab or section container |
| 9 | `path` | a file or folder path string |
| 10 | `opaque` | arbitrary data the surface passes through untouched |
| 11 | `repeating_group` | a multi-instance group of rows |
| 12 | `colour` | a colour value, carrying an explicit `StudioColorProfile` reference ([STU-DOC-003]) |

Codes 1, 8 and 11 are STRUCTURE, not values: a recipe MUST preserve them so a surface can be reconstructed, and MUST NOT flatten them away.

**[STU-IO-105] Export parameter facet enumeration (normative, closed, fourteen members).** Every parameter declares exactly one facet: `video_codec`, `video_frame`, `rate_control`, `gop_structure`, `colour`, `audio`, `multiplexing`, `captions`, `layout`, `metadata`, `performance`, `vr_immersive`, `publishing_destination`, `other`. Facets are how a surface is organised for a model that has never seen it; the shipped reference distribution across 453 distinct video-export identifiers is 178 `other`, 86 `publishing_destination`, 41 `video_codec`, 36 `audio`, 34 `rate_control`, 22 `colour`, 16 `layout`, 13 `gop_structure`, 10 `multiplexing`, 5 `video_frame`, 5 `vr_immersive`, 4 `captions`, 2 `metadata`, 1 `performance`.

**[STU-IO-106] Label coverage is honest.** In the reference video-export surface, only 44 of 453 distinct parameter identifiers carry a shipped human label — under ten percent. Studio MUST NOT pretend otherwise: a parameter with no recovered label MUST display its identifier, MUST be marked `label_source: identifier_fallback`, and MUST be listed in the UserManual coverage report as unlabelled. Inventing a label is forbidden.

**[STU-IO-107] Enumerated option names may be encoder-supplied.** For an `enumerated_integer` parameter, the option NAMES may be supplied by the encoder at runtime rather than serialised into any preset. What is recoverable offline is the integer value set the shipped presets actually use. Studio MUST therefore: query the encoder for option names when one is available; fall back to the observed integer set with `label_source: observed_values_only` when it is not; and NEVER guess a name. A recipe stores the INTEGER, not the name, so it survives an encoder update that renames options.

**[STU-IO-108] Codec identifiers are exporter-local.** A codec-selection integer is local to its exporter; the same integer means different codecs under different exporters. A recipe MUST therefore store the exporter identity alongside any codec integer, and a recipe whose exporter is unavailable MUST fail closed naming the exporter, never fall back to a different exporter's meaning for the same integer.

**[STU-IO-109] Group path is authority.** `group_path` is an ordered array of container identifiers, for example `["root", "audio_tab", "basic_audio"]`. It reconstructs the surface tree without a second layout file, and it is what lets a model ask "what are the audio settings" without knowing the surface. Group paths MUST be stable across versions; renaming a group is a breaking change to the recipe schema.

**[STU-IO-110] Recipe portability and registration.** A recipe is an authority record, exportable as a portable artifact registered into CKC per [STU-ASSET-012]. Importing a recipe whose `target_format` or exporter is unavailable MUST report the missing component and import the recipe in an unresolvable state, never silently substitute.

---

### 2. Node-render settings (the per-node export surface)

**[STU-IO-111] Node-render setting record set (normative, five closed shapes plus one).** A node, layer, artboard or slice declares an ordered array of render settings. Each is exactly one of these shapes.

**[STU-IO-112] Raster image settings.** `{format, contents_only, use_absolute_bounds, suffix, constraint, colour_profile}` where `format` ∈ {`PNG`, `JPG`}; `contents_only` (boolean, default true) excludes overlapping content outside the node; `use_absolute_bounds` (boolean, default false) exports the node's full bounds including clipped content; `suffix` is the filename suffix appended for this setting; `constraint` is the record of [STU-IO-114]; `colour_profile` ∈ {`DOCUMENT`, `SRGB`, `DISPLAY_P3_V4`}.

**[STU-IO-113] Vector and document settings.**

- **SVG:** `{format: "SVG", contents_only, use_absolute_bounds, suffix, colour_profile, outline_text, include_id_attribute, simplify_stroke}`. `outline_text` (boolean) converts text to paths; `include_id_attribute` (boolean) emits node ids as `id` attributes; `simplify_stroke` (boolean) reduces stroke geometry. A string-returning variant `SVG_STRING` produces the markup in-memory rather than as a file and is what a codegen or web-authoring caller uses.
- **PDF:** `{format: "PDF", contents_only, use_absolute_bounds, suffix, colour_profile}`. This is the NODE-render PDF, distinct from the document PDF export of [STU-IO-125].
- **Serialised tree:** `{format: "JSON_TREE"}` — the typed node tree of [STU-AUT-022]. It carries no image options.

**[STU-IO-114] Export constraint record (normative).** `{type, value}` where `type` ∈ {`SCALE`, `WIDTH`, `HEIGHT`}. Contract for `value`:

| Field | `SCALE` | `WIDTH` / `HEIGHT` |
|---|---|---|
| hard_min | > 0 | 1 |
| hard_max | NOT DECLARED IN SOURCE; Studio declares 100 | NOT DECLARED IN SOURCE; Studio declares 100000 |
| soft_min | 0.25 | 16 |
| soft_max | 4 | 8192 |
| default | 1 | the node's intrinsic dimension |
| unit | ratio (dimensionless) | pixels |
| precision | 4 decimal places | 0 decimal places |
| step / coarse_step / fine_step | 0.25 / 1 / 0.05 | 1 / 100 / 1 |

Multi-scale variants (1x, 2x, 3x) are N settings each with `type: SCALE` and its own `suffix`, not one setting with a list.

**[STU-IO-115] Animated raster settings.** `{format: "GIF", fps, loop_count, constraint}` where `fps` is drawn from the closed set {8, 12, 15, 24, 30}; `loop_count` is an integer with hard_min 0 (0 = infinite), hard_max NOT DECLARED IN SOURCE (Studio declares 65535), default 0, unit = count, precision integer; `constraint` uses [STU-IO-114] restricted to `HEIGHT`, `SCALE` or `WIDTH`.

**[STU-IO-116] Video settings.** `{format, fps, quality, constraint}` where `format` ∈ {`MP4`, `WEBM`}; `fps` is drawn from the closed set {12, 24, 30, 60}; `quality` ∈ {`LOW`, `MEDIUM`, `HIGH`}; `constraint` uses [STU-IO-114]. These are the SIMPLE video settings for the design and motion surfaces. The professional video export surface is [STU-IO-140] and above and is a different, much larger parameter set; a caller MUST choose one and MUST NOT expect one to accept the other's parameters.

**[STU-IO-117] Format-suffix and collision rule.** Two render settings on one node MUST NOT produce the same output filename. A collision is a validation error at authoring time, not a silent overwrite at export time.

---

### 3. Document PDF export

**[STU-IO-125] PDF export preset contract.** A document PDF export preset is a named, complete parameter set. The reference baseline ships 13 presets across two locale families, spanning 149 distinct settings, with each preset declaring between 145 and 148 of them. Studio's shipped preset set is its own; the CONTRACT is that a preset is COMPLETE — a preset declaring a subset of settings, with the rest resolved from an ambient default, is not admissible, because the ambient default is what makes a "known-good" preset produce different output on two machines.

**[STU-IO-126] PDF setting group set (normative, ten groups).** The 149 settings organise into: `general` (compatibility, page range, thumbnails, optimisation, binding, rotation, object compression), `fonts` (embedding policy, subsetting, always-embed and never-embed lists), `colour_images`, `greyscale_images`, `monochrome_images`, `colour_management`, `output_intent`, `standards_compliance`, `postscript_and_dsc`, `security`. Every setting declares its group.

**[STU-IO-127] Image-downsampling setting family (normative shape, applied three times).** Each of the colour, greyscale and monochrome image groups carries the same eleven-setting family: `anti_alias` (boolean), `crop_to_frame` (boolean), `minimum_resolution` (integer, ppi), `minimum_resolution_policy` (enumeration), `downsample` (boolean), `downsample_type` (enumeration), `target_resolution` (integer, ppi), `bit_depth` (integer; -1 means "leave unchanged"), `minimum_downsample_depth` (integer), `downsample_threshold` (float ratio), `encode` (boolean), plus the codec family: `filter` (enumeration), `auto_filter` (boolean), `auto_filter_strategy` (enumeration), and two quality dictionaries (a base dictionary and an automatic-colour-space dictionary).

**[STU-IO-128] PDF enumerated value domains (normative, from the shipped preset spread).**

| Setting | Observed value set | Notes |
|---|---|---|
| `compatibility_level` | `1.3`, `1.4`, `1.5` | 1.3 on the strictest standards presets, 1.4 on general-purpose, 1.5 on the newest standard preset |
| `colour_conversion_strategy` | `LeaveColorUnchanged`, `UseDeviceIndependentColor`, `CMYK`, `sRGB` | |
| `downsample_type` | `Bicubic`, `None` | `None` appears only where downsampling is disabled |
| `image_filter` | `DCTEncode`, `FlateEncode` | lossy versus lossless |
| `auto_filter_strategy` | `JPEG`, `JPEG2000` | |
| `minimum_resolution_policy` | `OK`, `Warning` | |
| `transfer_function_info` | `Apply`, `Remove` | |
| `ucr_and_black_generation` | `Preserve`, `Remove` | |
| `auto_rotate_pages` | `All`, `None` | |
| `binding` | `Left` | only one value observed; Studio MUST also offer `Right` and MUST label it `studio_declared` |
| `compress_objects` | `Tags`, `Off`, `All` | |
| `overprint_mode` | `1` | integer; only one value observed |
| `cannot_embed_font_policy` | `Warning`, `Error`, `OK` | `Warning` observed; the other two are Studio-declared |
| `default_rendering_intent` | `Default` | only one value observed |

Every "only one value observed" row above MUST be carried in the machine contract with `domain_completeness: partial` so a later capture can extend it without ambiguity, and Studio MUST NOT present a single-value picker as if it were the complete domain.

**[STU-IO-129] PDF numeric setting contracts (from the shipped spread).**

| Setting | Observed values | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|---|
| `colour_image_resolution` | 100, 300, 350 | 1 | unknown | 72 | 600 | 300 | ppi | 0 |
| `greyscale_image_resolution` | 150, 300, 350 | 1 | unknown | 72 | 600 | 300 | ppi | 0 |
| `monochrome_image_resolution` | 300, 1200 | 1 | unknown | 300 | 2400 | 1200 | ppi | 0 |
| `downsample_threshold` | 1.50000 | 1.0 | unknown | 1.0 | 3.0 | 1.5 | ratio | 5 |
| `max_subset_percent` | 100 | 0 | 100 | 0 | 100 | 100 | percent | 0 |
| `image_memory` | 1048576 | unknown | unknown | absent | absent | 1048576 | bytes | 0 |
| `start_page` | 1 | 1 | unknown | 1 | unknown | 1 | page number | 0 |
| `end_page` | -1 | -1 | unknown | -1 | unknown | -1 | page number (-1 = last) | 0 |
| `dsc_reporting_level` | 0 | 0 | unknown | 0 | 3 | 0 | level | 0 |
| `bit_depth` (per image family) | -1, 1, 2 | -1 | unknown | -1 | 16 | -1 | bits (-1 = unchanged) | 0 |

Every `unknown` above is honest: the source presets declare a VALUE, not a bound. Studio MUST NOT convert an observed value into a bound. Clamping to an observed set would forbid legal values.

**[STU-IO-130] Standards-compliance settings.** A PDF preset declares `standard_check_1a` (boolean), `standard_check_3` (boolean), `standard_compliant_only` (boolean) and `output_intent_profile` (string, may be empty). In the shipped spread, 2 of 13 presets set the 1a check, 3 set the 3 check, and 7 set compliant-only; four distinct output-intent profiles appear plus the empty value. A preset asserting a standard MUST declare its output-intent profile; asserting compliance with no intent profile is a validation error.

**[STU-IO-131] Font-embedding settings.** `embed_all_fonts` (boolean), `subset_fonts` (boolean), `max_subset_percent` (integer percent), `embed_open_type` (boolean), `always_embed` (ordered list), `never_embed` (ordered list), `cannot_embed_font_policy` (enumeration). In the shipped spread 11 of 13 presets embed all fonts, all 13 subset, and all 13 use a 100 percent subset threshold. A font that cannot be embedded MUST produce a receipt naming the font and the policy applied ([STU-IO-012]).

**[STU-IO-132] Localised preset descriptions.** A preset carries a `description` map keyed by locale. The shipped baseline carries descriptions for 25 locales. Studio MUST store the map, MUST render the operator's locale with a declared fallback chain, and MUST NOT store a single flattened description string.

**[STU-IO-133] PDF preset provenance.** Every preset carries `preset_source` ∈ {`shipped`, `operator`, `imported`} and, for `imported`, the origin. A shipped preset is read-only; editing one produces an operator preset with a recorded parent.

---

### 4. Still-image export

**[STU-IO-134] Still-image export setting surface.** The still-image export surface is a named key set organised into groups. The reference baseline declares 152 keys of which 117 are confirmed export settings and 35 are surface-only fields; 64 carry observed values across the shipped and user presets and 88 are recoverable only as module constants. Studio MUST carry, per key, whether it is a stored export setting or a surface-only field, and MUST NOT silently promote a surface field into a stored setting.

**[STU-IO-135] Still-image export group set (normative, thirteen groups with reference key counts).** `file_settings` (36), `file_naming` (18), `image_sizing` (13), `export_location` (12), `service` (12), `watermarking` (12), `metadata` (9), `content_credentials` (7), `video` (5), `output_sharpening` (4), `hdr_output` (2), `post_processing` (2), `unclassified` (20). Group membership drives surface organisation and the model-facing grouping.

**[STU-IO-136] Still-image enumerated value domains (normative).**

| Domain | Members |
|---|---|
| Output format | `JPEG`, `PNG`, `TIFF`, `PSD`, `PSB`, `DNG`, `AVIF`, `JPEG_XL`, `ORIGINAL` |
| Colour space | `sRGB`, `AdobeRGB`, `ProPhotoRGB`, `DisplayP3`, `Other` — the shipped presets observe only `sRGB`; the remaining members are Studio-declared and MUST be labelled `domain_completeness: partial` |
| Bit depth | `8`, `10`, `16`, `32` bits per component |
| TIFF compression | `None`, `LZW`, `ZIP` |
| Resize type | `WidthHeight`, `Dimensions`, `LongEdge`, `ShortEdge`, `Megapixels`, `Percentage` |
| Resolution unit | `pixels`, `in`, `cm`, `ppinch`, `ppcm` |
| Filename collision handling | `Ask`, `Overwrite`, `Rename`, `Skip` |
| Destination folder kind | `SpecificFolder`, `OriginalFolder`, `ChooseFolder`, `ChooseLater`, `TempFolder`, `DesignatedFolderPlusChild` |
| Metadata inclusion | `All`, `Copyright`, `CopyrightAndContact`, `ExcludeCameraInfo`, `ExcludeCameraRawInfo` |
| Keyword export | `flat`, `hierarchy` |
| Output sharpening media | `Screen`, `Glossy`, `Matte` |
| Output sharpening amount | `Low`, `Standard`, `High` |
| Post-processing action | `None`, `ShowInFileManager`, `OpenIn`, `OpenInSpecificApplication`, `ShowExportActionsFolder` |
| Extension case | `lowercase`, `uppercase` |
| Video handling | `Include`, `Exclude`, `PluginPresets` |
| Provenance-credential storage | `DontInclude`, `Embed`, `Cloud`, `EmbedAndPublish` |

**[STU-IO-137] Still-image numeric contracts (from the shipped spread).**

| Setting | Observed values | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|---|
| JPEG quality | 0.50427353382111, 0.6, 1 | 0 | 1 | 0 | 1 | 0.8 | ratio | 6 — the observed value carries eleven decimal places, so the stored precision MUST NOT be truncated to 2 or the value changes on round-trip |
| JPEG file-size limit | 100 | 1 | unknown | 50 | 10000 | 100 | kilobytes | 0 |
| Max width / max height | 500, 640, 1000, 2048 | 1 | unknown | 100 | 10000 | 1000 | selected resolution unit | 0 |
| Resize percentage | 100 | 1 | unknown | 1 | 400 | 100 | percent | 2 |
| Resolution | 72, 240 | 1 | unknown | 72 | 600 | 240 | selected resolution unit | 2 |
| Output sharpening level | 2 | 1 | 3 | 1 | 3 | 2 | level (1=Low, 2=Standard, 3=High) | 0 |
| Initial sequence number | 1 | 0 | unknown | 1 | 100000 | 1 | count | 0 |

**[STU-IO-138] Still-image boolean settings with shipped defaults.** `constrain_size` (observed true in 5 presets, false in 4), `do_not_enlarge` (false in 4, true in 1), `use_file_size_limit` (false in all 9 observed), `use_watermark` (false in all 9), `include_video_files` (true in 5, false in 4), `minimize_embedded_metadata` (false in 4, true in 4), `reimport_exported_photo` (false in 7), `stack_with_original` (false in 7), `use_subfolder` (false in 5), `renaming_tokens_on` (false in 9). Where a preset spread splits, Studio MUST pick a default and MUST label it `studio_declared`, because the source does not declare one.

**[STU-IO-139] Filename token contract.** The still-image filename template is a token string over four token groups: image name, sequence and date, metadata, and custom. Studio MUST support at minimum the tokens named in [STU-AUT-130], and the shipped default template is the bare image-name token.

---

### 5. Professional video export

**[STU-IO-140] Video export scale contract.** The reference video export surface is 1,541 shipped presets over 704 distinct preset identities, 40 distinct container formats, 453 distinct parameter identifiers and 62,670 parameter rows. Studio's own preset library is its own; the CONTRACT is that the preset store, the parameter surface, the format matrix and the preset picker MUST be specified and tested at that scale, and that a preset is a COMPLETE parameter set per [STU-IO-125].

**[STU-IO-141] Container format record.** A container is identified by a `(exporter_class_id, exporter_file_type)` pair. Each container declares its preset count, whether it carries video, whether it carries audio, and its codec and multiplexing parameter set. A recipe naming a container whose exporter is unavailable fails closed per [STU-IO-108].

**[STU-IO-142] Frame-rate encoding (normative).** Frame rate is stored as TICKS PER FRAME against a fixed tick base of **254,016,000,000 ticks per second**. It is NOT stored as a floating-point rate, because the common broadcast rates are exact rationals that float cannot represent. The decoded table for the shipped values:

| Ticks per frame | Frame rate |
|---|---|
| 4,233,600,000 | 60 |
| 4,237,833,600 | 59.94006 |
| 5,080,320,000 | 50 |
| 5,292,000,000 | 48 |
| 8,467,200,000 | 30 |
| 8,475,667,200 | 29.97003 |
| 8,475,675,675 | 29.97 |
| 10,160,640,000 | 25 |
| 10,584,000,000 | 24 |
| 10,594,584,000 | 23.976024 |
| 10,594,594,594 | 23.976 |
| 16,934,400,000 | 15 |
| 20,321,280,000 | 12.5 |

Two distinct tick values decode to 29.97 and two to 23.976; they are DIFFERENT rates and MUST NOT be normalised to one. Studio MUST store the tick value, MUST derive the display rate, and MUST NOT round-trip through a float.

**[STU-IO-143] Bitrate parameter family.** The rate-control facet carries at minimum `minimum_bitrate`, `target_bitrate`, `maximum_bitrate`, `bitrate_encoding_mode` (constant or variable, with a pass count), `bitrate_level` (a named tier plus a custom option) and `quality` parameters. All are `floating_point` type code. Bounds: hard bounds are declared per exporter and MUST be read from the exporter, not assumed; where the exporter declares none, both are `unknown` per [STU-IO-103]. Unit is megabits per second unless the exporter declares otherwise; the unit MUST be read, not assumed.

**[STU-IO-144] Declared bounds exist and MUST be honoured where present.** In the reference surface a parameter may carry `declared_min`, `declared_max` and `decimal_places` alongside its observed values — for example an audio mode parameter declaring min 0, max 3 and 2 decimal places while every shipped preset uses the single value 2. Where a declared bound exists, Studio MUST use it as the hard bound and MUST NOT substitute the observed value range. Where only observed values exist, Studio MUST record them as `observed_values` and leave the bounds `unknown`; clamping to an observed range would forbid legal values.

**[STU-IO-145] Hidden and slider flags are per parameter and per preset.** A parameter may be hidden in some presets and visible in others, and may present as a slider in some and a field in others. Studio MUST carry `is_hidden_in_any_preset` and `is_slider_in_any_preset` as separate facts from the per-recipe presentation, because a parameter hidden by default is still settable by a model and MUST remain in the corpus.

**[STU-IO-146] Colour and HDR parameters.** The colour facet carries at minimum: export colour space, colour primaries, mastering-display luminance minimum and maximum in candelas per square metre, content light level maximum and average in the same unit, and the black point, white point and gamma triple. Units MUST be carried literally; `cd/m^2` is the declared unit and MUST NOT be normalised away.

**[STU-IO-147] Publishing-destination parameters are adapter-scoped.** 86 of the 453 reference identifiers are publishing-destination parameters — credentials, tokens, descriptions, tags, server addresses and login actions for hosted upload services. Under [STU-IO-013] every one of these belongs to an OPTIONAL adapter, never to a core recipe. Studio MUST NOT store a credential or token in a recipe record; a publishing adapter references the kernel credential store, exactly as a web site record does ([STU-WEB-114]).

**[STU-IO-148] Image-sequence export.** A video recipe MAY target an image sequence rather than a container: `{export_as_sequence: true, still_format, frame_number_padding, frame_range}`. `export_as_sequence` is a declared parameter with a shipped description ("if checked, a sequence of still files is written, one for each video frame"), so it is a first-class mode, not an emergent behaviour.

---

### 6. Preset and library import

**[STU-IO-150] Preset container import (normative, twenty-four format families).** Studio MUST be able to READ the shipped preset container families so an operator's existing libraries transfer. The recovered family map:

| Extension | Family |
|---|---|
| `.atn` | recorded action sets |
| `.asl` | layer styles |
| `.abr` | brushes |
| `.grd` | gradients |
| `.pat` | patterns |
| `.csh` | custom shapes |
| `.shc` | contours |
| `.aco` | swatches |
| `.ase` | swatch exchange |
| `.acb` | colour books |
| `.act` | colour tables |
| `.acv` | curves |
| `.alv` | levels |
| `.ahu` | hue and saturation |
| `.blw` | black and white |
| `.cha` | channel mixer |
| `.hdt` | HDR toning |
| `.ado` | duotones |
| `.3dl` | 3D lookup tables |
| `.cube` | 3D lookup tables |
| `.look` | look lookup tables |
| `.tpl` | tool presets |
| `.irs` | web-optimiser settings |
| `.mnu` | menu customisation |

**[STU-IO-151] Preset import parse-status contract (normative, three members).** Every preset container import reports a parse status: `parsed` (every declared entry decoded and the reader landed exactly at the container's declared end with no residual bytes), `partial` (entry names and counts recovered but at least one field's meaning is inferred rather than read), or `failed` (the container could not be decoded, with the reason). The status MUST be carried per container and MUST be surfaced to the operator and the model; a `partial` import MUST NOT be presented as complete. In the reference sweep of 286 containers, 121 parsed fully, 165 parsed partially and none failed — a partial rate above half, which is why the status is mandatory rather than optional.

**[STU-IO-152] Preset import scale contract.** The reference sweep recovered 16,994 preset entries across 286 container files in 22 families, the largest single families being 8,901 swatches, 5,243 colour-book entries, 725 custom shapes, 643 brushes, 370 layer styles, 329 gradients and 312 patterns. Studio's preset importer, its picker and its search MUST be specified and tested at that scale.

**[STU-IO-153] Library file import.** Studio MUST read vector library containers carrying graphic styles, symbols, brushes and swatch families. The reference sweep across 183 library files recovered 15,987 published entries: 10,011 named colours, 3,155 swatches, 884 symbols, 659 gradients, 561 brushes, 382 patterns and 314 graphic styles. A library file contains both PUBLISHED entries (what the library offers) and INCIDENTAL definitions (gradients and patterns its own artwork happens to reference); the reference sweep found 18,424 definitions against 15,987 published entries. Studio MUST distinguish the two and MUST NOT report the definition count as the library size.

**[STU-IO-154] Format-count honesty.** Every count in this sub-section is an ENTRY count unless the row says "files". Reporting a file count as an entry count, or the reverse, is a documentation defect and MUST be caught by review.

---

### 7. Obligations

**[STU-IO-160] Colour and unit explicitness at every boundary.** [STU-IO-003] stands and is extended: every numeric parameter in this sub-section carries its unit as a SEPARATE field, and every colour parameter carries an explicit `StudioColorProfile` reference. A parameter whose unit is genuinely dimensionless declares `unit: dimensionless`; the field is never absent.

**[STU-IO-161] Asset library binding.** Every input Studio imports and every deliverable it exports is a CKC asset. Import registers the source; export registers the output and records the derivation edge, per [STU-ASSET-012] through [STU-ASSET-014]. Export MUST resolve every placed-asset link before writing and MUST fail closed on a `missing` or `unauthorized` link ([STU-ASSET-009]).

**[STU-IO-162] Parallel safety.** Two model lanes exporting two different documents MUST both succeed. Two lanes exporting the same document with different recipes MUST both succeed, because export is a read of document authority plus a write of new artifacts. Two lanes writing the same output path MUST resolve under the collision policy of [STU-AUT-131], never by racing.

**[STU-IO-163] Determinism.** Export MUST be a pure function of (document revision, recipe, resolved asset manifest ids). Re-running the same export MUST produce byte-identical output except for fields the format defines as time-varying, which MUST be enumerated per format in its writer's contract. A writer that embeds an undeclared timestamp, a random id or a locale-dependent string breaks promotion equivalence and is a defect.

**[STU-IO-164] Headless and quiet.** Every import, export and batch conversion runs headless under 14.20: no foreground window, no focus steal, no modal dialog, progress and per-file outcome readable through structured job state. An export configuration that would block on a dialog MUST refuse to start naming the offending parameter ([STU-AUT-136]).

**[STU-IO-165] Validation descriptor set.** This sub-section contributes at minimum: `export_parameter_bounds_collapsed` (hard and soft written as one range), `export_parameter_bound_invented` (a bound present where the source declared none and no `studio_declared` label), `export_parameter_missing_unit`, `export_enumerated_domain_presented_as_complete` (a partial domain shown as closed), `export_codec_integer_without_exporter`, `export_frame_rate_stored_as_float`, `export_preset_incomplete` (a preset declaring a subset of its format's settings), `export_output_filename_collision`, `export_placed_asset_unresolved`, `export_writer_nondeterministic_field_undeclared`, `preset_import_partial_reported_as_complete`, `library_definition_count_reported_as_entry_count`, `export_credential_in_recipe_record`.

**[STU-IO-166] Storage constraint.** Recipes, presets, import profiles, receipts and the parameter registry are SurrealDB `SCHEMAFULL` tables. Exported bytes, imported preset containers and preservation blobs are content-addressed artifacts ([STU-IO-005], [STU-ASSET-008]). No SQLite, libSQL, Turso or PostgreSQL anywhere, including fixtures and caches ([STU-OVR-003]).

**[STU-IO-167] GUI / Argus / UserManual obligation.** [STU-IO-014] remains in force unchanged and additionally covers every record shape, enumeration, bound and parameter introduced by [STU-IO-100] through [STU-IO-166]. Every enumeration here MUST appear in the model-facing UserManual as its literal token list, every parameter MUST document its seven contract fields separately, and every `unknown` bound MUST be documented as unknown rather than omitted.

---

### 8. Microtask Derivation

**[STU-IO-168] Derivation rule (NORMATIVE).** The interoperability microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states a **parameter-model rule** ([STU-IO-101], [STU-IO-102], [STU-IO-103], [STU-IO-106], [STU-IO-107], [STU-IO-108], [STU-IO-109], [STU-IO-110], [STU-IO-144], [STU-IO-145]), a **closed enumeration or value domain** ([STU-IO-104], [STU-IO-105], [STU-IO-128], [STU-IO-136], [STU-IO-150], [STU-IO-151]), a **settings record shape** ([STU-IO-111], [STU-IO-112], [STU-IO-113], [STU-IO-114], [STU-IO-115], [STU-IO-116], [STU-IO-117], [STU-IO-127], [STU-IO-130], [STU-IO-131], [STU-IO-132], [STU-IO-133], [STU-IO-139], [STU-IO-141], [STU-IO-143], [STU-IO-146], [STU-IO-147], [STU-IO-148]), a **numeric contract table** ([STU-IO-129], [STU-IO-137], [STU-IO-138]), a **preset or group contract** ([STU-IO-125], [STU-IO-126], [STU-IO-134], [STU-IO-135]), an **encoding contract** ([STU-IO-142]), a **scale contract** ([STU-IO-140], [STU-IO-152], [STU-IO-153], [STU-IO-154]), or an **execution guarantee** ([STU-IO-162], [STU-IO-163]), where that clause can be implemented and proven independently of its siblings.
2. Each **validation-descriptor clause** in sub-section 9, [STU-IO-173] through [STU-IO-185]. Each of the 13 descriptors named in [STU-IO-165] is stated as its own clause precisely so it yields its own microtask: a check is a unit of implementable, independently provable work, and one microtask reading "implement 13 checks" is not implementable by the small models these contracts are sized for. A descriptor list inside a single clause, whether as prose or as a table, is one unit to any derivation tool and therefore loses 12 units of real work.

No other unit yields a microtask. Exactly 6 clauses in this module yield nothing, and they are:

- **Baseline, scope-fence and supersession clauses** — [STU-IO-100], which sits under the bookkeeping heading `0. Baseline, supersession and disposition`. These are discharged when the v02.206 bundle lands, not by a work packet.
- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-IO-169] Open items and blocked dependencies.** This module declares no BLOCKED dependency. It does declare a large number of UNKNOWN bounds — every `hard_min` or `hard_max` marked `unknown` in [STU-IO-129], [STU-IO-137] and [STU-IO-143]. An unknown bound is NOT an open item: [STU-IO-103] settles it normatively by requiring the parameter be passed unclamped and the encoder's own error surfaced. Those clauses therefore yield ordinary microtasks. Should a later amendment introduce a genuine open item or a BLOCKED dependency, that clause STILL yields a microtask, and that microtask's FIRST acceptance criterion MUST be resolving the named dependency — reading the named surface, obtaining the named decision, or raising a BLOCKED record with the exact blocker. A declared gap MUST NOT be dropped from the yields index.

**[STU-IO-170] Microtask content obligation.** A microtask derived under [STU-IO-168] MUST carry into its own body: the clause anchor; the SEVEN parameter-contract fields of every parameter it touches, as SEPARATE fields, with `unknown` and `absent` preserved and never collapsed per [STU-IO-103]; the complete member list of every enumerated value domain it touches, with its `domain_completeness` marking where the domain is partial; the real unit token, never a guess, per [STU-IO-160]; and the determinism obligation of [STU-IO-163] where it touches a writer. A microtask that says "implement the PDF image settings" without the eleven-setting family of [STU-IO-127] and the value domains of [STU-IO-128] does not satisfy this clause.

**[STU-IO-171] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-IO-168]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| The export parameter model | [STU-IO-101]-[STU-IO-110] | 10 clause | 10 |
| Node-render settings (the per-node export surface) | [STU-IO-111]-[STU-IO-117] | 7 clause | 7 |
| Document PDF export | [STU-IO-125]-[STU-IO-133] | 9 clause, 1 enumeration, 1 parameter table | 11 |
| Still-image export | [STU-IO-134]-[STU-IO-139] | 6 clause, 1 enumeration, 1 parameter table | 8 |
| Professional video export | [STU-IO-140]-[STU-IO-148] | 9 clause | 9 |
| Preset and library import | [STU-IO-150]-[STU-IO-154] | 5 clause | 5 |
| Obligations | [STU-IO-160]-[STU-IO-167] | 8 clause | 8 |
| Validation Descriptor Catalogue | [STU-IO-173]-[STU-IO-185] | 13 validator | 13 |
| Clauses yielding nothing | 6 clauses, listed in [STU-IO-168] | — | 0 |
| **Module total** | | **73 clauses** | **71** |

Of this module's 73 clauses, 6 yield nothing and 67 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **71**. The last numeric column is the yields count.

**[STU-IO-172] Anchor binding.** A microtask derived from this module cites its clause anchor directly. A microtask staged before this module landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor in [STU-IO-100]–[STU-IO-185], or to a preserved v02.205 anchor in [STU-IO-001]–[STU-IO-014], clears that status. A microtask that cannot cite either is out of scope for the interoperability domain and MUST be re-derived or retired, not activated.

---

### 9. Validation Descriptor Catalogue

Each descriptor below is its own clause because each is its own unit of implementable, independently provable work: feed the runtime a document that violates the rule and assert the check fires with the stated diagnostic. [STU-IO-165] names the set; the clauses in this sub-section state what each member catches, which clause it enforces, its severity, and what its diagnostic MUST name. Every one is a `StudioValidationDescriptor` in the catalogue of 14.24.

**[STU-IO-173] `export_parameter_bounds_collapsed`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a parameter stores its hard and soft bounds as one range, or mirrors an undeclared bound from its opposite, enforcing [STU-IO-103]. The diagnostic MUST name the parameter and both bound pairs; this is unrecoverable once written, so the check runs at write time, not only at review.

**[STU-IO-174] `export_parameter_bound_invented`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a bound is present where the source declared none and the value carries no `studio_declared` label, enforcing [STU-IO-103]. The diagnostic MUST name the parameter and the unlabelled bound.

**[STU-IO-175] `export_parameter_missing_unit`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a numeric parameter carries no `unit` field, rather than carrying `dimensionless`, enforcing [STU-IO-160]. The diagnostic MUST name the parameter.

**[STU-IO-176] `export_enumerated_domain_presented_as_complete`.** The interoperability validator MUST reject, with severity `warning`, a document or command in which a value domain marked `domain_completeness: partial` is offered as a closed picker, enforcing [STU-IO-128]. The diagnostic MUST name the domain and the setting it belongs to.

**[STU-IO-177] `export_codec_integer_without_exporter`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a recipe stores a codec-selection integer without the exporter identity that gives it meaning, enforcing [STU-IO-108]. The diagnostic MUST name the recipe and the codec parameter.

**[STU-IO-178] `export_frame_rate_stored_as_float`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a frame rate is stored as a floating-point rate rather than as ticks per frame against the declared tick base, enforcing [STU-IO-142]. The diagnostic MUST name the recipe and the rate; the two tick values that both decode to 29.97, and the two that decode to 23.976, MUST remain distinct.

**[STU-IO-179] `export_preset_incomplete`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a preset declares a subset of its format's settings and relies on an ambient default for the rest, enforcing [STU-IO-125]. The diagnostic MUST name the preset and every missing setting.

**[STU-IO-180] `export_output_filename_collision`.** The interoperability validator MUST reject, with severity `error`, a document or command in which two render settings on one node resolve to the same output filename, enforcing [STU-IO-117]. The diagnostic MUST name both settings and the colliding name; the check runs at authoring time, not at export time.

**[STU-IO-181] `export_placed_asset_unresolved`.** The interoperability validator MUST reject, with severity `error`, a document or command in which an export would write a deliverable containing a placed-asset link in the `missing` or `unauthorized` state, enforcing [STU-IO-161]. The diagnostic MUST name the placement, the asset id and the link state; the export fails closed and writes nothing.

**[STU-IO-182] `export_writer_nondeterministic_field_undeclared`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a format writer emits a timestamp, random identifier or locale-dependent string that its contract does not declare as time-varying, enforcing [STU-IO-163]. The diagnostic MUST name the writer, the field, and the two differing byte ranges from a double run.

**[STU-IO-183] `preset_import_partial_reported_as_complete`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a preset container that parsed with status `partial` is presented to the operator or the model as fully parsed, enforcing [STU-IO-151]. The diagnostic MUST name the container, its parse status, and the fields whose meaning was inferred.

**[STU-IO-184] `library_definition_count_reported_as_entry_count`.** The interoperability validator MUST reject, with severity `warning`, a document or command in which a library's total definition count is reported as its published entry count, enforcing [STU-IO-153]. The diagnostic MUST name both counts and the library.

**[STU-IO-185] `export_credential_in_recipe_record`.** The interoperability validator MUST reject, with severity `error`, a document or command in which a recipe record contains a credential, token, password or private key rather than a reference into the kernel credential store, enforcing [STU-IO-147]. The diagnostic MUST name the recipe and the offending field name only, never the secret value.
