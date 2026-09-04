---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-08"
section_id: "14.8"
title: "14.8 Color Management and Pipeline"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
supersedes_body_range: "14-studio-creative-suite.md lines 1483-1643 (14.8 Color Management & Pipeline)"
declared_yields_total: 320
yields_ledger_clause: "STU-COL-272"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
anchor_prefix: "STU-COL"
anchor_range_new: "STU-COL-100 .. STU-COL-275, STU-COL-300 .. STU-COL-355 (plus STU-COL-104A)"
anchor_range_preserved: "STU-COL-001 .. STU-COL-036 (see 14.8.0 for the disposition of each)"
---
<a id="148-color-management-and-pipeline"></a>
## 14.8 Color Management and Pipeline

Colour is a shared Studio PIPELINE, not a per-domain feature. Every fill, stroke, swatch, gradient,
pattern, text colour, adjustment, grade, LUT, proof and separation across raster, vector, layout,
video, motion and compositing resolves through ONE colour value model, ONE profile primitive
(`StudioColorProfile`), ONE swatch primitive (`StudioSwatch`), ONE gradient primitive
(`StudioGradient`), ONE pattern primitive (`StudioPattern`), and ONE engine
(`ColorEngine: Send + Sync`). A source suite's colour panel or command name is never a Studio name.

This module is SELF-CONTAINED. A capable implementer with no chat context and no access to the
Studio research corpus MUST be able to implement the colour pipeline, and to derive the colour
microtask set (14.8.19), from this module plus the shared contracts it names (14.0 storage,
14.2 architecture, 14.3 primitives, 14.7 typography, 14.23 canonical field contracts,
14.24 validation). Where this module and 14.23 disagree on a FIELD NAME, TYPE or SCHEMA ID, 14.23
wins; where they disagree on BEHAVIOUR, RANGE, DEFAULT, UNIT or ENUMERATED VALUE, this module wins
and the divergence is a defect in 14.23 to be repaired.

---

### 14.8.0 Authority, Derivation and Supersession

**[STU-COL-100] Derivation basis and the missing-menu problem.** Colour is the largest single domain
in the captured surface (7,478 registry rows against 3,040 for typography and 4,609 for layout) and
simultaneously the domain with almost NO menu surface: only 5 of those 7,478 rows are menu entries
(0.07 percent), against 63 for typography. Colour is not exposed as a command tree in any source
application; it is exposed as properties on objects, options inside dialogs, preset payloads and
engine behaviour. There is therefore no menu to transcribe and no command list to dedupe.
**This module defines the colour model from PRIMITIVES, deriving each clause from the parsed value
types, enumerations, preset payloads and shader source of the installed applications.** The named
capture files are recorded per clause in `14-08-color.provenance.json`. The captures are EVIDENCE
and are never authority.

**[STU-COL-101] Anchor continuity.** Anchors STU-COL-001 through STU-COL-036 were assigned in
Master Spec v02.199-v02.205. Anchors added by this module begin at `STU-COL-100`. No existing anchor
is renumbered or reused. Every existing anchor is RETAINED, REFINED or SUPERSEDED, stated below.

**[STU-COL-102] Disposition of the pre-existing 14.8 anchors.**

*Derivation: reading aid inside a non-yielding clause; yields no microtask. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Anchor | State | Disposition |
|---|---|---|
| STU-COL-001 | REFINED | `StudioColorProfile` remains the profile primitive; field contract bound by [STU-COL-125]-[STU-COL-129]. |
| STU-COL-002 | REFINED | Colour models; the per-model value contracts and bounds are now in [STU-COL-110]-[STU-COL-117] and the mode enumeration in [STU-COL-118]. |
| STU-COL-003 | REFINED | Bit depth; bound by [STU-COL-120]-[STU-COL-124]. |
| STU-COL-004 | REFINED | ICC handling; bound by [STU-COL-125]-[STU-COL-139]. |
| STU-COL-005 | REFINED | OCIO path; bound by [STU-COL-150]-[STU-COL-157]. |
| STU-COL-006 | RETAINED, REINFORCED | Native colour engine; restated and hardened by [STU-COL-140]-[STU-COL-144]. |
| STU-COL-007 | REFINED | `StudioSwatch` kinds; bound by [STU-COL-170]-[STU-COL-176]. |
| STU-COL-008 | REFINED | Swatch groups and palette scope; bound by [STU-COL-177]-[STU-COL-179]. |
| STU-COL-009 | REFINED | Swatch interchange; bound by [STU-COL-180]-[STU-COL-182]. |
| STU-COL-010 | REFINED | Gradient geometries; the missing `diamond` geometry is added by [STU-COL-190]. |
| STU-COL-011 | REFINED | Gradient controls; bound by [STU-COL-191]-[STU-COL-193]. |
| STU-COL-012 | REFINED | `StudioPattern`; bound by [STU-COL-195]-[STU-COL-196]. |
| STU-COL-013 | REFINED | The prepress operation table; each row now has a bound clause in 14.8.12-14.8.14. |
| STU-COL-014 | RETAINED | Channel-operation boundary against 14.4. |
| STU-COL-015 | REFINED | Find/replace colour; bound by [STU-COL-183]. |
| STU-COL-016 | REFINED | Colour picker; bound by [STU-COL-230]-[STU-COL-233]. |
| STU-COL-017 | REFINED | Harmony; bound by [STU-COL-235]. |
| STU-COL-018 | REFINED | Recolour; bound by [STU-COL-236]. |
| STU-COL-019 | RETAINED | Generative recolour is an optional adapter only. |
| STU-COL-020 | RETAINED | Native spot colour is first class and licence-free. |
| STU-COL-021 | **SUPERSEDED** | The branded-library posture is factually wrong about what the source applications ship. Replaced by [STU-COL-184]-[STU-COL-187]. |
| STU-COL-022 | REFINED | Soft proof; bound by [STU-COL-240]-[STU-COL-243]. |
| STU-COL-023 | REFINED | Rendering intents; Studio's own encoding is fixed by [STU-COL-135]. |
| STU-COL-024 | REFINED | Gamut warning; bound by [STU-COL-244]-[STU-COL-245]. |
| STU-COL-025 | REFINED | Separations preview; bound by [STU-COL-212]. |
| STU-COL-026 | REFINED | Overprint; bound by [STU-COL-210]-[STU-COL-211]. |
| STU-COL-027 | REFINED | Ink manager; bound by [STU-COL-205]-[STU-COL-209]. |
| STU-COL-028 | REFINED | Trapping; the fifteen-field trap preset is bound by [STU-COL-213]. |
| STU-COL-029 | REFINED | Flattening; the seven-field flattener preset is bound by [STU-COL-214]. |
| STU-COL-030 | REFINED | Appearance of black; bound by [STU-COL-215]. |
| STU-COL-031 | REFINED | Mode conversion; the full option sets are bound by [STU-COL-118]-[STU-COL-119]. |
| STU-COL-032 | RETAINED | No untagged device triples anywhere. |
| STU-COL-033 - STU-COL-035 | REFINED | Global, spot, tint and mixed-ink propagation; bound by [STU-COL-172]-[STU-COL-176]. |
| STU-COL-036 | REFINED | Mesh and freeform gradients; bound by [STU-COL-194]. |

**[STU-COL-103] Capture-versus-spec contradiction of record: branded colour libraries.** The
superseded [STU-COL-021] asserted that "current Photoshop no longer bundles Pantone books and routes
them through a licensed plug-in", and generalised from that to a blanket licence-gated-adapter
posture for branded libraries. The installed applications contradict the generalisation. The parsed
install ships:

- Twelve colour-book containers in the raster application, decoding to **5,243 colours**:
  ANPA Color (300, Lab), DIC Color Guide (1,280, Lab), FOCOLTONE (860, CMYK),
  HKS E (98, Lab), HKS E Process (86, CMYK), HKS K (100, Lab), HKS K Process (86, CMYK),
  HKS N (98, Lab), HKS N Process (86, CMYK), HKS Z (59, Lab), HKS Z Process (86, CMYK),
  TRUMATCH (2,104, CMYK).
- Twenty colour-book containers in the vector application (14 plus 6 of a second book format), and
  **10,011 colours** in the `colors` entry kind across its 118 swatch libraries.

Only PANTONE is absent. Every other branded book family is still bundled, and one of them
(TRUMATCH, 2,104 colours) is larger than the absent one. The correct posture is therefore
PER-BOOK, not blanket: Studio's native spot primitive is always present and licence-free
([STU-COL-020], retained), and each branded book is a separately licensed, separately loadable data
adapter whose presence or absence is recorded per book. Superseded by [STU-COL-184]-[STU-COL-187].

**[STU-COL-104] Two further contradictions of record.**

1. **Colour-component defaults diverge between source applications.** `_RGBColor.Red` is declared
   with `default: 255.0` in the raster application's type library and `default: 0.0` in the vector
   application's. Both are 0.0-255.0 in range. A specification that inherits "the vendor default"
   is therefore under-determined. Studio fixes ONE default and states it; see [STU-COL-112].
2. **Gradient midpoint is bounded, and not at 0-100.** The superseded [STU-COL-010] described an
   "editable midpoint" with no bound. The vector application declares `GradientStop.MidPoint` as
   **13.0 to 87.0 percent**, while `RampPoint` and `Opacity` on the same object are 0.0-100.0. A
   midpoint slider built on a 0-100 assumption produces values the engine rejects. Bound by
   [STU-COL-192].

**[STU-COL-104A] Naming discipline and the one permitted exception.** Per [STU-SECTION-003] a source
suite's product, panel, tool or command name is never a Studio name and does not appear in
this module's normative text. The SOLE exception is the contradiction and disposition record
of [STU-COL-102] through [STU-COL-104], where a vendor class, property, enumeration or
branded-library name is cited AS EVIDENCE so a reviewer can verify the disagreement against
the named capture, and [STU-COL-185]-[STU-COL-187], where a branded colour-book family must be
named because its presence or absence IS the fact being specified. Those citations are
provenance, not Studio vocabulary, and no Studio type, field, command, panel or manual entry
may take its name from them. Elsewhere this module refers to source applications by role -
"the raster application", "the vector application", "the captured grading surface", "the
captured develop model" - which is also how the companion `14-08-color.provenance.json`
addresses them. A letter-suffixed anchor (this clause) is a legal form, following the letter-suffixed
anchors this section already carries, such as `STU-RAW-008a`, `STU-FX-133a` and
`STU-VID-001a`.

NEVER ASSIGNED. `STU-COL-OBLIG-001` was never assigned and MUST NOT be assigned later. It is
cited in two places and neither citation survives as written. Earlier text named it as the
precedent for letter-suffixed anchors, which was circular because the anchor has no clause.
`[STU-COL-022]` cites it substantively for Argus inspectability of the soft-proof state; that
obligation is real and is carried by `[STU-COL-250]`, the GUI / Argus / UserManual obligation
stated once for 14.8, so a reader following the dead id MUST read `[STU-COL-250]` instead. No
clause is to be written to fill the dead anchor.

**[STU-COL-105] Why hard and soft bounds are separate fields.** The captured grading
surface is the clearest evidence in the corpus that a single "range" is lossy. Of its declared
parameters, `Temperature` accepts **-150 to 150** but its control presents **-100 to 100**;
`Exposure` accepts **-7 to 7** but presents **-5 to 5**; `Saturation` accepts **0 to 300** but
presents up to **200**; `Blur` accepts **0 to 1000** but presents up to **30**; `Faded Film`
accepts **0 to 150** but presents up to **100**. Worse, the SAME control name carries a DIFFERENT
hard bound in a different section of the same effect: `Temperature` in the primary correction is
hard -150/150, while `Temperature` in the secondary correction is hard **-300/300** - both with a
-100/100 control. Collapsing hard into soft is irreversible and would silently reject legal values
in one section while accepting them in another. [STU-COL-106] is therefore non-negotiable.

---

### 14.8.1 The Colour Parameter Contract

**[STU-COL-106] Seven-field parameter record (NORMATIVE, applies to every numeric colour parameter
in this module).** Identical in shape to [STU-TYP-105] and stated again here so this module stands
alone. Every numeric parameter MUST be declared with SEVEN SEPARATE fields:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Meaning |
|---|---|
| `hard_min` | Lowest value the engine accepts. Below it is an error, not a clamp. |
| `hard_max` | Highest value the engine accepts. Above it is an error, not a clamp. |
| `soft_min` | Lowest value the default control presents. A user or model MAY type past it. |
| `soft_max` | Highest value the default control presents. A user or model MAY type past it. |
| `default` | Factory value when the parameter is unset. |
| `unit` | The real unit token. |
| `precision` | Decimal places carried and round-tripped. |

`hard` and `soft` MUST be emitted as four distinct fields from the first schema version, even when a
source declares only one of them, with the other marked `UNKNOWN`. A single collapsed range is a
specification defect that cannot be repaired without re-deriving from the captures.

**[STU-COL-107] Unknown-bound rule.** Where a bound, default, unit or precision was not declared,
the field value is the literal token `UNKNOWN` and the parameter is NOT clamped on that side.
`UNKNOWN` MUST be preserved through the schema, the API, the UI and the model surface. Inventing a
number is a specification defect. Where an implementer must choose a soft bound to build a usable
control, the chosen value is recorded with `soft_bound_source = "implementation"`, never promoted to
`hard_*`, and never presented as vendor-derived. Where THIS MODULE fixes a bound that no capture
declared, the clause says so inline and the bound is a Studio normative choice, not an observation.

**SELF-AUDIT.** This module carries 144 numeric parameter rows across 36 parameter tables. **38**
carry a complete seven-field set with no `UNKNOWN`; **106** carry at least one stated `UNKNOWN`; 375
of the 1,008 individual fields are `UNKNOWN`. Colour has a materially higher completion rate than
typography (which has zero complete rows, see [STU-TYP-238]) for one reason: the captured grading
surface declares hard and soft bounds SEPARATELY on many of its parameters, and the colour-component
type libraries declare bounds and defaults. Those declarations are the reason the seven-field
contract exists at all ([STU-COL-105]).

Every one of those 144 rows carries `hard_min`, `hard_max`, `soft_min`, `soft_max`, `default`,
`unit` and `precision` as SEVEN SEPARATE COLUMNS, with the literal token `UNKNOWN` written into each
unknown cell. No table omits a column. This is a hard requirement and not a formatting preference:
an omitted column is indistinguishable from an unknown value once the table is read back, and a
parameter table carrying fewer than four of the seven named headers is not recognised as a parameter
table at all, so its parameters would silently vanish from the microtask set derived under the
rule of [STU-COL-270]. A table that drops a column is a defect even when every remaining value is
correct.

**[STU-COL-108] Observed-value rule.** A range recovered by surveying shipped presets is OBSERVED,
not declared. Observed ranges MUST be recorded as `observed_min` / `observed_max` metadata and MUST
NOT populate `hard_min` / `hard_max`. The captured adjustment-preset corpus is entirely observed:
its values were read out of 153 shipped presets across 25 adjustment types, so clamping to them
would reject legal values. Every observed range in this module is labelled OBSERVED inline.

**[STU-COL-109] Colour unit tokens.** `percent` (0-100 scale), `unit_interval` (0.0-1.0 scale),
`byte` (0-255 scale), `degrees`, `stops` (log2 exposure), `kelvin`, `nits` (cd/m2), `ppi`,
`dimensionless`, `count`, `index`. A component stored on a 0-1 scale and a component stored on a
0-100 scale are DIFFERENT parameters even when they describe the same quantity; the unit token
disambiguates them and MUST never be omitted.

---

### 14.8.2 Colour Value Model

**[STU-COL-110] Tagged-value law.** A colour value is a `(model, components, profile_ref)` triple.
There is no untagged device value anywhere in Studio: not in a swatch, not in a gradient stop, not
in a pattern, not in a text fill, not in an adjustment parameter, not in an export recipe, not on
the wire, not in a model command. Retained and restated from [STU-COL-032].

**[STU-COL-111] Colour model enumeration.** `color_model` is a seven-value enumeration:
`gray`, `rgb`, `cmyk`, `lab`, `hsb`, `mixed_ink`, `none`. `hsb` is an ENTRY and INTERCHANGE model,
not a storage model: a value entered as HSB is converted to the document's storage model at the
decode boundary and the HSB triple is not retained. `none` represents the absence of colour and
is distinct from a fully transparent colour.

**[STU-COL-112] Component bounds (NORMATIVE PARAMETER TABLE).** These are the declared component
bounds. Studio fixes one default per component; where the source applications disagree the
divergence is stated and Studio's choice is normative.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `rgb.r` | 0.0 | 255.0 | 0.0 | 255.0 | 0.0 | byte | 1 |
| `rgb.g` | 0.0 | 255.0 | 0.0 | 255.0 | 0.0 | byte | 1 |
| `rgb.b` | 0.0 | 255.0 | 0.0 | 255.0 | 0.0 | byte | 1 |
| `cmyk.c` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |
| `cmyk.m` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |
| `cmyk.y` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |
| `cmyk.k` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |
| `lab.l` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | dimensionless | 1 |
| `lab.a` | -128.0 | 127.0 | -128.0 | 127.0 | 0.0 | dimensionless | 1 |
| `lab.b` | -128.0 | 127.0 | -128.0 | 127.0 | 0.0 | dimensionless | 1 |
| `gray.k` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |
| `hsb.h` | 0.0 | 360.0 | 0.0 | 360.0 | 0.0 | degrees | 1 |
| `hsb.s` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |
| `hsb.b` | 0.0 | 100.0 | 0.0 | 100.0 | 0.0 | percent | 1 |

**Divergence of record:** the raster application declares `rgb.*` default 255.0 (white); the vector
application declares 0.0 (black) for the same components with the same range. Studio normatively
fixes **0.0**, so that an unset colour is deterministic and matches the CMYK, Lab and Gray defaults
(all 0.0). An import from a source declaring 255.0 MUST write the explicit value rather than relying
on the default.

**[STU-COL-113] Storage precision versus entry precision.** Components are STORED as f32 in the
document's declared precision ([STU-COL-120]) and are ENTERED at the scale of [STU-COL-112]. The
entry scale is a UI and API contract; the storage scale is engine-internal. A byte-scale entry of
`128` and a unit-interval entry of `0.50196` are the same colour and MUST round-trip.

**[STU-COL-114] Entry value modes.** The colour entry surface MUST accept a value in `8_bit`
(0-255), `16_bit` (0-65535), `unit_interval` (0.0-1.0) or `percent` (0-100) mode, per model, with
the mode a user-selectable preference that does not change storage. Hexadecimal RGB entry
(`#RRGGBB` and `#RRGGBBAA`) MUST be accepted and emitted.

**[STU-COL-115] Alpha is not a colour component.** Opacity and alpha are separate from the colour
value and are carried on the OBJECT, the PAINT or the CHANNEL, never inside `components`. A colour
value has no alpha slot.

**[STU-COL-116] Colour-space enumeration for values that are not process colours.** A stored value
may declare `color_space` = `rgb` | `cmyk` | `lab` | `mixed_ink` | `hsb` | `no_alternate`. A spot
swatch carries BOTH a `color_space` (its own definition) and an `alternate_space` with
`alternate_color_value` (how it renders when not separated). `no_alternate` means the ink has no
process approximation and MUST NOT be silently converted.

**[STU-COL-117] Image colour spaces.** A placed raster asset declares its own space from a
seven-value enumeration: `grayscale`, `rgb`, `cmyk`, `lab`, `separation`, `device_n`, `indexed`.
`separation` and `device_n` are ink-space images and MUST NOT be forced into a process model on
import; they carry their ink names and separate correctly.

---

### 14.8.3 Document Modes, Bit Depth and HDR

**[STU-COL-118] Document mode enumeration.** `document_mode` is an eight-value enumeration:
`grayscale`, `rgb`, `cmyk`, `lab`, `bitmap`, `indexed`, `multichannel`, `duotone`. A
`new_document_mode` subset of five (`grayscale`, `rgb`, `cmyk`, `lab`, `bitmap`) is available at
creation; `indexed`, `multichannel` and `duotone` are reachable only by conversion.
`change_mode` is a seven-value operation enumeration:
`to_grayscale`, `to_rgb`, `to_cmyk`, `to_lab`, `to_bitmap`, `to_indexed`, `to_multichannel`.
Note the asymmetry: there is no `to_duotone` conversion operation; duotone is entered from
grayscale through the duotone ink configuration of [STU-COL-119].

**[STU-COL-119] Mode-conversion option sets (NORMATIVE).** Each conversion carries its own option
record. Conversions that discard channels MUST warn before executing and MUST be one undoable
operation.

**Bitmap conversion** - six fields:
`method` (`half_threshold` | `pattern_dither` | `diffusion_dither` | `halftone_screen` |
`custom_pattern`, default `diffusion_dither`), `resolution`, `pattern_name` (custom pattern only),
`frequency`, `angle`, `shape` (halftone screen only, six values: `round`, `diamond`, `ellipse`,
`line`, `square`, `cross`).

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `bitmap.resolution` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 72.0 | ppi | UNKNOWN |
| `bitmap.frequency` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | lines_per_inch | UNKNOWN |
| `bitmap.angle` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | degrees | UNKNOWN |
| `bitmap.threshold` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |

**Indexed conversion** - eight fields:
`palette` (twelve values: `exact` (default), `mac_os`, `windows`, `web`, `uniform`,
`local_perceptual`, `local_selective`, `local_adaptive`, `master_perceptual`, `master_selective`,
`master_adaptive`, `previous`), `colors`, `forced`
(`none` | `black_white` | `primaries` | `web`), `transparency` (bool), `dither`
(`none` | `diffusion` | `pattern` | `noise`), `dither_amount` (diffusion only),
`preserve_exact_colors` (bool), `matte` (seven values: `none`, `foreground_color`,
`background_color`, `white`, `black`, `semi_gray`, `netscape_gray`).

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `indexed.colors` | UNKNOWN | 256 | UNKNOWN | UNKNOWN | UNKNOWN | count | 0 |
| `indexed.dither_amount` | 1 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | 0 |

`indexed.colors` is settable only for some palette types; for `exact`, `mac_os`, `windows`,
`web` and `previous` it is read-only and derived. `hard_max` 256 follows from the 8-bit palette
of [STU-COL-120]; `hard_min` is not declared.

**Indexed colour table** is a first-class editable resource: an ordered list of up to 256 RGB
entries with an optional `transparency_index`. It is importable and exportable as a colour-table
file. Four tables ship by default (`black_white` with 2 entries, `grayscale` with 256,
`mac_os` with 256, `windows` with 256).

**Duotone configuration** - an ordered list of one to four inks. Each ink carries an `ink_name`, an
`ink_color` (a `StudioSwatch` reference), and a transfer CURVE mapping input tone to ink density.
`ink_count` selects monotone (1), duotone (2), tritone (3) or quadtone (4). Overprint colours
between ink pairs are a separate list.

> **Recovery limit of record.** The captured duotone corpus contains 114 shipped presets. Their
> declared ink COUNT and ink NAMES were recovered from every one; the byte layout of the per-ink
> transfer-curve record was NOT recovered and did not reproduce consistently across the corpus.
> Studio therefore defines the duotone curve as a Studio-native curve primitive
> ([STU-COL-225]) and does NOT claim compatibility with the source binary layout. Importing a
> source duotone preset recovers ink count and ink names, and MUST report the curve as unrecovered
> rather than fabricating one.

**Grayscale conversion** carries a tonal mapping selection (a channel-weight set or a curve).
**Multichannel conversion** produces one spot channel per source channel and is lossy for
composite viewing.

**[STU-COL-120] Bit depth.** `bits_per_channel` is a four-value enumeration with the numeric values
`1`, `8`, `16`, `32`. 1-bit is valid only for `bitmap` mode. 32-bit is floating point, linear-light,
unbounded, and is the HDR and scene-linear precision. Tool and filter availability is legitimately
reduced at 32-bit; the reduction MUST be reported per operation rather than discovered by failure.

**[STU-COL-121] Per-model bit-depth availability.** Studio's normative availability:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Model | 1 | 8 | 16 | 32 |
|---|---|---|---|---|
| `bitmap` | yes | no | no | no |
| `grayscale` | no | yes | yes | yes |
| `rgb` | no | yes | yes | yes |
| `cmyk` | no | yes | yes | no |
| `lab` | no | yes | yes | no |
| `indexed` | no | yes | no | no |
| `duotone` | no | yes | yes | no |
| `multichannel` | no | yes | yes | no |

The source type library declares `bits_per_channel` as a document-level property with NO per-model
restriction; the restrictions above are Studio's normative choice, not a captured constraint, and
are labelled as such. An implementer MUST NOT report them as vendor-derived.

**[STU-COL-122] HDR luminance model.** A 32-bit document carries an explicit HDR configuration:
`hdr_transfer` (`linear` | `pq` | `hlg`), `hdr_reference_white`, `hdr_peak_luminance`, and
`hdr_system_gamma` (HLG only).

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `hdr_reference_white` | 100 | 1000 | UNKNOWN | UNKNOWN | UNKNOWN | nits | UNKNOWN |
| `hdr_peak_luminance` | 100 | 10000 | UNKNOWN | UNKNOWN | UNKNOWN | nits | UNKNOWN |
| `hdr_system_gamma` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |

The two declared bounds are read from the captured grading surface, which exposes an HDR white
control bounded 100-1000 and an HDR range control bounded 100-10000, both in nits.

**[STU-COL-123] 32-bit preview control.** A non-destructive preview transform MUST exist for 32-bit
documents, carrying `preview_exposure`, `preview_gamma`, and a display-transform selection
(`icc_display` | `unmanaged_linear` | `ocio_display`). It changes SCREEN PRESENTATION ONLY and MUST
NOT change stored document values, MUST NOT be baked into an export unless explicitly requested, and
MUST be reported in the Argus inspection state.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `preview_exposure` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.0 | stops | UNKNOWN |
| `preview_gamma` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 1.0 | dimensionless | UNKNOWN |

**[STU-COL-124] Auto tone mapping.** A document MAY carry `auto_tone_map` (bool) selecting whether
mismatched-dynamic-range sources are automatically tone-mapped into the working space. The captured
sequence presets set this true on 27 of 27 presets that declare it; Studio's default is therefore
`true`, and it MUST be defeatable per document and per placed asset.

---

### 14.8.4 ICC Profiles and the Working Space

**[STU-COL-125] `StudioColorProfile`.** Schema id `hsk.studio.color_profile@1`. Fields:
`profile_id`, `display_name`, `color_model`, `profile_class` (`input` | `display` | `output` |
`device_link` | `color_space` | `abstract` | `named_color`), `icc_version` (`v2` | `v4`),
`pcs` (`xyz` | `lab`), `rendering_intents_supported` (set), `is_matrix_trc` (bool),
`is_lut_based` (bool), `white_point`, `content_hash`, and the profile BYTES held in
content-addressed artifact storage. A profile is identified by `content_hash`, never by file path or
display name, so a profile with the same name on two hosts cannot silently differ.

**[STU-COL-126] Profile classes Studio MUST parse.** ICC v2 and v4, matrix/TRC and LUT-based
(`mft1`, `mft2`, `mAB `, `mBA `), with `A2B0`/`A2B1`/`A2B2` and `B2A0`/`B2A1`/`B2A2` tag selection
by intent, `rTRC`/`gTRC`/`bTRC` curves (both `curv` and `para` forms), `wtpt`, `chad`,
`rXYZ`/`gXYZ`/`bXYZ`, `gamt`, and `ncl2` named-colour tags. Named-colour profiles are how a spot ink
library binds to measured values ([STU-COL-185]).

**[STU-COL-127] Document working spaces.** A `StudioDocument` carries FOUR working-space bindings,
independently settable: `working_space_rgb`, `working_space_cmyk`, `working_space_gray`,
`working_space_spot`. Wide-gamut spaces are ordinary ICC profiles and require no special case.
A document additionally carries a `blending_space` ([STU-COL-160]) that MAY differ from the working
space.

**[STU-COL-128] Colour-management policy.** A document carries the policy record:
`enable_color_management` (bool), `rgb_policy`, `cmyk_policy`, `gray_policy` (each
`off` | `preserve_embedded` | `convert_to_working`), `mismatch_ask_when_opening` (bool),
`mismatch_ask_when_pasting` (bool), `missing_ask_when_opening` (bool), `engine` (a named CMM
selection resolved against an `engine_list`), `intent` (the default rendering intent),
`use_black_point_compensation` (bool), `accurate_lab_spots` (bool),
`idealized_black_to_screen` (bool) and `idealized_black_to_export` (bool). These twelve fields are
each independently stored; the last two are the appearance-of-black control of [STU-COL-215] and are
separate because screen and output policy differ.

A named colour-settings BUNDLE binds a full policy record plus the four working spaces, is
importable and exportable as a file, and is selectable by name.

**[STU-COL-129] Embedded-profile handling.** A placed asset MAY carry its own embedded profile. On
placement the document applies the matching policy. A document carries
`placed_vector_profile_policy` separately from the raster policies, because a vector or PDF
placement may carry per-object profiles rather than one document profile.

**[STU-COL-130] Assign profile.** `assign_profile` RETAGS: it changes the profile reference and
leaves component values unchanged, so the numbers stay and the appearance changes. Its variants are
`no_color_management` (strip the tag), `working` (tag with the document working space) and
`custom` (tag with a named profile).

**[STU-COL-131] Convert to profile.** `convert_to_profile` CONVERTS: it changes component values so
the appearance is preserved, and retags. It carries `destination_profile`, `intent`,
`black_point_compensation` (bool), `dither` (bool), and `flatten_image` (bool). Assign and convert
are DIFFERENT operations with opposite invariants and MUST NOT share one command.

**[STU-COL-132] Embed on export.** Every export recipe carries an explicit profile-embedding
selection: `dont_include`, `include_all`, `include_source`, `include_destination`, or
`leave_unchanged`. The default is `include_destination` for any export that converts and
`include_source` for any export that does not.

**[STU-COL-133] Colour conversion strategy at export.** An export recipe carries
`color_conversion_strategy` (`no_conversion` | `convert_to_destination` | `repurpose`) and a
`color_destination` selection (`none`, `document_cmyk`, `working_cmyk`, `document_rgb`,
`working_rgb`, `named_profile`). `repurpose` converts only values that are outside the destination
and leaves in-gamut values untouched; it is a distinct third strategy and MUST NOT be folded into
either of the others.

**[STU-COL-134] Device-link profiles.** A device-link profile MUST be usable as a single-step
conversion that bypasses the PCS. When one is selected, `intent` and `black_point_compensation` are
inert and MUST be reported as inert rather than silently applied.

**[STU-COL-135] Rendering intents (NORMATIVE ENCODING).** Four intents. The source applications
encode them with DIFFERENT integer values (one uses perceptual=1, saturation=2,
relative_colorimetric=3, absolute_colorimetric=4; another uses perceptual=0, saturation=1,
relative_colorimetric=2, absolute_colorimetric=3). Studio fixes its own encoding and converts at
every import boundary:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio `intent` | Studio value | ICC intent number |
|---|---|---|
| `perceptual` | 0 | 0 |
| `relative_colorimetric` | 1 | 1 |
| `saturation` | 2 | 2 |
| `absolute_colorimetric` | 3 | 3 |

Studio's value equals the ICC intent number, which neither source application does. Any import that
carries a vendor integer MUST be mapped by name, never by numeric passthrough.

**[STU-COL-136] Black point compensation.** BPC is an INDEPENDENT boolean, never bundled into the
intent. It is selectable separately on: convert-to-profile, soft-proof, export conversion, display
transform, and per-adjustment colour conversion. A UI that offers "relative colorimetric with BPC"
as a fifth intent is non-conformant.

**[STU-COL-137] Default intent semantics.** `default_rendering_intent` is a document property
applying to conversions that do not name one. A separate `color_rendering_dictionary` selection
exists for PostScript-family output and is stored independently.

**[STU-COL-138] Gamut mapping obligation.** For an out-of-gamut value under
`relative_colorimetric` or `absolute_colorimetric`, the engine MUST clip and MUST be able to report
that clipping occurred per pixel or per object, because that report is what drives gamut warning
([STU-COL-244]). Under `perceptual` and `saturation` the engine uses the profile's own mapping
tables and MUST NOT substitute a clip.

**[STU-COL-139] Chromatic adaptation.** Conversions between profiles with different white points
MUST apply chromatic adaptation using the profile's `chad` tag where present, and a declared default
adaptation matrix where absent. The default MUST be recorded in the engine configuration, not left
implicit, because it changes results.

---

### 14.8.5 The Colour Engine

**[STU-COL-140] Native colour engine (RESTATEMENT AND HARDENING of [STU-COL-006]).** The colour
transform engine, ICC parsing and evaluation, OCIO configuration handling, gamut mapping, LUT
evaluation, ink and separation mathematics, and grading mathematics are owned by the `studio-engine`
crate behind the `ColorEngine: Send + Sync` trait ([STU-ARC-002]). Studio MUST NOT depend on a
platform colour management module (Windows ICM/WCS, macOS ColorSync), on a system-installed
LittleCMS, or on a subscription colour service, at runtime, on any platform, in any build profile,
including tests.

**[STU-COL-141] No renderer applies ICC (STATED AS A REQUIREMENT, NOT AN ASSUMPTION).** No Rust
rendering, rasterisation or GPU crate in the intended stack consumes ICC transforms natively. GPU
and 2D rendering crates operate on untagged numeric buffers and know nothing about profiles. This
module therefore does NOT assume that a crate provides colour management, and it does not name one
that does. **Studio MUST implement ICC parsing and transform evaluation itself inside `ColorEngine`,
or vendor a pure-Rust colour-management implementation as a managed workspace dependency behind that
trait.** Either way `ColorEngine` is the only owner. An implementer who cannot find a crate that
does this has not found a gap in the specification; the specification is telling them to build it.

**[STU-COL-142] Transform materialisation.** `ColorEngine` MUST materialise every transform into a
form `RenderEngine` can consume without knowing about colour: either an explicit matrix-plus-curve
chain, or a baked 3D LUT with a declared grid size and domain, plus a shaper 1D LUT where the
transform is non-linear. The matrix-plus-curve form is EXACT and carries no interpolation error at
all, so it MUST be preferred wherever both profiles reduce to curves and a 3x3; baking a LUT for a
transform that has a matrix form introduces error for nothing. Pixels MUST NOT reach `RenderEngine`
with an unresolved profile reference. The materialised transform is CACHEABLE and MUST be keyed by
(source profile hash, destination profile hash, intent, BPC, precision, grid size, domain,
INTERPOLATION RULE, shaper presence and shaper identity). The interpolation rule is part of the key
and part of the artefact's declared fields for the reason [STU-COL-353] gives: two transforms built
from the same profiles, the same grid and the same domain but evaluated under different
interpolation rules are DIFFERENT transforms and MUST NOT share a cache entry.

**[STU-COL-143] Materialisation determinism (SCOPED to the materialisation path).** For a fixed key
under [STU-COL-142], the MATERIALISED transform MUST be bit-identical on every host. Interpolation
rule, rounding mode, evaluation order and clamping behaviour are part of the engine contract, not
implementation freedom. This is a promotion-equivalence requirement of 14.24: a materialised
transform that differs by one least-significant bit across hosts fails promotion. This obligation
attaches to the ARTEFACT that [STU-COL-142] caches and NOT to the per-pixel application of it; the
two paths are separated by [STU-COL-350], and the application path is held to the numeric tolerance
of [STU-COL-352] instead. Read without that separation this clause is unsatisfiable by any
implementation that dispatches on the processor features it finds at runtime, which is every
implementation in the field, so the separation is what makes the requirement real rather than
aspirational.

**[STU-COL-144] Precision contract.** Transforms evaluate at a declared internal precision that is
at least as high as the higher of the source and destination document precisions, and never lower
than 16-bit. A 32-bit float pipeline evaluates in float throughout and MUST NOT round-trip through
an 8-bit or 16-bit intermediate.

---

### 14.8.6 OCIO and Scene-Linear Colour

**[STU-COL-150] OCIO support.** For scene-linear and 32-bit float work, Studio MUST support an
OpenColorIO-class configuration, at OCIO profile version 1 AND version 2.x semantics, because both
ship in the field. A configuration is a document-scoped or application-scoped resource identified by
content hash.

**[STU-COL-151] Configuration surface.** A parsed configuration exposes: `ocio_profile_version`,
`name`, `description`, `search_path`, `luma_coefficients`, a `roles` map, a `colorspaces` list, a
`displays` map (display -> ordered views), a `view_transforms` list, a `looks` list, and a
`display_colorspaces` list. Every one of these MUST be surfaced to the operator and to the model
surface; a configuration reduced to "a list of colour space names" is non-conformant.

**[STU-COL-152] Roles.** The role vocabulary Studio MUST recognise, at minimum:
`default`, `reference`, `data`, `scene_linear`, `rendering`, `compositing_linear`,
`compositing_log`, `color_timing`, `color_picking`, `matte_paint`, `texture_paint`. A role is an
indirection: an operation that names a role resolves through the configuration, so the same document
behaves correctly under two different configurations.

**[STU-COL-153] Captured configuration scale (for sizing, not as a bundling requirement).** The
captured install shipped three configurations totalling **406 colour spaces and 54 views**:
one at OCIO profile version 1 with 353 colour spaces, 26 views, 1 display and 11 roles; one at
version 2.1 with 14 colour spaces, 6 views, 5 displays and 9 roles; one at version 2.1 with 39
colour spaces, 18 views, 9 displays and 9 roles. Luma coefficients in the version-1 configuration
were 0.2126 / 0.7152 / 0.0722. Studio MUST handle a configuration of at least this scale without
degradation. Studio does NOT bundle any vendor configuration; the operator supplies one.

**[STU-COL-154] Display transform selection.** A document carries a display-transform selection with
three mutually exclusive modes: `icc_display` (the ICC path of 14.8.4), `unmanaged_linear` (no
transform; values go to the display unchanged), `ocio_display` (a (display, view) pair from the
active configuration). The selection MUST be visible in the Argus inspection state, because a
screenshot alone cannot distinguish the three.

**[STU-COL-155] Colour-space conversion nodes.** A document MAY carry explicit colour-space
conversion operations naming an OCIO source and destination colour space, or a role. These are
document operations, not view settings, and they DO change stored values.

**[STU-COL-156] Working colour space for timeline and sequence work.** A timeline or sequence
declares a working colour space. The captured sequence presets used two: `BT.709 RGB Full` on 22
presets and `BT.2100 HLG RGB Full` on 5. Studio's working-space selection is an open registry of
named spaces, not a two-value enumeration, and MUST include at minimum BT.709, BT.2020, BT.2100 PQ,
BT.2100 HLG, DCI-P3, Display P3, sRGB, Linear Rec.709, Linear Rec.2020, ACES2065-1, ACEScg, ACEScc
and ACEScct.

**[STU-COL-157] ICC and OCIO coexistence.** ICC and OCIO are two paths through the SAME
`ColorEngine`, not two engines. A document declares which path governs its display transform and
which governs its export transform; the two MAY differ (OCIO display, ICC export is a normal
configuration). A document MUST NOT be forced to choose one path for everything.

---

### 14.8.7 LUTs

**[STU-COL-158] LUT primitive.** A LUT is a first-class colour resource:
`lut_kind` (`lut_1d` | `lut_3d` | `shaper_plus_3d`), `grid_size`, `domain_min` (a 3-vector),
`domain_max` (a 3-vector), `input_space`, `output_space`, `interpolation`
(`trilinear` | `tetrahedral`), `content_hash`, and the sample payload in content-addressed artifact
storage. `domain_min` and `domain_max` are REQUIRED and MUST NOT default to 0-1 silently: a
scene-linear LUT commonly has a domain outside 0-1 and applying it as if clamped destroys the grade.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `lut.grid_size` | 2 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | count | 0 |

Observed grid sizes across the captured 288-file LUT corpus: 21 (98 files), 16 (5), 32 (2), 2 (6),
256 (1), 1024 (3). `hard_min` 2 follows from the format; `hard_max` is not declared by any source
and is `UNKNOWN`. The 256 and 1024 sizes appear in files whose dimensionality was not confirmed by
the capture and MUST NOT be assumed to be 3D.

**[STU-COL-159] LUT formats.** Studio MUST import at minimum the text cube format (with `TITLE`,
`LUT_1D_SIZE` / `LUT_3D_SIZE`, `DOMAIN_MIN`, `DOMAIN_MAX` and sample rows), the 3DL format, and a
shader-stack look format. Studio MUST export the text cube format. Format counts in the captured
corpus, recorded so an implementer sizes the importer correctly: 117 in one interchange text format,
115 in the cube format, 45 in a vendor binary format, 6 in a vendor 1D format, 3 look files, 2 other.
A LUT file IS a file, so those are file counts; every other count in this module counts entities.

**[STU-COL-160] LUT and grade application order (NORMATIVE).** When a colour operation stack
contains more than one of these, they apply in exactly this order and the order is not
configurable:
1. **Input / technical LUT** - converts a camera or source encoding into the working space.
2. **Primary correction** - white balance, exposure, contrast, tone, saturation ([STU-COL-220]).
3. **Creative look LUT** with its intensity mix ([STU-COL-221]).
4. **Look adjustments** - the post-look tone and colour trim.
5. **Curves** - RGB curves, then hue/saturation curves ([STU-COL-225], [STU-COL-226]).
6. **Colour wheels** - lift/gamma/gain and their shadow/midtone/highlight split ([STU-COL-222]).
7. **Secondary corrections** - keyed or masked regions, in stack order ([STU-COL-227]).
8. **Vignette** ([STU-COL-228]).
9. **Output / display LUT or display transform**.
A stack that applies a creative look before the technical input LUT produces a different and wrong
image. This ordering is part of the determinism contract of [STU-COL-143].

---

### 14.8.8 Where Colour Management Sits Relative to the Compositor

**[STU-COL-161] Pipeline position (NORMATIVE).** Colour management sits BELOW the compositor on the
input side and ABOVE the compositor on the output side. The compositor never sees a device value and
never performs a profile conversion.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Stage | Owner | Operates in |
|---|---|---|
| 1. Decode | interop (14.13) | The asset's own encoding |
| 2. Input transform | `ColorEngine` | Asset space -> document working space |
| 3. Layer content evaluation, adjustments, filters, effects | `RasterEngine` / `VectorEngine` / `RenderEngine` | Document working space, document precision |
| 4. Blending and compositing | `RenderEngine` | Document BLENDING space ([STU-COL-162]) |
| 5. Document grade and LUT stack | `ColorEngine` + `RenderEngine` | Working space, in the order of [STU-COL-160] |
| 6a. Display transform | `ColorEngine` | Working space -> display |
| 6b. Export transform | `ColorEngine` | Working space -> destination profile, with intent and BPC |
| 6c. Proof transform | `ColorEngine` | Working -> proof device -> display, two intents ([STU-COL-241]) |
| 7. Inspection overlays (gamut warning, separations preview, ink limit) | `ColorEngine` | Computed on the 6c branch, never on authority values |

**[STU-COL-162] Blending space.** A document declares a `blending_space` (`rgb` | `cmyk`)
independently of its working space. Blend modes, opacity and transparency group compositing evaluate
in the blending space. A CMYK document that blends in RGB and a RGB document that blends in CMYK are
both legal and both change the result, so the field MUST be explicit, stored, and reported - never
inferred from the document mode.

**[STU-COL-163] Compositor purity.** `RenderEngine` receives buffers already in the working or
blending space and receives materialised transforms ([STU-COL-142]) as data. It MUST NOT call
`ColorEngine` per pixel, MUST NOT resolve a profile, and MUST NOT decide an intent. This is what
keeps the GPU path free of colour-management state and keeps `handshake_core` free of GPU
dependencies ([STU-ARC-002]).

**[STU-COL-164] Overlay purity.** Gamut warning, out-of-gamut markers, separation previews, ink
limit views and proof previews are OVERLAYS computed from the proof branch. They MUST NOT be
composited into the document, MUST NOT be present in an export unless the export explicitly requests
a proof rendering, and MUST be individually queryable as inspection state so a headless model can
read "12.4 percent of pixels out of gamut" rather than looking at a picture.

**[STU-COL-165] Placed-asset colour independence.** Every placed asset carries its own profile and
its own input transform. Changing the document working space MUST re-derive every placed asset's
input transform and MUST NOT alter the assets' stored values.

---

### 14.8.9 Swatches

**[STU-COL-170] `StudioSwatch`.** Schema id `hsk.studio.swatch@1`. Fields: `swatch_id`, `name`,
`swatch_kind`, `color_model`, `color_space`, `components`, `profile_ref`, `base_swatch_ref`
(tints and derived swatches), `alternate_space` and `alternate_color_value` (spot and mixed ink),
`group_ref`, `color_editable` (bool), `color_removable` (bool), `visible` (bool),
`creator_id`, and `is_root_group_member` (bool).

**[STU-COL-171] Swatch kinds.** Seven kinds plus four reserved swatches.

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per swatch kind.*

| `swatch_kind` | Behaviour |
|---|---|
| `process` | Ordinary process colour in the document model. |
| `global` | Process colour that live-updates every use when edited. |
| `spot` | Named separation ink; separates as its own plate; tintable; carries an optional measured definition. |
| `mixed_ink` | One or more spot inks combined with process inks at declared percentages. |
| `mixed_ink_group` | A generated, editable stepped series over a set of base inks. |
| `tint` | A percentage of a base swatch that tracks the base. |
| `gradient` | A named `StudioGradient` stored as a swatch. |

Reserved swatches, always present and not deletable: `none`, `paper` (the substrate colour, which
is not white and is not printed), `registration` (marks on every plate), `black`.

**[STU-COL-172] Global propagation.** Editing a `global` or `spot` swatch MUST live-update every
object fill, object stroke, text range fill, text range stroke, gradient stop, pattern colour,
paragraph decoration colour and adjustment colour that references it, as ONE deterministic
operation emitting ONE history entry, without disturbing per-object overrides such as tint
percentage or opacity.

**[STU-COL-173] Spot definition.** A `spot` swatch carries: `ink_name`, an optional MEASURED
definition (Lab, or a named-colour profile entry) used for accurate screen and proof simulation, and
an `alternate_space` + `alternate_color_value` used when the spot is converted to process. The spot
separates as its own plate REGARDLESS of the alternate. `spot_kind` records the definition space:
`cmyk`, `rgb` or `lab`. A spot with `no_alternate` ([STU-COL-116]) has no process approximation and
converting it is an explicit, warned operation.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `spot_tint` | 0.0 | 100.0 | 0.0 | 100.0 | 100.0 | percent | 1 |

`spot_tint` default is 100.0, not 0.0: an untinted spot is full strength.

**[STU-COL-174] Tint swatches.** A `tint` swatch stores `base_swatch_ref` and `tint_value`, and
recomputes when the base changes. It is not a copy.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `tint_value` | 0.0 | 100.0 | 0.0 | 100.0 | UNKNOWN | percent | 1 |

**[STU-COL-175] Mixed inks.** A `mixed_ink` swatch stores an `ink_list` (references to inks), an
`ink_percentages` list aligned to it, an `ink_name_list`, and the spot subset as
`mixed_ink_spot_color_list` with its names. Its `color_space` is `mixed_ink`.

**[STU-COL-176] Mixed-ink groups.** A `mixed_ink_group` stores its base inks and generates a stepped
series. Each generated member is an editable `mixed_ink` swatch; editing a member does not break the
group, and editing the group regenerates unedited members only.

**[STU-COL-177] Swatch groups.** Swatches organise into named groups. Exactly one ROOT group exists
per palette and is not deletable. A swatch belongs to at most one group. Group operations:
create, rename, delete (with a replace-on-delete target), reorder, and move-between-groups.

**[STU-COL-178] Palette scope.** Three scopes: `document`, `application`, `system`. A document
palette travels with the document; an application palette is available to every document on the
host; a system palette reflects the OS palette and is read-only. A swatch reference resolves in
document -> application -> system order and an unresolvable reference is a validation error, never a
silent black.

**[STU-COL-179] Swatch panel operations.** Create, duplicate, edit, delete with replace-on-delete,
merge selected swatches into one, add-all-unnamed-colours-in-document, add-all-used-colours,
select-all-unused, delete-all-unused, sort by name and by colour value, and colorize a
grayscale or bitmap placed asset with a chosen swatch. Each is a typed command, model-invokable and
individually undoable.

**[STU-COL-180] Swatch interchange.** Studio MUST import and export a portable swatch exchange
format carrying, per entry: name, colour model (`RGB` | `CMYK` | `LAB` | `GRAY`), components as
floats, and colour type (`global` | `spot` | `normal`), with group blocks. Studio MUST also import
and export a Studio-native palette file that additionally carries profile references, tint and
mixed-ink structure, and reserved-swatch identity, none of which the portable format can express.

> **Coverage note of record.** The raster application ships ZERO files of the portable exchange
> format, so the format's behaviour was not exercised against that application's own data during
> capture. It IS exercised by the vector application, which ships 40 such libraries. This is a gap
> in evidence, not a gap in the format.

**[STU-COL-181] Import from another Studio document.** Loading swatches from another `StudioDocument`
MUST preserve kind, group membership, base-swatch relationships and profile references, and MUST
report name collisions with an explicit resolution (`rename`, `replace`, `skip`) rather than
silently merging.

**[STU-COL-182] Swatch library scale (for sizing).** The captured installs ship, as swatch and
colour-book data: 8,901 swatch entries across 24 containers and 5,243 colour-book colours across
12 books in the raster application; 3,155 swatch entries, 10,011 colour-book colours, 659 gradient
entries and 382 pattern entries across 118 swatch libraries in the vector application. Studio's
swatch surfaces MUST remain responsive at this scale; a panel that degrades above a few hundred
entries is non-conformant.

**[STU-COL-183] Find and replace colour.** Studio MUST find every usage of a colour - across object
fills, object strokes, text fills, text strokes, gradient stops, pattern colours, paragraph
decoration and adjustment parameters - and replace it with another, operating on `StudioSwatch` and
`StudioColorProfile` references rather than on numeric equality. Numeric-equality matching MUST also
be offered as a separate mode with an explicit tolerance.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `color_match_tolerance` | 0.0 | UNKNOWN | UNKNOWN | UNKNOWN | 0.0 | delta_e | UNKNOWN |

---

### 14.8.10 Branded Colour Libraries

Supersedes [STU-COL-021].

**[STU-COL-184] Native spot is licence-free (retained from [STU-COL-020]).** The
`StudioSwatch(spot)` primitive - a named separation ink with an optional measured definition, a tint
control, an alternate space, and full separation, proofing, overprint, ink-manager and
separations-preview behaviour - ships and functions with NO external library and NO licence. Nothing
about branded books gates spot colour.

**[STU-COL-185] Colour-book resource.** A branded book is a DATA ADAPTER that populates native spot
swatches. A book resource carries: `book_id`, `title`, `name_prefix`, `name_postfix`,
`description` (including the rights holder's own copyright string), `declared_count`,
`color_space` (`lab` | `cmyk` | `rgb`), `components_per_color`, `ink_type` (`spot` | `process`),
`page_size` and `page_selector_offset` (the physical fan-deck layout, which is what makes a book
navigable rather than a flat list), and the entries. Each entry carries `name`, `code` and
`components`. The displayed swatch name is `name_prefix + name + name_postfix`; a book with an empty
prefix and postfix is legal.

**[STU-COL-186] Per-book licensing posture (NORMATIVE).** Licensing is PER BOOK, not blanket.
Each book resource declares a `license_state` (`bundled` | `operator_supplied` | `absent`) and a
`license_note`. Studio MUST NOT bundle a book whose rights holder does not permit redistribution,
MUST load an operator-supplied book from the same resource format, and MUST degrade gracefully when
a book is absent: a document referencing an absent book keeps every spot swatch functional with its
stored measured definition and its stored name, and loses only the ability to LOOK UP further
colours from that book. Losing the lookup MUST NOT change any rendered colour.

**[STU-COL-187] Book coverage recorded for planning.** The captured installs bundle five branded
book families (ANPA, DIC, FOCOLTONE, HKS in four variants with process companions, TRUMATCH) and do
NOT bundle PANTONE. This is evidence of what a vendor's licence position looked like at capture
time; it is NOT a Studio bundling instruction. Studio's own bundling decision is an operator
decision recorded per book in `license_state`, and this module does not make it.

**[STU-COL-188] Named-colour profile binding.** Where a book is supplied as an ICC named-colour
profile (`ncl2`), Studio MUST bind it through [STU-COL-126] rather than through a bespoke parser, so
the measured values drive proofing directly.

---

### 14.8.11 Gradients and Patterns

**[STU-COL-190] Gradient geometry.** `StudioGradient` (schema id `hsk.studio.gradient@1`) is one
primitive discriminated by geometry. SEVEN geometries, all required:

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per gradient geometry.*

| Gradient geometry | Structure |
|---|---|
| `linear` | Axis with angle, origin and length. |
| `radial` | Centre, radius, aspect ratio, plus a highlight offset (`hilite_angle`, `hilite_length`). |
| `elliptical` | Radial with independent axes. |
| `angular` | Sweep around a centre. |
| `diamond` | Rhombic distance field from a centre. |
| `freeform` | Free-placed colour points or colour lines, not bound to an axis. |
| `mesh` | An editable grid of mesh points interpolating across patches. |

`diamond` was absent from the superseded [STU-COL-010] and is present in the captured design-tool
paint model alongside linear, radial and angular. `bitmap` is NOT a gradient geometry; an image fill
is a distinct paint kind ([STU-COL-197]).

**[STU-COL-191] Gradient stops.** A gradient carries an ordered list of colour stops and an
independent ordered list of OPACITY stops. The two lists are separate and MUST NOT be forced to
align, because a gradient commonly has three colour stops and two opacity stops. A colour stop
carries `position`, `color` (a tagged value or swatch reference) and `midpoint`. An opacity stop
carries `position`, `opacity` and `midpoint`.

**[STU-COL-192] Gradient stop bounds (NORMATIVE PARAMETER TABLE).**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `gradient_stop.position` | 0.0 | 100.0 | 0.0 | 100.0 | UNKNOWN | percent | UNKNOWN |
| `gradient_stop.midpoint` | 13.0 | 87.0 | 13.0 | 87.0 | 50.0 | percent | UNKNOWN |
| `gradient_stop.opacity` | 0.0 | 100.0 | 0.0 | 100.0 | 100.0 | percent | UNKNOWN |
| `gradient.angle` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.0 | degrees | UNKNOWN |
| `gradient.length` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.0 | document_unit | UNKNOWN |
| `gradient.hilite_angle` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.0 | degrees | UNKNOWN |
| `gradient.hilite_length` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.0 | document_unit | UNKNOWN |

The `midpoint` bound of 13.0-87.0 is DECLARED, not invented, and is the single most commonly
mis-implemented bound in this module. A midpoint control that offers 0-100 emits values the engine
rejects. Its default is 50.0 (the geometric centre), which is inside the bound.

**[STU-COL-193] Gradient controls.** `reverse` (bool), `dither` (bool, to suppress banding),
`interpolation` (`linear` | `perceptual`), and stroke application
(`within_stroke` | `along_stroke` | `across_stroke`). Interpolation choice CHANGES the rendered
result and MUST be stored, not treated as a rendering preference.

**[STU-COL-194] Mesh and freeform structure.** A `mesh` gradient stores an editable grid of mesh
points, each carrying a colour and an independent transparency, interpolating across the patch; a
`linear` or `radial` gradient is convertible to `mesh` and the conversion is one-way and warned.
A `freeform` gradient stores either free-placed colour POINTS or colour LINES (a `points` /
`lines` mode selector), each carrying a spread and an opacity, none bound to an axis. Both remain
`StudioGradient` values discriminated by `geometry`, not separate primitives.

**[STU-COL-195] `StudioPattern`.** Schema id `hsk.studio.pattern@1`. A pattern carries:
`tile_type` (`grid` | `brick_by_row` | `brick_by_column` | `hex_by_row` | `hex_by_column`),
`brick_offset`, `tile_width`, `tile_height`, `horizontal_spacing`, `vertical_spacing`,
`overlap_order` (which tile draws on top at an overlap), `move_tile_with_art` (bool), and the tile
content as a `StudioLayer` subtree.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `pattern.brick_offset` | 0.0 | 100.0 | 0.0 | 100.0 | UNKNOWN | percent | UNKNOWN |
| `pattern.tile_width` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | document_unit | UNKNOWN |
| `pattern.tile_height` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | document_unit | UNKNOWN |
| `pattern.horizontal_spacing` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | document_unit | UNKNOWN |
| `pattern.vertical_spacing` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | document_unit | UNKNOWN |

**[STU-COL-196] Pattern placement transform.** A pattern fill carries a transform INDEPENDENT of the
object it fills: `rotation`, `scale_factor`, `shear_angle`, `shear_axis`, `shift_angle`,
`shift_distance`, `reflect` (bool) and `reflect_angle`. Transforming the object MUST offer the
choice of transforming the pattern with it or leaving the pattern fixed. Every one of these eight
fields defaults to a neutral value (0.0 or `false`).

**[STU-COL-197] Paint kind enumeration.** A fill or stroke entry declares one paint kind:
`none`, `solid`, `gradient`, `pattern`, `image`, `video`, `shader`. `image` carries a
`scale_mode` (`fill` | `fit` | `crop` | `tile`), an image reference, a placement transform, a
scaling factor, a rotation, and an image filter block. `shader` is a procedural paint carrying a
shader id and a property map. Every paint entry additionally carries `visible`, `opacity` and
`blend_mode`.

**[STU-COL-198] Blend-mode enumeration (NORMATIVE).** Twenty-eight modes. The source applications
disagree on both the set and the integer encoding, so Studio fixes its own list and maps by NAME at
every import boundary:
`pass_through`, `normal`, `dissolve`, `behind`, `clear`, `darken`, `multiply`, `color_burn`,
`linear_burn`, `darker_color`, `lighten`, `screen`, `color_dodge`, `linear_dodge`, `lighter_color`,
`overlay`, `soft_light`, `hard_light`, `vivid_light`, `linear_light`, `pin_light`, `hard_mix`,
`difference`, `exclusion`, `subtract`, `divide`, `hue`, `saturation`, `color`, `luminosity`.
One captured application ships 16 modes, another 28, another 19. `pass_through` is meaningful only
on a group. `behind` and `clear` are paint-application modes, not layer modes. Blend modes evaluate
in the blending space of [STU-COL-162].

---

### 14.8.12 Inks, Separations and Overprint

**[STU-COL-205] Ink resource.** An ink is a first-class document resource with ELEVEN fields:
`name`, `ink_type`, `is_process_ink` (bool), `convert_to_process` (bool), `alias_ink_name`,
`neutral_density`, `trap_order`, `angle`, `frequency`, `solidity`, `print_ink` (bool).
`alias_ink_name` maps one ink onto another so two differently named spots separate onto one plate.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `ink.neutral_density` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |
| `ink.trap_order` | 1 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | count | 0 |
| `ink.angle` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | degrees | UNKNOWN |
| `ink.frequency` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | lines_per_inch | UNKNOWN |
| `ink.solidity` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |

**[STU-COL-206] Ink type enumeration.** Four values: `normal`, `transparent`, `opaque`,
`opaque_ignore`. These drive trapping behaviour: `transparent` inks trap through, `opaque` inks trap
against, `opaque_ignore` inks are excluded from trap computation.

**[STU-COL-207] Ink print status.** Three values per ink: `disable`, `enable`, `convert`.
`convert` means convert this spot to process at output while leaving the document's swatch a spot.

**[STU-COL-208] Ink manager operations.** Per-ink spot-to-process conversion; all-spots-to-process
as one operation; ink aliasing; a global `use_standard_lab_values_for_spots` toggle (the
`accurate_lab_spots` field of [STU-COL-128]); and per-ink neutral density and trapping sequence.
Every operation is a typed, model-invokable, undoable command operating on native
`StudioSwatch(spot)` inks.

**[STU-COL-209] Composite versus separation output.** `color_separation_mode` is a three-value
enumeration: `composite`, `host_based_separation`, `in_rip_separation`. A `color_output_mode`
selection additionally governs composite output (`composite_leave_unchanged`, `composite_gray`,
`composite_rgb`, `composite_cmyk`). These are output-recipe fields, not document fields.

**[STU-COL-210] Overprint.** Overprint is settable INDEPENDENTLY on an object's fill, its stroke,
and its stroke gap. Three separate booleans, never one. Text carries its own
`overprint_fill` and `overprint_stroke` per character range ([STU-TYP-157]). Paragraph rules,
shading and borders each carry their own overprint and gap-overprint ([STU-TYP-193]-[STU-TYP-195]).
Kenten and ruby carry a THREE-value overprint (`auto` | `on` | `off`) rather than a boolean, because
they inherit from the parent run by default.

**[STU-COL-211] Overprint preview and black overprint.** An overprint-preview mode MUST render the
overprint result on screen. A document-level `overprint_black` policy controls whether 100% K
overprints by default. An export recipe carries `preserve_overprint_settings` (bool) and an
`overprint_mode` selection independently of the document policy.

**[STU-COL-212] Separations preview.** Three inspection facets on one surface:
per-plate visibility toggles (each ink on or off, plus a composite view); an INK LIMIT view with a
configurable total-ink-coverage threshold that highlights areas exceeding it; and a per-ink
percentage-coverage readout at a sampled point. All three MUST be readable as structured inspection
state, not only as pixels ([STU-COL-164]).

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `ink_limit_threshold` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | percent | 0 |

`hard_max` is not a fixed number: total ink coverage can exceed 400% in a document with spot plates,
so a control that clamps at 400 is wrong. The bound is `UNKNOWN` and the engine computes the actual
maximum from the active ink set.

**[STU-COL-213] Trap presets.** A trap preset is a named resource with FIFTEEN fields:
`name`, `default_trap_width`, `black_width`, `trap_join` (`miter` | `round` | `bevel`),
`trap_end` (`miter` | `overlap`), `objects_to_images` (bool), `images_to_images` (bool),
`internal_images` (bool), `one_bit_images` (bool),
`image_placement` (`center` | `choke` | `neutral_density` | `spread`),
`step_threshold`, `black_color_threshold`, `black_density`, `sliding_trap_threshold`,
`color_reduction`. Presets are assignable to page ranges.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `trap.default_trap_width` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | points | UNKNOWN |
| `trap.black_width` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | points | UNKNOWN |
| `trap.step_threshold` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |
| `trap.black_color_threshold` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |
| `trap.black_density` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |
| `trap.sliding_trap_threshold` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |
| `trap.color_reduction` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |

The percent bounds are Studio's normative choice for threshold and reduction fields; the source
declares the FIELDS but not their ranges. They are marked as chosen, not captured, per [STU-COL-107]'s
honesty requirement, and `black_density` remains `UNKNOWN` because it is a measured density, not a
percentage.

**[STU-COL-214] Transparency flattener presets.** A flattener preset is a named resource with SEVEN
fields: `name`, `raster_vector_balance`, `line_art_and_text_resolution`,
`gradient_and_mesh_resolution`, `clip_complex_regions` (bool), `convert_all_strokes_to_outlines`
(bool), `convert_all_text_to_outlines` (bool). A spread MAY override the document preset.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `flattener.raster_vector_balance` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | 0 |
| `flattener.line_art_and_text_resolution` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | ppi | 0 |
| `flattener.gradient_and_mesh_resolution` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | ppi | 0 |

A flattener PREVIEW MUST highlight, as separate selectable overlays: rasterised regions, outlined
strokes, outlined text, regions affected by transparency, and all affected objects.

**[STU-COL-215] Appearance of black.** TWO independent booleans, not one:
`idealized_black_to_screen` and `idealized_black_to_export`. Each controls whether 100% K is
displayed or output as a rich accurate black or as the device's own black on an RGB device. They are
separate because the correct screen answer and the correct export answer routinely differ.

---

### 14.8.13 Camera Colour Rendering

**[STU-COL-218] Camera profile model (NORMATIVE).** A camera profile is a colour-RENDERING model,
not a geometric one. Its structure is:
- TWO illuminant-referenced 3x3 `color_matrix` entries (XYZ to camera), one per calibration
  illuminant. REQUIRED.
- TWO `calibration_illuminant` declarations. REQUIRED.
- TWO optional 3x3 `forward_matrix` entries (camera to XYZ under D50).
- TWO optional 3x3 `camera_calibration` matrices.
- An optional 3D `hue_sat_map` deformation lattice over (hue, saturation, value), with up to two
  illuminant-referenced instances.
- An optional `look_table` lattice of the same shape, applied AFTER the hue/sat map.
- An optional 1D `profile_tone_curve`.
- Optional `baseline_exposure_offset`, `default_black_render`, `profile_look_table_encoding`,
  `profile_embed_policy`, `profile_dynamic_range`, `profile_calibration_signature`,
  `profile_copyright`, `unique_camera_model` and `profile_name`.

Evaluation order is fixed: colour matrix (interpolated between the two illuminants by the estimated
scene white point), then forward matrix, then hue/sat map, then look table, then tone curve.

**[STU-COL-219] Camera profile scale and shape (for sizing and for defaulting).** The captured
corpus was 4,373 profiles, 895 MB, covering 1,429 distinct camera models under 129 profile names.
Tag presence across it:
`unique_camera_model`, `color_matrix_1`, `color_matrix_2`, `calibration_illuminant_1`,
`calibration_illuminant_2`, `profile_name` and `profile_embed_policy` on 100 percent;
`profile_look_table_dims`/`data` on 99.89 percent; `forward_matrix_1`/`2` on 99.68 percent;
`profile_calibration_signature` on 99.52 percent; `profile_copyright` on 96.52 percent;
`profile_tone_curve` on 67.62 percent; `profile_look_table_encoding` and `default_black_render` on
57.19 percent; `baseline_exposure_offset` on 51.70 percent; `profile_hue_sat_map_dims`/`data_1` on
26.48 percent; `hue_sat_map_data_2` on 26.39 percent; `profile_dynamic_range` on 0.02 percent.
The dominant illuminant pair is (standard light A, D65) on 4,365 of 4,373 profiles; five other pairs
occur, including (A, flash), (D65, A), (D55, D75) and (flash, tungsten), so an implementation that
hard-codes A/D65 is wrong on 8 profiles.
The dominant hue/sat lattice is 90 x 30 x 1 (hue x saturation x value) on 1,116 of the 1,158
profiles that carry one; an 8 x 2 x 1 lattice occurs on 22.
Profile payloads are BULK BINARY and belong in content-addressed artifact storage with SurrealDB
holding the record and reference ([STU-SDB-002]); a 895 MB profile corpus does not belong in a
document database.

**[STU-COL-220] Basic tone and white balance parameter block.** These are the primary correction
controls of stage 2 in [STU-COL-160].

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `temperature` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `tint` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `exposure` | -7 | 7 | -5 | 5 | UNKNOWN | stops | UNKNOWN |
| `contrast` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `highlights` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `shadows` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `whites` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `blacks` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `hdr_specular` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `saturation` | 0 | 300 | UNKNOWN | 200 | UNKNOWN | dimensionless | UNKNOWN |
| `vibrance` | -100 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |
| `sharpen` | -100 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |
| `tint_balance` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `white_balance_temperature_kelvin` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 6000 | kelvin | 0 |
| `white_balance_tint_offset` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 18 | dimensionless | 0 |

`saturation` has a declared `soft_max` of 200 and NO declared `soft_min`; that asymmetry is real and
MUST be preserved rather than symmetrised. `default` is `UNKNOWN` for every declared-bound row
because the source serialisation carries no factory value; the two kelvin rows take their defaults
from the observed as-shot modal values and are labelled as such.

**[STU-COL-221] Creative look.** `look` selects a look resource by slot index; `look_intensity`
mixes it.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `look` | 0 | 998 | UNKNOWN | UNKNOWN | UNKNOWN | index | 0 |
| `input_lut` | 0 | 998 | UNKNOWN | UNKNOWN | UNKNOWN | index | 0 |
| `look_intensity` | 0 | 200 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |
| `faded_film` | 0 | 150 | UNKNOWN | 100 | UNKNOWN | dimensionless | UNKNOWN |

A slot index is a reference into an ordered look or LUT registry; the 998 bound is the registry
capacity, not a colour range. Studio's registry MUST support at least 998 entries; the captured
install shipped 325 looks and 288 LUT files.

**[STU-COL-222] Colour wheels: the lift/gamma/gain model (NORMATIVE MATHEMATICS).** Studio's colour
wheels implement the following, recovered from the shipped grading shader source. It is stated as
mathematics, not as prose, because two implementations that differ here produce visibly different
grades.

Per tonal band (overall, shadow, midtone, highlight), given a colour offset vector `offset`, a gamma
vector `gamma`, a gain vector `gain`, and scalars `Temperature`, `Magenta`, `Contrast`, `Pivot`,
`Saturation`, `PostSat`:

```
TempGain.r = pow(2, gain.r + (Temperature + Magenta * 0.5))
TempGain.g = pow(2, gain.g + (-Magenta))
TempGain.b = pow(2, gain.b + (-Temperature + Magenta * 0.5))

Contrast'  = pow(2, Contrast)

offset'.c  = TempGain.c * (offset.c + Pivot - Contrast' * Pivot)     for c in {r,g,b}
gain'.c    = TempGain.c * Contrast'                                   for c in {r,g,b}
gamma'.c   = pow(3, gamma_band.c + gamma.c)                           for c in {r,g,b}
```

Band results compose by adding the band offset to the overall offset, and the band split points are
`lowMid` (shadow/midtone crossover) and `midHigh` (midtone/highlight crossover). Luminance weights
are an explicit 3-vector; the shipped legacy configuration carries `luminance.r = 0.299` and
`luminance.b = 0.114` (Rec.601 luma). Studio stores the luminance weight vector explicitly and
selects it from the working space rather than hard-coding Rec.601.

Declared control values from the shader source: `Pivot` default 0.5, `Contrast` min -1 default 0,
`Saturation` max 2 with clamp-at-min, `PostSat` default (1,1,1), `offset` default (0,0,0),
`gamma` default (0,0,0), `gain` default (0,0,0), `lowMid` default 0.5, `midHigh` default 0.5.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `wheel.pivot` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.5 | unit_interval | UNKNOWN |
| `wheel.contrast` | -1 | UNKNOWN | UNKNOWN | UNKNOWN | 0 | dimensionless | UNKNOWN |
| `wheel.input_saturation` | UNKNOWN | 2 | UNKNOWN | UNKNOWN | 1 | dimensionless | UNKNOWN |
| `wheel.final_saturation` | UNKNOWN | 2 | UNKNOWN | UNKNOWN | 1 | dimensionless | UNKNOWN |
| `wheel.low_mid_split` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.5 | unit_interval | UNKNOWN |
| `wheel.mid_high_split` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0.5 | unit_interval | UNKNOWN |
| `wheel.offset` (per channel) | UNKNOWN | UNKNOWN | -1 | UNKNOWN | 0 | dimensionless | UNKNOWN |
| `wheel.gamma` (per channel) | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | dimensionless | UNKNOWN |
| `wheel.gain` (per channel) | UNKNOWN | UNKNOWN | -1 | UNKNOWN | 0 | dimensionless | UNKNOWN |

**[STU-COL-223] Five-wheel colour grading and the legacy key trap.** Studio's colour-grading surface
has FIVE wheels: shadow, midtone, highlight, global, and a blending/balance pair. Each wheel carries
HUE, SATURATION and LUMINANCE.

> **Trap of record.** In the captured develop model the shadow and highlight HUE and SATURATION
> values have NO modern key. They are stored under LEGACY split-toning keys
> (`SplitToningShadowHue`, `SplitToningShadowSaturation`, `SplitToningHighlightHue`,
> `SplitToningHighlightSaturation`, plus `SplitToningBalance`), while shadow and highlight
> LUMINANCE and the entire midtone and global wheels use the modern keys
> (`ColorGradeShadowLum`, `ColorGradeHighlightLum`, `ColorGradeMidtoneHue`, `ColorGradeMidtoneSat`,
> `ColorGradeMidtoneLum`, `ColorGradeGlobalHue`, `ColorGradeGlobalSat`, `ColorGradeGlobalLum`,
> `ColorGradeBlending`). The four modern keys `ColorGradeShadowHue`, `ColorGradeShadowSat`,
> `ColorGradeHighlightHue` and `ColorGradeHighlightSat` DO NOT EXIST.
>
> An importer that reads only `ColorGrade*` keys silently drops shadow and highlight hue and
> saturation - four of the fifteen grading values. Studio's own model MUST use ONE uniform key
> family across all five wheels, and the importer MUST map the legacy keys explicitly. This is a
> mandatory acceptance case ([STU-COL-262]).

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `grade.<band>.hue` | 0 | 360 | 0 | 360 | 0 | degrees | 0 |
| `grade.<band>.saturation` | 0 | 100 | 0 | 100 | 0 | percent | 0 |
| `grade.<band>.luminance` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `grade.blending` | 0 | 100 | 0 | 100 | 50 | percent | 0 |
| `grade.balance` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |

Bounds for hue and saturation are Studio's normative choice consistent with the observed value
ranges (hue observed to 315, saturation observed to 54, luminance observed to -100); the source
declares no bound and the observed range is a lower bound only ([STU-COL-108]). `grade.blending`
default 50 is the observed modal value and is labelled as such.

**[STU-COL-224] Calibration controls.** Seven controls, all defaulting to 0: `red_hue`,
`red_saturation`, `green_hue`, `green_saturation`, `blue_hue`, `blue_saturation`, `shadow_tint`.
They modify the camera calibration matrix before the profile's hue/sat map.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `calibration.<channel>_hue` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `calibration.<channel>_saturation` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `calibration.shadow_tint` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |

Bounds are Studio's normative choice; the source declares only the default of 0 for all seven.

**[STU-COL-225] Tone curves.** Studio MUST provide FOUR independent tone curves - composite, red,
green, blue - each a point list. A curve point is an `(input, output)` pair on a 0-255 domain by
default, with an explicit domain declaration so a float pipeline can use 0.0-1.0. The identity curve
is the two-point list `[(0,0), (255,255)]` and is the default for all four.

> **Recovery limit of record.** The captured curve preset container stores NO channel identifier at
> all; the channel label was recovered by index order only. Studio's own curve storage MUST carry an
> explicit channel identifier per curve, so its own round trip does not inherit that ambiguity.

A PARAMETRIC tone curve MUST also exist, with four region controls and three split points:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `parametric.highlights` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `parametric.lights` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `parametric.darks` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `parametric.shadows` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `parametric.shadow_split` | 0 | 100 | 0 | 100 | 25 | percent | 0 |
| `parametric.midtone_split` | 0 | 100 | 0 | 100 | 50 | percent | 0 |
| `parametric.highlight_split` | 0 | 100 | 0 | 100 | 75 | percent | 0 |
| `curve_refine_saturation` | 0 | 100 | 0 | 100 | 100 | percent | 0 |

The three split defaults 25 / 50 / 75 are Studio's normative choice; the captured observed values
were 15-18, 50 and 75, and the source declares no default. `curve_refine_saturation` default 100 is
the observed constant value.

**[STU-COL-226] Hue / saturation / luminance curves.** Studio MUST provide the eight cross-channel
curves: hue-vs-hue, hue-vs-saturation, hue-vs-luminance, luminance-vs-saturation,
saturation-vs-saturation, plus a per-band HSL mixer with eight colour bands (red, orange, yellow,
green, aqua, blue, purple, magenta), each with hue, saturation and luminance offsets.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `hsl.<band>.hue` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `hsl.<band>.saturation` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |
| `hsl.<band>.luminance` | -100 | 100 | -100 | 100 | 0 | dimensionless | 0 |

Bounds are Studio's normative choice; the source declares the field set but no ranges.

**[STU-COL-227] Secondary (keyed) correction.** A secondary correction is a stack entry carrying a
key definition, a refinement block, and a correction block. The KEY carries set-colour, add-colour
and remove-colour sampling plus per-axis hue, saturation and luminance ranges with soft edges. The
REFINEMENT carries denoise and blur. The CORRECTION carries its own temperature, tint, contrast,
sharpen and saturation.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `secondary.denoise` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |
| `secondary.blur` | 0 | 1000 | UNKNOWN | 30 | UNKNOWN | pixels | UNKNOWN |
| `secondary.temperature` | -300 | 300 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `secondary.tint` | -300 | 300 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `secondary.contrast` | -150 | 150 | -100 | 100 | UNKNOWN | dimensionless | UNKNOWN |
| `secondary.sharpen` | -100 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |
| `secondary.saturation` | 0 | 300 | UNKNOWN | 200 | UNKNOWN | dimensionless | UNKNOWN |

**Note the divergence, which is real and normative:** `secondary.temperature` and `secondary.tint`
are hard -300/300, while `temperature` and `tint` in the primary block ([STU-COL-220]) are hard
-150/150. Both present a -100/100 control. A shared parameter definition for "temperature" would
silently reject legal secondary values or silently accept illegal primary ones. The two are
DIFFERENT parameters and MUST be defined separately.

**[STU-COL-228] Vignette.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `vignette.amount` | -5 | 5 | -3 | 3 | UNKNOWN | dimensionless | UNKNOWN |
| `vignette.midpoint` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |
| `vignette.roundness` | -100 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | dimensionless | UNKNOWN |
| `vignette.feather` | 0 | 100 | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |

**[STU-COL-229] Colour adjustment catalogue.** The following colour adjustments MUST exist as
`StudioAdjustment` kinds, each non-destructive, each maskable, each reorderable in the effect stack.
The parameter block of each is normative:

*Derivation: catalogue table, splits per row; yields 23 microtasks, one per colour adjustment. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Adjustment | Parameters |
|---|---|
| Levels | input black, input white, gamma, output black, output white, plus per-channel curves |
| Curves | four channel curves ([STU-COL-225]) |
| Exposure | exposure (stops), offset, gamma |
| Brightness / Contrast | brightness, contrast, use-legacy (bool) |
| Vibrance | vibrance, saturation |
| Hue / Saturation / Lightness | master hue, saturation, lightness plus per-band ([STU-COL-226]) |
| Colour Balance | shadow/midtone/highlight x cyan-red / magenta-green / yellow-blue, preserve luminosity (bool) |
| Black and White | six channel weights (red, yellow, green, cyan, blue, magenta), tint enable, tint colour |
| Channel Mixer | per output channel: source r, g, b, constant; plus monochrome (bool) |
| Selective Colour | per colour band: cyan, magenta, yellow, black; relative-or-absolute (bool) |
| Photo Filter | filter colour (Lab), density, preserve luminosity (bool) |
| Gradient Map | a `StudioGradient` reference |
| Posterise | level count |
| Threshold | threshold, true-colour, false-colour |
| Invert | none |
| Recolour | hue, saturation, lightness |
| Shadows / Highlights | shadow amount, highlight amount, plus radius and tonal-width controls |
| Split Toning | balance, highlight hue, highlight saturation, shadow hue, shadow saturation |
| White Balance | temperature, tint |
| Tone Compression | method (enumerated), compression, exposure, gamma |
| LUT (3D) | a `StudioLUT` reference ([STU-COL-158]) |
| OCIO transform | source colour space, destination colour space or (display, view) |
| Soft Proof | profile, intent, black point compensation (bool), gamut check (bool) |

> **Bound honesty for this table.** The captured adjustment corpus recovered the PARAMETER SETS
> above and 4-character storage tags for each, but its numeric values are OBSERVED across 153
> shipped presets, not declared. Every numeric bound in this catalogue is therefore `UNKNOWN`
> unless a different capture declared it, and MUST NOT be clamped to the observed range
> ([STU-COL-108]). Representative observed ranges, recorded as evidence only and NOT as bounds:
> Levels gamma 0.5-2.0; Exposure -6.0-6.0 with gamma 2.2; Brightness -1.0-1.0 and Contrast 0.0-2.0
> on a unit scale; Posterise level count 3-17; Threshold 0.25-0.75; Photo Filter density 0.5-1.0;
> Tone Compression method 0-7.
>
> Note also that the Soft Proof adjustment carries exactly four fields in the captured model -
> profile, intent, black point compensation, gamut check - which independently confirms
> [STU-COL-136]: BPC is a separate field from intent, in a shipping implementation.

---

### 14.8.14 Picker, Harmony and Recolour

**[STU-COL-230] Colour picker.** ONE picker primitive across the whole product. It MUST provide:
entry in HSB, HSL, RGB, RGB hex, CMYK, Lab and Gray; the four value modes of [STU-COL-114]; a
colour wheel, per-model sliders and a spectrum bar; an eyedropper that samples anywhere on the
canvas including rendered images, gradients and composited results, with a configurable sample
radius; out-of-gamut and out-of-web warnings each with a one-click in-gamut or web-safe substitute;
access to swatch groups and spot libraries from within the picker; and promote-to-swatch.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `eyedropper.sample_radius` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | 0 | pixels | 0 |

**[STU-COL-231] Picker warning semantics.** The out-of-gamut warning is computed against the
document's CURRENT PROOF CONDITION when soft-proofing is active and against the document's output
working space otherwise. Which of the two is in force MUST be visible; a warning that does not say
what it is warning about is non-conformant.

**[STU-COL-232] Colour samplers.** Persistent sample points MUST be placeable on a document, each
reading out in a selectable model, and each MUST be readable as structured inspection state for a
headless model.

**[STU-COL-233] Foreground and background colour.** The application carries a foreground and a
background colour, each a full tagged value. They are application state, not document state, and
MUST NOT be persisted into a document.

**[STU-COL-235] Colour harmony.** A DETERMINISTIC harmony primitive generating a variation palette
from a base colour under a named rule. Rules: `complementary`, `split_complementary`, `analogous`,
`monochromatic`, `triad`, `tetrad`, `compound`, `shades`, `tints`, `warm`, `cool`, `vivid`, `muted`.
Output is a set of `StudioSwatch` values. A `limit_to_library` option constrains every generated
colour to the nearest member of a chosen swatch library or colour book. Generation MUST be a pure
function of (base colour, rule, count, library) and MUST be reproducible.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `harmony.variation_count` | 1 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | count | 0 |
| `harmony.variation_step` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | percent | UNKNOWN |

**[STU-COL-236] Recolour.** A DETERMINISTIC recolour primitive remapping every colour in a
selection. Two modes:
- ASSIGN: a table of current-colour to new-colour rows, with merge (fold several current colours
  into one row), exclude (leave a row unmapped), and prominence-weighted extraction that orders rows
  by area coverage.
- EDIT: linked colour handles on a wheel, with a global link toggle so handles move together
  preserving relative hue and saturation.
Plus COLOUR REDUCTION: reduce to N colours or to the members of a chosen library, with independent
preserve toggles for white, black and greys, and a `colorize_method` selection.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `recolor.target_color_count` | 1 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | count | 0 |

**[STU-COL-237] Generative colour is an adapter, never a dependency.** Any AI or generative recolour,
palette suggestion, colour matching or auto-grade capability is an OPTIONAL `StudioModelAdapter`
layered OVER the deterministic primitives of [STU-COL-235] and [STU-COL-236]. It MUST NOT be a hard
dependency, MUST NOT replace a deterministic primitive, and MUST NOT be the only path to any
capability. Every deterministic primitive is first-class and always available offline
([STU-OVR-002]).

---

### 14.8.15 Soft Proof, Gamut and Accessibility

**[STU-COL-240] Soft proof.** Soft-proofing simulates an output condition on screen WITHOUT
converting document values. It is toggleable, its state is document-scoped and inspectable, and it
never alters authority values ([STU-COL-164]).

**[STU-COL-241] Proof condition record.** A proof condition carries SIX fields:
`proof_profile` (the device being simulated), `proof_intent` (document -> proof device),
`display_intent` (proof device -> display), `black_point_compensation` (bool),
`simulate_paper_color` (bool) and `simulate_black_ink` (bool). The TWO intents are separate: the
document-to-device intent and the device-to-display intent are different decisions and a
single-intent proof is wrong. `simulate_paper_color` and `simulate_black_ink` are separate booleans
and are not implied by each other.

**[STU-COL-242] Proof presets.** A proof condition is nameable, savable and selectable. Studio ships
at least: current CMYK working space; current RGB working space; legacy uncompensated RGB; and the
accessibility conditions of [STU-COL-243].

**[STU-COL-243] Colour-vision simulation.** Studio MUST ship colour-vision-deficiency proof
conditions as first-class proof profiles, at minimum protanopia and deuteranopia, and MUST allow an
operator-supplied condition. These are proof conditions, not a filter: they use the same pipeline,
the same toggle and the same inspection state as any other proof.

**[STU-COL-244] Gamut warning.** An overlay marking every pixel or object outside the target gamut,
with a configurable warning colour and warning opacity. It is computed from the clipping report
of [STU-COL-138] and MUST be queryable as structured state, at minimum as an out-of-gamut area
fraction and a bounding region list.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `gamut_warning.opacity` | 0 | 100 | 0 | 100 | 100 | percent | 0 |

**[STU-COL-245] In-gamut substitution.** Where a picked colour is out of gamut, the picker MUST
offer a one-click substitute that is the nearest in-gamut colour under the ACTIVE intent, and MUST
show the delta. The substitution is an explicit operator or model action, never automatic.

**[STU-COL-246] Contrast checking.** Studio MUST expose a contrast-ratio computation between two
tagged colour values, resolved through the colour engine rather than assuming sRGB, so a designer
working in a wide-gamut document gets a correct ratio. It is a query, not an adjustment.

---

### 14.8.16 Model Steerability, GUI and Manual Obligations

**[STU-COL-250] GUI / Argus / UserManual obligation (stated once for 14.8).** Every operator-facing
colour surface enumerated in this module MUST be reachable and drivable through the native operator
UI and the typed model-steerable command surface as two projections of the same primitive (14.16);
MUST be observable and safely steerable headlessly through the Argus visual-debug path with stable
`author_id` targeting under the quiet/headless law, INCLUDING soft-proof state, proof condition,
gamut-warning state and coverage, separations-preview plate state, ink-limit state and the active
display transform as structured inspection state (14.20); and MUST be documented in the
dual-audience UserManual (14.22). Every model-authored colour mutation follows the sandbox ->
validation -> `PromotionGate` lifecycle of [STU-ARC-005].

**[STU-COL-251] Parameter introspection for models.** The typed command surface MUST expose, for
every numeric colour parameter, the full seven-field record of [STU-COL-106], and MUST expose every
enumeration with its complete member list and Studio's own encoding ([STU-COL-135], [STU-COL-198]).
A model MUST receive an explicit `UNKNOWN` rather than a fabricated bound.

**[STU-COL-252] Colour values in model commands.** A model command that carries a colour MUST carry
the full tagged triple ([STU-COL-110]). A command carrying a bare hex string or a bare component
array MUST be rejected at decode, not silently interpreted as sRGB.

**[STU-COL-253] Privacy obligation.** Colour profiles, colour books, LUTs, look libraries, camera
profiles, OCIO configurations, palettes and proof presets are resources subject to the kernel
`ResourceBroker` and the record-level permissions of [STU-SDB-005]. A model lane MUST NOT enumerate
a profile library, a licensed colour book or an operator-supplied LUT set it has not been granted.

**[STU-COL-254] Bulk binary placement.** Profile payloads, LUT payloads, colour-book payloads and
look payloads are BULK BINARY. They live in content-addressed artifact storage with SurrealDB
holding the record and the reference ([STU-SDB-002], [STU-SDB-003]). They MUST NOT be stored as
document fields. For scale: the captured camera-profile corpus alone is 895 MB.

---

### 14.8.17 Validation and Acceptance

**[STU-COL-260] Colour validation descriptors.** The `StudioValidationDescriptor` catalog (14.24)
MUST include the eighteen descriptors below. This table SPAWNS EIGHTEEN microtasks, one per row.

*Derivation: catalogue table, splits per row; yields 18 microtasks, one per validation descriptor. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Validation descriptor | What it checks | Governing clause |
|---|---|---|
| `col.parameter_record_complete` | Every numeric parameter carries all seven fields as separate stored fields | [STU-COL-106] |
| `col.no_invented_bound` | No `UNKNOWN` bound replaced by a number; no `soft_*` mirrored from a `hard_*` | [STU-COL-107] |
| `col.no_clamp_to_observed` | No observed preset range used as a hard bound | [STU-COL-108] |
| `col.tagged_value_law` | No untagged device triple anywhere, including on the wire and in model commands | [STU-COL-110] |
| `col.enumeration_encoding` | Every enumeration carries its full member list and Studio's own encoding | [STU-COL-135] |
| `col.assign_vs_convert` | Assign preserves components; convert preserves appearance; neither is the other | [STU-COL-130] |
| `col.transform_deterministic` | A transform materialised from one cache key is bit-identical across hosts | [STU-COL-143] |
| `col.pipeline_order` | The nine-step grade order and the seven-stage pipeline position are enforced | [STU-COL-160] |
| `col.compositor_purity` | `RenderEngine` resolves no profile, calls no per-pixel colour transform, decides no intent | [STU-COL-163] |
| `col.overlay_purity` | Warnings, previews and separations never composite into authority values | [STU-COL-164] |
| `col.no_platform_cmm` | No platform colour-management module or system colour library in the graph | [STU-COL-140] |
| `col.swatch_reference_resolvable` | Every swatch reference resolves in document, application then system order | [STU-COL-178] |
| `col.display_profile_sourced` | Every display device resolves its profile through a named acquisition mechanism or is explicitly unmanaged; no code path substitutes sRGB for a missing profile | [STU-COL-306] |
| `col.config_provenance_recorded` | Every scene-linear document records the configuration identity, closure hash and declared versions it was authored against | [STU-COL-314] |
| `col.accuracy_within_tolerance` | Every path measured against the reference corpus is within its declared mean and maximum CIEDE2000 tolerance, computed with the declared white point | [STU-COL-323] |
| `col.scope_measurement_point_declared` | Every scope reading carries the measurement point and the scale it was taken in, and a delivery check is never taken on the display-referred branch | [STU-COL-332] |
| `col.materialisation_is_scalar` | The materialisation path carries no runtime vector dispatch, and no application-path code is reachable from it | [STU-COL-351] |
| `col.interpolation_is_keyed` | Every materialised transform carrying a lookup table declares its interpolation rule, and that rule is part of the cache key rather than inferred from the sampler | [STU-COL-353] |

**[STU-COL-261] No-platform-CMM tripwire.** The build MUST fail if any crate in the `studio-engine`
or `handshake_core` dependency graph links a platform colour-management module or a
system-installed colour library. The tripwire runs alongside the SQLite tripwire of [STU-OVR-003]
and the text-engine tripwire of [STU-TYP-231].

**[STU-COL-262] Mandatory acceptance cases.** Acceptance MUST include the twenty-six cases below.
This table SPAWNS TWENTY-SIX microtasks, one per row.

*Derivation: catalogue table, splits per row; yields 26 microtasks, one per acceptance case. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Acceptance case | What it proves | Governing clause |
|---|---|---|
| `acc.model_round_trip` | A value round-trips through every model pair with bounded error | [STU-COL-111] |
| `acc.assign_vs_convert` | Assign leaves components unchanged; convert changes them and preserves appearance within a declared tolerance | [STU-COL-131] |
| `acc.four_intents_differ` | All four intents give four different results for an out-of-gamut source, and BPC changes the result independently of intent | [STU-COL-135] |
| `acc.device_link_inert` | A device-link profile reports intent and BPC as inert rather than applying them | [STU-COL-134] |
| `acc.cross_host_bit_identical` | The same MATERIALISED transform, built from one cache key, is byte-identical on two hosts | [STU-COL-143] |
| `acc.lut_domain_honoured` | A LUT with a non-0-1 domain applies correctly, and applying it as if the domain were 0-1 gives a detectably different result | [STU-COL-158] |
| `acc.stack_order_enforced` | Reordering the grade stack changes the result and an out-of-order stack is refused | [STU-COL-160] |
| `acc.legacy_grade_keys` | A five-wheel grade imported from a legacy-key source recovers all fifteen values, including the four stored under split-toning keys | [STU-COL-223] |
| `acc.parameter_identity` | `secondary.temperature` accepts 250 and `temperature` rejects it | [STU-COL-227] |
| `acc.gradient_midpoint_bound` | A gradient midpoint of 5.0 is rejected and 13.0 is accepted | [STU-COL-192] |
| `acc.global_swatch_propagation` | Editing a global swatch updates every referencing site in one history entry without disturbing a per-object tint | [STU-COL-172] |
| `acc.absent_book_renders_same` | A document referencing an absent colour book renders identically to one where the book is present | [STU-COL-186] |
| `acc.spot_separates` | A spot swatch separates onto its own plate regardless of its alternate space | [STU-COL-173] |
| `acc.ink_limit_over_400` | The ink-limit view reports coverage above 400 percent on a document with spot plates | [STU-COL-212] |
| `acc.proof_is_non_destructive` | Soft proof changes not a single stored component value | [STU-COL-240] |
| `acc.gamut_coverage_readable` | Gamut-warning coverage is readable as a number by a headless model | [STU-COL-244] |
| `acc.blending_space_effective` | Changing the blending space alters a composite result and the change is reported | [STU-COL-162] |
| `acc.untagged_command_rejected` | A colour command carrying an untagged component array is rejected at decode | [STU-COL-252] |
| `acc.delta_e_reference_pairs` | The colour-difference implementation reproduces the published reference value for all thirty-four conformance pairs, including the pairs that cross the hue-difference branch | [STU-COL-322] |
| `acc.display_profile_change_detected` | Changing a monitor's associated profile outside Studio causes the display transform to be re-materialised without a restart, and the old transform is evicted from the cache | [STU-COL-303] |
| `acc.unmanaged_display_is_visible` | A monitor with no resolvable profile reports the unmanaged state as readable inspection state and is not rendered as if it were sRGB | [STU-COL-306] |
| `acc.window_span_reported` | A window straddling two monitors with different profiles binds to the majority-area monitor and reports the spanning condition rather than presenting the second part as verified | [STU-COL-305] |
| `acc.config_closure_mismatch_reported` | A document whose recorded configuration closure differs from the resolved one reports the mismatch and names the differing side instead of re-rendering the grade silently | [STU-COL-315] |
| `acc.legal_range_area_threshold` | A signal outside the preferred range over more than the threshold area reports an out-of-gamut fraction, one below the threshold does not raise, and an out-of-range component is reported separately from an invalid combination | [STU-COL-335] |
| `acc.materialisation_dispatch_invariant` | The same cache key materialises byte-identically on one architecture with a wider instruction set available and with it disabled, and again on a second architecture | [STU-COL-351] |
| `acc.interpolation_rules_differ_and_are_keyed` | The same profiles, grid and domain materialised under the two interpolation rules produce two DIFFERENT artefacts, two different cache entries, and a measurably different image; and a path that substitutes a rule reports the substitution rather than hiding it | [STU-COL-353] |

**[STU-COL-263] Round-trip obligation.** Every field named in this module MUST survive a
save/load/save cycle byte-identically, and MUST survive an import/export round trip through the
interchange formats of 14.13 with any loss explicitly reported rather than silent. A colour value
that loses its profile reference in a round trip is a failure, not a degradation.

---

### 14.8.18 Scope Edges

**[STU-COL-265] Owned here.** The colour value model; colour models, document modes and mode
conversion; bit depth and HDR; ICC parsing, working spaces, policies, assign, convert, embed;
rendering intents and black point compensation; the colour engine and transform materialisation;
OCIO and scene-linear; LUTs and their application order; the pipeline position of colour management
relative to the compositor; swatches, groups, palettes and interchange; branded colour books; spot
inks, ink manager, separations, overprint, trapping, flattening and appearance of black; gradients
and patterns; blend-mode encoding; the colour picker; harmony and recolour; camera colour rendering;
the grading model; soft proof, gamut warning and colour-vision simulation; DISPLAY device
characterisation, meaning the consumption of a display profile, monitor identity, reaction to display
change and the unmanaged state, together with the optional out-of-process calibration adapter lane;
the ACES-named scene-linear default with its role bindings, display targets and configuration
provenance record; the colour-accuracy metric, its per-path tolerances and its reference patch
corpus; the video scope measurement contract with legal-range and gamut-error checking; and the split
between transform materialisation and transform application with the determinism obligation each
carries.

**[STU-COL-266] Not owned here (referenced).** Channel operations, alpha and spot channels,
apply-image and calculations (14.4); tonal and colour ADJUSTMENT LAYERS as layer objects (14.4) -
this module owns their colour contracts and parameter blocks, 14.4 owns their layer behaviour;
effect and filter stacking (14.9); text colour application (14.7); page and prepress OUTPUT surfaces
(14.6); export recipes and file formats (14.13); raw decode (14.12); the video and compositing
colour surfaces beyond the grading model stated here.

**[STU-COL-267] Colour capability provenance scale.** The capability registry recorded 7,478 rows in
the `color` domain, the largest single domain, across eight ingested applications (4,740 presets,
1,588 options, 458 capabilities, 357 commands, 182 dialogs, 100 tools, 48 panels, 5 menu entries).
Those rows are EVIDENCE of surface breadth; this module is the contract. Registry rows merged on
name and the merge key was defective at capture time, so registry counts MUST NOT be cited as a
measure of shared capability across applications.

**[STU-COL-268] Preset names are evidence, contracts are normative.** The captured colour corpus
contains 4,740 preset rows. A preset's NAME is evidence; the normative statement is the CONTRACT
around it - that the swatch system carries these kinds with these fields, that a colour book has
this record shape, that a LUT declares this domain. This module therefore enumerates contracts and
scale, not preset names. Shipping a named preset library is a content decision, not a specification
decision.

---

### 14.8.19 Microtask Derivation

**[STU-COL-270] Derivation rule (NORMATIVE).** The colour microtask set is derived from this module
mechanically, not editorially. A derivation tool extracts exactly these unit kinds:

**Rule 0 -- derivation markers are authoritative.** Every table in this sub-section carries an
italic `*Derivation: ...*` marker sentence directly above it stating how many microtasks that table
yields. The marker is NORMATIVE. A tool that classifies a table differently from its marker has
diverged from this sub-section and MUST be corrected to the marker, not the reverse. The six marker
forms are: parameter table taken whole (1); enumeration table taken whole (1); preset or command
table taken whole (1); catalogue table splitting per row (N, with the subject named); contract table
carried into the clause's own microtask (0); and reading aid inside a non-yielding clause (0). A
catalogue marker states its own count, and that count MUST equal the table's row count unless the
marker says otherwise and gives the reason. The summary index of [STU-COL-274] is COMPUTED FROM these
markers and is a projection of them, never a second source: where the two disagree, the markers win
and the index is regenerated.

**Clause arithmetic (NORMATIVE, and stated so a divergence is diagnosable).** 14.8 defines
185 clause anchors, every one of them as a paragraph opening with its bold anchor at line start,
none inside a table cell, none inside a blockquote and none inside a fenced block. Subtracting the
18 anchors of the non-yielding set below leaves 167 yielding clauses, and 167 is exactly what
the clause rows of the ledger in [STU-COL-272] sum to. A tool that reaches a different
yielding-clause count for 14.8 is either not seeing all 185 definitions or honouring more than
18 exclusions, and this arithmetic says which. ONE reading is legitimately different and is not a
divergence: exactly one of the 185, the letter-suffixed `STU-COL-104A`, is also one of the 18
exclusions, so a tool whose anchor pattern does not admit an upper-case letter suffix sees 184 minus
17 and reaches the same 167. Both readings are correct and the fourth-pass note in [STU-COL-275]
records why. Note also that the non-yielding set names only anchors this module defines: an anchor
from the superseded v02.205 module cannot be excluded here because it was never counted here in the
first place.

**Rule 0a -- anchors inside table cells are never definitions here.** Every one of the 185 clauses
in 14.8 is defined as a PARAGRAPH opening with its bold anchor at line start; not one is defined
inside a table cell. 167 distinct anchors appear in cells of this sub-section, and they fall into
exactly two categories, neither of which is a definition. 132 are cross-references to clauses
defined as paragraphs elsewhere in 14.8. The remaining 35, spanning STU-COL-001 to STU-COL-036, are
anchors of the SUPERSEDED v02.205 module whose disposition [STU-COL-102] records; the clauses they name
are withdrawn, retained or refined there, not defined here. Every table carrying an in-cell anchor
says so in its own marker. A tool that treats an in-cell anchor as a clause definition here produces
a second unit for a clause rule A has already counted, or a unit for a clause this module does not
define at all; both are double counts and neither is work. This rule constrains only 14.8; other
modules do define clause families in table cells, and this rule says nothing about them.

**Absence token.** This module writes the literal token `UNKNOWN` into any parameter cell whose value
the source did not declare. `UNKNOWN` means the capture carries no value for that field. It is not a
bound, it is not zero, and it is not a licence to substitute one. Sibling modules may declare a
different token for the same meaning -- the effects module uses `--` per [STU-FX-131a] -- so a reader
or a tool MUST take the absence token from the module it is reading and MUST NOT assume one shared
token across section 14.

1. **Clause.** One microtask per clause anchor, EXCEPT the declared non-yielding set below.
   Derivation is NOT gated on the clause containing MUST or SHALL: a clause may state a stored
   contract, an enumeration or a mathematical model in the indicative mood and still be a unit of
   work.
2. **Parameter table.** One microtask per table whose header carries at least four of `hard_min`,
   `hard_max`, `soft_min`, `soft_max`, `default`, `unit`, `precision`. Every row of that table is an
   acceptance criterion of that one microtask.
3. **Enumeration.** One microtask per enumeration, with its members as acceptance criteria.
4. **Catalogue row.** One microtask per row of a catalogue table whose first column names a
   separately implementable subject; [STU-COL-274] declares which tables those are.
5. **Validation descriptor.** One microtask per descriptor row of [STU-COL-260].
6. **Acceptance case.** One microtask per row of [STU-COL-262].

**Declared non-yielding set (NORMATIVE, by anchor).** These eighteen clauses yield NOTHING. They
are authority bookkeeping, scope statements, pure cross-references, or obligations that attach to
every other microtask rather than forming one:
`STU-COL-100`, `STU-COL-101`, `STU-COL-102`, `STU-COL-103`, `STU-COL-104`, `STU-COL-104A`,
`STU-COL-250`, `STU-COL-265`, `STU-COL-266`, `STU-COL-267`, `STU-COL-268`, `STU-COL-270`,
`STU-COL-271`, `STU-COL-272`, `STU-COL-273`, `STU-COL-274`, `STU-COL-275`, `STU-COL-340`.
Every other clause anchor in this module yields exactly one microtask. A tool MUST use this list
rather than inferring exclusions from prose, because inference produced the two-clause divergence
that is recorded in [STU-COL-275].

**[STU-COL-271] Microtask content obligation.** A microtask derived under [STU-COL-270] MUST carry
into its own body: the clause anchor; the full seven-field parameter record of every parameter it
touches, with `UNKNOWN` preserved and hard and soft bounds kept separate; the complete member list
of every enumeration it touches with Studio's own encoding; and the determinism obligation stated
in [STU-COL-143] where it touches a transform. A microtask that says "implement the basic correction
controls" without the fifteen rows of [STU-COL-220] and their split bounds does not satisfy this
clause.

**[STU-COL-272] Yields index (NORMATIVE LEDGER).** One row per unit group. The last column is the
microtask count that group yields under [STU-COL-270]. The TOTAL row is the module's declared yields
total and is the figure a reconciler compares against a derivation tool's output.

*Derivation: reading aid inside a non-yielding clause; yields no microtask. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Unit group | Source | Unit kind | Yields |
|---|---|---|---|
| Parameter contract and units | [STU-COL-105]-[STU-COL-109] | clause | 5 |
| Colour value model | [STU-COL-110]-[STU-COL-117] | clause | 8 |
| Modes, bit depth, HDR | [STU-COL-118]-[STU-COL-124] | clause | 7 |
| ICC, working spaces, intents | [STU-COL-125]-[STU-COL-139] | clause | 15 |
| Colour engine | [STU-COL-140]-[STU-COL-144] | clause | 5 |
| OCIO and scene-linear | [STU-COL-150]-[STU-COL-157] | clause | 8 |
| LUTs | [STU-COL-158]-[STU-COL-160] | clause | 3 |
| Pipeline position | [STU-COL-161]-[STU-COL-165] | clause | 5 |
| Swatches | [STU-COL-170]-[STU-COL-183] | clause | 14 |
| Branded colour books | [STU-COL-184]-[STU-COL-188] | clause | 5 |
| Gradients and patterns | [STU-COL-190]-[STU-COL-198] | clause | 9 |
| Inks, separations, prepress | [STU-COL-205]-[STU-COL-215] | clause | 11 |
| Camera colour and grading | [STU-COL-218]-[STU-COL-229] | clause | 12 |
| Picker, harmony, recolour | [STU-COL-230]-[STU-COL-237] | clause | 7 |
| Proof, gamut, accessibility | [STU-COL-240]-[STU-COL-246] | clause | 7 |
| Model steerability | [STU-COL-251]-[STU-COL-254] | clause | 4 |
| Validation clauses | [STU-COL-260]-[STU-COL-263] | clause | 4 |
| Display device characterisation | [STU-COL-300]-[STU-COL-308] | clause | 9 |
| ACES scene-linear default | [STU-COL-310]-[STU-COL-317] | clause | 8 |
| Colour accuracy metric and patch sets | [STU-COL-320]-[STU-COL-326] | clause | 7 |
| Video scopes and legal range | [STU-COL-330]-[STU-COL-337] | clause | 8 |
| Materialisation and application paths | [STU-COL-350]-[STU-COL-355] | clause | 6 |
| Numeric parameter tables (36 tables, 144 rows) | throughout 14.8.2-14.8.25 | parameter table | 36 |
| Rendering intents | [STU-COL-135] | enumeration | 1 |
| Swatch kinds | [STU-COL-171] | catalogue row | 7 |
| Gradient geometries | [STU-COL-190] | catalogue row | 7 |
| Colour adjustment catalogue | [STU-COL-229] | catalogue row | 23 |
| Validation descriptors | [STU-COL-260] | validator | 18 |
| Mandatory acceptance cases | [STU-COL-262] | acceptance case | 26 |
| Display profile acquisition mechanisms | [STU-COL-301] | catalogue row | 4 |
| Display change kinds | [STU-COL-303] | enumeration | 1 |
| ACES encodings | [STU-COL-311] | catalogue row | 4 |
| Display targets and their views | [STU-COL-313] | catalogue row | 7 |
| Reference patch sets | [STU-COL-324] | catalogue row | 6 |
| Video scopes | [STU-COL-331] | catalogue row | 6 |
| Scope measurement points | [STU-COL-332] | enumeration | 1 |
| Scope scales | [STU-COL-333] | enumeration | 1 |
| Non-bakeable materialisation cases | [STU-COL-354] | catalogue row | 5 |
| Declared non-yielding clauses | [STU-COL-270] non-yielding set | excluded | 0 |
| **TOTAL** | **14.8 whole** | **all kinds** | **320** |

**[STU-COL-273] Anchor binding.** A microtask derived from this module cites the clause anchor
directly. A microtask staged before this module landed carries
`spec_anchor_status = "PROVISIONAL"`; binding it to an anchor from this module clears that status.
A microtask that cannot cite an anchor in this module is out of scope for the colour domain and
MUST be re-derived or retired, not activated.

**[STU-COL-274] Table spawn declarations (NORMATIVE).** A derivation tool cannot tell from a table's
shape alone whether it is one unit or many. This clause declares it for every non-parameter table in
14.8, so no tool has to guess. The table below is itself DECLARED NON-SPAWNING.

*Derivation: reading aid inside a non-yielding clause; yields no microtask. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Table (first column) | Clause | Rows | Marker classification | Yields |
|---|---|---|---|---|
| All numeric parameter tables | throughout | 144 | parameter table, taken whole (each) | 36 |
| Anchor | [STU-COL-102] | 34 | reading aid in a non-yielding clause | 0 |
| Field | [STU-COL-106] | 7 | contract table carried into its clause | 0 |
| Model | [STU-COL-121] | 8 | contract table carried into its clause | 0 |
| Studio `intent` | [STU-COL-135] | 4 | enumeration table, taken whole | 1 |
| Stage | [STU-COL-161] | 9 | contract table carried into its clause | 0 |
| `swatch_kind` | [STU-COL-171] | 7 | catalogue, splits per row (one per swatch kind) | 7 |
| Gradient geometry | [STU-COL-190] | 7 | catalogue, splits per row (one per gradient geometry) | 7 |
| Adjustment | [STU-COL-229] | 23 | catalogue, splits per row (one per colour adjustment) | 23 |
| Validation descriptor | [STU-COL-260] | 18 | catalogue, splits per row (one per validation descriptor) | 18 |
| Acceptance case | [STU-COL-262] | 26 | catalogue, splits per row (one per acceptance case) | 26 |
| Unit group | [STU-COL-272] | 40 | reading aid in a non-yielding clause | 0 |
| Table (first column) | [STU-COL-274] | 34 | reading aid in a non-yielding clause | 0 |
| Missed unit group | [STU-COL-275] | 6 | reading aid in a non-yielding clause | 0 |
| Acquisition mechanism | [STU-COL-301] | 4 | catalogue, splits per row (one per acquisition mechanism) | 4 |
| Display device field | [STU-COL-302] | 7 | contract table carried into its clause | 0 |
| `display_change_kind` | [STU-COL-303] | 6 | enumeration table, taken whole | 1 |
| Adapter boundary field | [STU-COL-308] | 6 | contract table carried into its clause | 0 |
| Colour space | [STU-COL-311] | 4 | catalogue, splits per row (one per ACES encoding) | 4 |
| Role | [STU-COL-312] | 9 | contract table carried into its clause | 0 |
| Display target | [STU-COL-313] | 7 | catalogue, splits per row (one per display target) | 7 |
| Configuration provenance field | [STU-COL-314] | 8 | contract table carried into its clause | 0 |
| Implementation trap | [STU-COL-322] | 4 | contract table carried into its clause | 0 |
| Reference patch set | [STU-COL-324] | 6 | catalogue, splits per row (one per reference patch set) | 6 |
| Scope | [STU-COL-331] | 6 | catalogue, splits per row (one per scope) | 6 |
| `scope_measurement_point` | [STU-COL-332] | 5 | enumeration table, taken whole | 1 |
| `scope_scale` | [STU-COL-333] | 5 | enumeration table, taken whole | 1 |
| Quantisation level | [STU-COL-333] | 6 | contract table carried into its clause | 0 |
| Term | [STU-COL-334] | 4 | contract table carried into its clause | 0 |
| Open item | [STU-COL-340] | 10 | reading aid in a non-yielding clause | 0 |
| Path | [STU-COL-350] | 2 | contract table carried into its clause | 0 |
| Obligation | [STU-COL-353] | 5 | contract table carried into its clause | 0 |
| Non-bakeable case | [STU-COL-354] | 5 | catalogue, splits per row (one per non-bakeable case) | 5 |
| **TOTAL TABLE UNITS** | **all 68 tables** | **476** | **computed from the markers above** | **153** |

**[STU-COL-275] Reconciliation of record.** **First pass (ledger).** A derivation tool run against this module before the
ledger existed reached **185**. The declared total is **225**. The difference is **40** and
decomposes exactly, with no unexplained residual:

*Derivation: reading aid inside a non-yielding clause; yields no microtask. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Missed unit group | Count | Why the tool could not reach it | Fix applied |
|---|---|---|---|
| Mandatory acceptance cases | 18 | The cases were a numbered prose list, which the tool does not read as units | [STU-COL-262] is now an eighteen-row table |
| Validation descriptors | 12 | The descriptors were a semicolon-separated prose sentence, so none was individually addressable | [STU-COL-260] is now a twelve-row table |
| Gradient geometries | 7 | Its first column read `Geometry`, which is not in the tool's subject vocabulary, so the table was skipped | First column renamed to `Gradient geometry`, and [STU-COL-274] declares the spawn count |
| Rendering intents enumeration | 1 | Its first column reads `Studio intent`, not recognised as an enumeration subject | [STU-COL-274] declares it |
| Yielding-clause exclusion set | 2 | The tool inferred two extra exclusions from prose because no explicit non-yielding list existed | [STU-COL-270] now declares the non-yielding set by anchor, so nothing is inferred |
| Residual | 0 | The tool's 185 equals 127 inferred yielding clauses plus 28 parameter tables plus 23 adjustment rows plus 7 swatch-kind rows | none needed |

The tool was CORRECT on parameter tables, on the adjustment catalogue and on the swatch-kind
catalogue, and this ledger reproduces those three rows unchanged. It was WRONG only where the module
expressed real work in prose rather than in a table it could read, and on two clause exclusions it
had to guess. Both are defects in the module text and have been repaired here rather than argued
away. The spec remains authority: a tool that now produces a total other than 225 has diverged from
this sub-section and MUST be reconciled against it, not the reverse.

**Second pass (markers).** A later tool run reached **222** against the same declared **225**, a
residual of **3**. The cause was mechanical, not a disagreement about the work: the spawn counts
were declared CENTRALLY in [STU-COL-274], and the derivation tool does not read a central
declaration table -- it reads a marker attached to each table. Every table in 14.8 now carries an
italic `*Derivation: ...*` marker directly above it under rule 0 of [STU-COL-270], the marker is
normative over any tool heuristic, and [STU-COL-274] is regenerated FROM those markers rather than
maintained beside them, so the index and the markers cannot drift apart. Recomputing from the
markers alone gives 129 yielding clauses plus 96 table units = **225**, which is the declared total.
The residual is **0**.

I did not reverse-engineer which individual tables the 222 run split differently, because the marker
convention makes that determination moot: a tool no longer classifies these tables, it reads their
declarations. Each catalogue marker states its own count and names its subject, so a tool auditing a
declared count against the rows it actually produces will surface any future mismatch instead of
absorbing it silently. That audit currently reports no mismatch on any of the thirteen non-parameter
tables in this module.

**Third pass (anchor rows), and one open item.** A later tool run reported 252 units for 14.8
against the declared 225. The five substantive unit kinds summed to 222 -- 122 clause, 4 validator,
28 parameter table, 1 enumeration, 67 catalogue row -- and 27 further units were `anchor_row`.

The premise was verified rather than assumed. 126 distinct anchors appear in cells of 14.8. 91 are
cross-references to clauses defined as paragraphs in this sub-section, and 35 are anchors of the
superseded v02.205 module recorded in the disposition table of [STU-COL-102]. NOT ONE is defined
only in a cell. All 8 tables carrying an in-cell anchor now say so in their markers, and rule 0a
states it for the sub-section, which takes `anchor_row` to 0 and removes 27 of the 30-unit excess.

**The remaining 3 is an OPEN ITEM against the derivation tool, not a defect this module can find.**
The tool's clause plus validator count for 14.8 is 126; this sub-section defines 129 yielding
clauses. Four checks were run and all four came back clean, so the ledger is not over-claiming:
every ledger clause row was audited against the anchors actually defined and not in the non-yielding
set, and the rows sum to exactly 129; every catalogue marker was audited for declared count against
both row count and DISTINCT first-column subjects, and all five agree; all 146 clause definitions sit
at line start, none inside a table cell, a blockquote or a fenced block, and 14.8 is the only one of
the two modules that contains blockquotes and a fenced block at all; and the non-yielding set parses
to 17 anchors, all 17 of which this module defines. The clause-arithmetic statement in [STU-COL-270]
now publishes 146 minus 17 equals 129 so the divergence is diagnosable from the module rather than by
inspection: a tool reaching 126 is either not seeing three of the 146 definitions or honouring three
exclusions the module does not declare, and those are the only two possibilities. Until that is
resolved on the tool side, the declared total stands at 225 on the evidence above.

**Fourth pass (the letter-suffixed anchor), and its closure.** The counts recorded in the three
passes above describe 14.8 AS IT STOOD AT THOSE PASSES, before sub-sections 14.8.20 through 14.8.25
were added; they are kept as the record of how each divergence was found and closed, and the current
figures are the ones in [STU-COL-270] and [STU-COL-272]. The third pass left a residual of 3
unattributed. It is now attributed, and it is arithmetic rather than a missing clause. This module
defines one anchor with an UPPER-CASE letter suffix, `STU-COL-104A`, and declares that same anchor
non-yielding. A tool whose anchor pattern admits only a lower-case suffix does not see that
definition at all, so it counts one fewer definition AND one fewer exclusion than this sub-section
does. Both counts move together and the yielding total is unchanged: this sub-section's human count
is 185 definitions minus 18 exclusions, and such a tool's count is 184 minus 17, and both equal 167.
The two numbers are therefore BOTH correct readings of the same text and neither is a defect to
repair; a tool reaching a yielding-clause count other than 167 has a real divergence, and a tool
whose definition and exclusion counts are each exactly one lower than the human count has this one.
The letter-suffixed form stays because [STU-COL-104A] declares it legal and other sub-sections of
section 14 carry the same form.

**Fifth pass (four gaps closed, and two engine defects).** Sub-sections 14.8.20 through 14.8.23
close four verified holes: device characterisation, which was absent entirely because all fourteen
calibration mentions in 14.8 were CAMERA calibration; an ACES-named scene-linear default, which the
OCIO machinery of 14.8.6 required but never named; a testable colour-accuracy metric, which the
module demanded in more than thirty places while stating no number anywhere; and video scopes with
legal-range checking, which did not exist in section 14 at all because every waveform in the
timeline sub-section is an AUDIO waveform. Sub-section 14.8.25 closes two defects found in the
engine clauses while researching an unrelated question: [STU-COL-142] declared a cache key without
the interpolation rule, which let two genuinely different transforms share one cache entry,
and [STU-COL-143] required bit-identity in terms broad enough to be unsatisfiable by any implementation
that dispatches on processor features at runtime. Both are now repaired in the clauses themselves
and in [STU-COL-350]-[STU-COL-355]. The declared total moves from 225 to 320 and the ledger
of [STU-COL-272] carries the decomposition; nothing was renumbered and no clause was removed.

---

### 14.8.20 Display Device Characterisation

**[STU-COL-300] Studio consumes display characterisation; it does not measure it (SCOPE POSTURE,
CLOSING A NAMED GAP).** Fourteen places in this module speak of calibration and every one of them
is CAMERA calibration - the illuminant-referenced matrices of [STU-COL-218] and the seven camera
calibration controls of [STU-COL-224]. Not one is DISPLAY characterisation. That is a real
hole: [STU-COL-154] lets a document select `icc_display`, and the display transform of [STU-COL-161]
stage 6a converts working space to display, but nothing said where the display's profile comes
from, so the profile at the end of the pipeline was assumed rather than sourced. This sub-section
sources it. Studio READS a display profile that something else produced and REACTS when it changes.
Studio does NOT drive a colorimeter or a spectrophotometer, does NOT implement an instrument
protocol, and does NOT ship an instrument driver; measuring a display is an optional out-of-process
adapter lane ([STU-COL-308]) on the same posture as every other adapter in this module
([STU-COL-185], [STU-COL-237]). A display profile is an ordinary `StudioColorProfile` of
`profile_class = display` ([STU-COL-125]) and requires no new profile primitive.

**[STU-COL-301] Per-monitor profile acquisition from the host.** The host operating system, not
Studio, holds the association between a monitor and its ICC profile, and each platform exposes that
association through a different named mechanism. Studio MUST implement acquisition on each supported
platform through the mechanism named below, MUST acquire PER MONITOR rather than once per
application, and MUST record which mechanism produced the profile it is using so a wrong colour on
one host is diagnosable without guessing. Where a platform offers both a per-device and a
per-drawing-surface call, the per-device call is authoritative, because a per-surface call answers
for whichever monitor the surface currently sits on and therefore cannot characterise a second
monitor at all. Mechanisms are named here as the published platform interfaces they are, which is
provenance under [STU-SECTION-003]; no Studio type, command or panel takes its name from one.

*Derivation: catalogue table, splits per row; yields 4 microtasks, one per acquisition mechanism.*

| Acquisition mechanism | Platform | What Studio calls or reads | Failure and absence behaviour |
|---|---|---|---|
| `display_profile.windows_wcs` | Windows | The Windows Color System per-device association, keyed on the monitor rather than on a device context, resolved through the per-user scope first and the machine-wide scope as fallback. TWO generations of that per-device call exist and they are NOT equivalent: the older one is keyed on the monitor's device name and is documented as not covering advanced-colour profiles, and the newer one is keyed on the graphics adapter's locally unique identifier plus a source index and does cover them. Studio MUST prefer the newer call where the host provides it, because on an HDR display the older call answers with the wrong profile rather than with no profile. | An unassociated monitor yields no profile and enters the unmanaged state of [STU-COL-306]; it is never silently treated as sRGB |
| `display_profile.macos_colorsync` | macOS | The ColorSync display-device association for the specific screen, or equivalently the screen's own colour space, read PER SCREEN and never as one application-wide default. The raw ICC bytes are extracted from the returned colour space so the result is an ordinary `StudioColorProfile` under [STU-COL-125]. | A screen reporting no profile enters the unmanaged state of [STU-COL-306] |
| `display_profile.x11_atom_and_colord` | Linux under X11 | The ICC Profiles in X root-window property, whose name carries an output index suffix for every output after the first, and, where the display extension exposes per-output objects, the same property carried on the OUTPUT object, which is the authoritative source for a multi-monitor session because the root-window property alone cannot address a second monitor. The colour-management D-Bus service's device database is consulted for the device-to-profile mapping when neither property is present. | Neither source present is the unmanaged state of [STU-COL-306], not an error dialog |
| `display_profile.wayland_protocol` | Linux under Wayland | The compositor colour-management protocol, whose presence MUST be probed at runtime and never assumed, with the colour-management D-Bus service's device database as the fallback source | A compositor that does not advertise the protocol falls back, and if the fallback is also absent the state is unmanaged |

The Wayland row is the one an implementer will find least stable, and this module does not pretend
otherwise: at the time this module was written that protocol was published in the staging area of the
platform's protocol collection rather than in the stable area, and had been revised more than once
while there ([STU-COL-340] records this). The acquisition path is therefore stated as a
probe-then-fall-back contract rather than as a fixed interface version. What Studio MUST NOT do is
treat a missing protocol as a licence to assume sRGB.

**[STU-COL-302] `StudioDisplayDevice` and stable monitor identity.** Schema id
`hsk.studio.display_device@1`. A display device record binds a monitor to the profile in force on
it, and its identity MUST survive disconnection, reconnection, a reboot and a change of connector,
because a display-transform selection that loses its monitor on every unplug is not a colour-managed
pipeline. Identity is derived from the monitor's own descriptor data, not from an enumeration index
and not from a connector name; both of those are reassigned by the host and by a docking or
stream-splitting topology.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Meaning |
|---|---|
| `display_device_id` | The stable identity: a hash over the monitor descriptor's manufacturer id, product code, and serial-number and monitor-name descriptor blocks. Stable across reconnection because it is derived from the monitor, not from the host's enumeration. |
| `descriptor_bytes_hash` | Content hash of the whole raw monitor descriptor as read, so two units that collide on the fields above are still distinguishable by their full descriptor. |
| `identity_confidence` | `unique` when the descriptor carries a serial-number or monitor-name descriptor block; `ambiguous` when it does not, or when two connected monitors resolve to the same `display_device_id`; `absent` when no descriptor was readable at all. |
| `host_enumeration_key` | The platform's own current handle or device name. VOLATILE by contract, stored for diagnosis, and MUST NOT be used as identity. |
| `active_profile_ref` | The `content_hash` of the `StudioColorProfile` in force ([STU-COL-125]), or the unmanaged marker of [STU-COL-306]. |
| `profile_source` | Which mechanism of [STU-COL-301] produced it, or `operator_supplied` under [STU-COL-304]. |
| `acquired_at` | When the association was last read, so a stale association is visible rather than assumed current. |

The identity fields are the monitor descriptor's own: the manufacturer identifier and product code
in the descriptor's fixed header, the optional binary serial number beside them, and the free-text
serial-number and monitor-name descriptor blocks. Three failure modes are known and MUST be handled
rather than assumed away, and the first two are documented failures of the field service that solves
this problem on one platform, not hypotheticals. Two units of the same model can ship with identical
manufacturer, model and serial fields, so a device id composed of those three fields collides; the
field service's own issue history records the second monitor's profile registration being silently
dropped on exactly that collision, EVEN THOUGH its own descriptor checksum could have separated the
two. `descriptor_bytes_hash` exists so Studio has that separation, and `identity_confidence` exists so
an unseparable pair is surfaced to the operator instead of silently binding a grade to the wrong
panel. Some connections present no descriptor at all, which is `absent`. A monitor reached through a
dock or a multi-stream topology may change its host enumeration key without changing its descriptor,
which is exactly the case `host_enumeration_key` is kept volatile for; that last case is reported in
the field but this module found no primary specification for it, so it is handled defensively rather
than cited as established behaviour ([STU-COL-340]).

**[STU-COL-303] Reacting to display change, and the notification that does not exist.** The display
association is LIVE. Studio MUST subscribe to the host's display-configuration notifications and MUST
re-resolve the active profile, the materialised display transform of [STU-COL-142] and its cache key
when one arrives. A pipeline that reads the display profile once at startup shows the operator a
stale transform for the rest of the session and is non-conformant. The design constraint an
implementer will hit is that NO platform surveyed for this module publishes a notification meaning
"a monitor's associated profile changed": the platforms publish resolution, arrangement, scale and
add/remove notifications, and an operator changing the profile in a host control panel produces
either a general configuration notification or nothing at all. `profile_association_changed` is
therefore DETECTED, not received - Studio re-reads the association under [STU-COL-301] and compares
the resulting profile `content_hash` with the stored `active_profile_ref`, on every configuration
notification and on window activation. Waiting for a profile-changed event is waiting for an event
that is not sent.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| `display_change_kind` | What triggers it | What Studio MUST re-derive |
|---|---|---|
| `profile_association_changed` | A re-read of the association returns a profile whose `content_hash` differs from the stored `active_profile_ref`. Detected by comparison, never received as an event. | The `active_profile_ref`, the display transform, and every cached transform keyed on the old profile hash |
| `display_added` | A monitor is connected | A `StudioDisplayDevice` record and its profile, before the first frame is presented on it |
| `display_removed` | A monitor is disconnected | The binding of every window that was on it, and the unmanaged state where no replacement resolves |
| `display_reconfigured` | Resolution, arrangement, refresh or the primary-monitor selection changes | The monitor-to-window mapping of [STU-COL-305] |
| `scale_factor_changed` | The per-monitor scale factor changes for a window | The monitor-to-window mapping only; a scale change alone never changes colour and MUST NOT invalidate a colour transform cache |
| `session_or_compositor_changed` | The graphics session, compositor or colour-management service restarts | Every field of every `StudioDisplayDevice`, because the whole association layer was replaced |

`scale_factor_changed` is listed precisely so it is NOT treated as a colour event. A resolution or
scale change that flushes the colour transform cache costs a full re-materialisation for nothing,
and an implementer who folds the two together will not notice, because the output is still correct.

**[STU-COL-304] Operator-supplied display profile.** An operator MUST be able to supply an ICC
display profile for a specific `display_device_id` and have it OVERRIDE what [STU-COL-301] read from
the host. The override is stored against the display device, survives restart, is separately
clearable, and is reported in the inspection state of [STU-COL-307] as an override rather than as a
host reading, because a colour complaint whose cause is a forgotten override is otherwise
undiagnosable. A supplied profile is validated as a display-class profile on load; a profile whose
`profile_class` is not `display` MUST be refused with a reason, not accepted and quietly misused.

**[STU-COL-305] Multiple monitors and a window that spans two.** Studio MUST support a different
profile per monitor simultaneously. A window is bound to exactly ONE display device at a time for
colour purposes, and that binding is the monitor holding the largest area of the window. When a
window spans two monitors with different profiles the parts on the second monitor are colour-managed
for the FIRST, which is a real and unavoidable error under a single-transform presentation path;
Studio MUST therefore report the spanning condition in the inspection state of [STU-COL-307] rather
than let the operator judge colour on a surface that is silently wrong. The binding changes when the
majority-area monitor changes, which re-derives the display transform under [STU-COL-303].

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `display_binding.majority_area_fraction` | 0.5 | 1.0 | 0.5 | 1.0 | 0.5 | unit_interval | 3 |
| `display_binding.rebind_hysteresis` | 0.0 | 0.5 | 0.0 | 0.2 | 0.05 | unit_interval | 3 |

Neither bound is a measurement. `majority_area_fraction` is fixed at 0.5 by the definition of a
majority and cannot be lower; its upper bound is the whole window. `rebind_hysteresis` is a Studio
normative choice under [STU-COL-107], declared here as such: it exists so a window dragged along a
monitor boundary does not re-materialise its display transform on every frame, and the 0.05 default
is a judgement, not an observation. No capture in this corpus declares either value.

The largest-area rule is not invented here either. It is what the platform call that maps a window to
a monitor already does - it returns the monitor holding the largest area of intersection - and it is
what one of the two major browser engines resolves a window's display with. The other engine is the
counter-example this clause exists to avoid: it applies the PRIMARY monitor's profile to every window
regardless of which monitor the window is on, and does not re-resolve when a window moves, which is
a defect its own issue tracker has carried open for years. Studio MUST bind per window, and MUST
re-bind on move.

**[STU-COL-306] The unmanaged display state is explicit and visible.** Where no profile resolves for
a display device, Studio enters `display_color_state = unmanaged` for that device. Unmanaged is a
NAMED, STORED, REPORTED state and MUST NOT be implemented as a silent substitution of sRGB. Studio
MUST present values through the `unmanaged_linear` path of [STU-COL-154] when in this state, MUST
show a persistent operator-visible indication that colour on that display is unverified, and MUST
report the state in the inspection state of [STU-COL-307]. Assuming sRGB is the specific failure this
clause exists to forbid: it is correct often enough to hide the problem and wrong often enough to
lose work, and it is indistinguishable from a working configuration in a screenshot.

**[STU-COL-307] Display characterisation in the inspection state.** [STU-COL-154] already argues
that a screenshot cannot distinguish the three display-transform modes, so the selection is
inspection state. The same argument reaches further and this clause carries it there: a screenshot
also cannot distinguish a correct display profile from a wrong one, an operator override from a
host reading, a stale association from a current one, or a colour-managed window from one spanning
onto a second monitor. Studio MUST therefore expose, as structured Argus inspection state
under [STU-COL-250], for every display device and every window: the `display_device_id` and its
`identity_confidence`; the `active_profile_ref` and the profile's `display_name`; the
`profile_source` and whether it is an override; `acquired_at`; the `display_color_state`; the
window-to-device binding and whether the window is spanning; and the cache key of the materialised
display transform in force ([STU-COL-142]). A headless model MUST be able to read "this window is
unmanaged" as a value rather than infer it from a picture.

**[STU-COL-308] Hardware calibration is an optional out-of-process adapter lane.** Driving a
colorimeter or spectrophotometer to MEASURE a display and generate a profile is OUT of the Studio
process and is an optional `StudioDeviceAdapter` lane, on the same posture as the colour-book data
adapter of [STU-COL-185] and the generative adapter of [STU-COL-237]. It MUST NOT be a hard
dependency, MUST NOT be required for any capability in this sub-section, and MUST NOT be the only
path to a display profile - an operator who supplies a profile under [STU-COL-304] never touches the
lane. Studio links no instrument library into `studio-engine` or `handshake_core`; the no-platform-CMM
tripwire of [STU-COL-261] applies unchanged, and an instrument SDK linked into either crate is the
same defect.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Adapter boundary field | Contract |
|---|---|
| `invocation` | A separate process invoked with arguments and observed through its exit status and its files. Never an in-process library call, and never a dynamically loaded instrument driver. |
| `capability_probe` | The lane reports whether an instrument is present, and its identity, WITHOUT taking a measurement. Absence is an ordinary answer, not an error. |
| `inputs` | A test-patch set to display and a measurement configuration, written to a working directory Studio owns. |
| `outputs` | A measurement data file and an ICC display profile written to that working directory. The profile is then ingested through [STU-COL-304] exactly as an operator-supplied profile would be; the lane has no privileged path into the engine. |
| `licence_isolation` | The adapter's licence terms attach to the adapter, never to Studio. The lane MUST run correctly against a replacement adapter, so no adapter's file naming, option spelling or version is a Studio contract. |
| `absence_behaviour` | With no adapter installed, every clause of this sub-section still holds and the operator uses host or supplied profiles. Nothing degrades and nothing is disabled. |

The boundary is specified as processes and files because that is the boundary the field-standard
open-source calibration toolchain actually exposes: separate command-line executables that generate
a test-patch list, discover and drive an instrument, read measured patches into a measurement data
file, build an ICC profile from that file, and install the profile through the host's own mechanism.
There is no documented library or daemon interface; the integration surface IS the file set and the
exit status. A second and independent reason for the process boundary is licensing - that toolchain
is distributed under a strong copyleft licence, with a separate commercial licence sold for
closed-source incorporation, so linking it into `studio-engine` would be a licensing decision
disguised as an engineering one. Studio names no adapter product in this module, and an implementer
MUST NOT harden the lane around one adapter's spelling of its options or its file extensions.

---

### 14.8.21 ACES Scene-Linear Default

**[STU-COL-310] ACES is the named scene-linear default (CLOSING A NAMED GAP), and it is a
CONFIGURATION contract rather than new machinery.** [STU-COL-150] already mandates an
OpenColorIO-class configuration at both profile-version-1 and version-2 semantics, [STU-COL-151]
already requires the whole configuration surface to be exposed, and [STU-COL-156] already lists
ACES2065-1, ACEScg, ACEScc and ACEScct among the working spaces a timeline may declare. What was
missing is the naming: nothing said WHICH configuration governs scene-linear work, so two documents
could both be "scene-linear", disagree completely, and neither could say why. Studio therefore
fixes a DEFAULT SCENE-LINEAR CONTRACT, expressed entirely in machinery [STU-COL-150]
through [STU-COL-157] already require, adding no engine capability: when a document works scene-linear, the
active configuration MUST satisfy the role bindings of [STU-COL-312] and MUST offer the display
targets and views of [STU-COL-313], and the document MUST record which configuration it was
authored against under [STU-COL-314]. A configuration that does not satisfy the contract is
reported as non-satisfying under [STU-COL-315]; it is never silently accepted. This does not
reverse [STU-COL-153]: that clause forbids bundling a configuration captured from a source
application, and whether an openly published reference configuration ships with Studio remains a
content decision under [STU-COL-268], not a specification decision. The contract binds either way.

**[STU-COL-311] The ACES encodings Studio MUST carry.** Four encodings, and they are four different
jobs. Treating them as interchangeable "ACES" is the single most common way a scene-linear pipeline
goes wrong: an archival encoding used as a render working space wastes precision on colours no
render produces, and a log grading encoding that cannot represent values at or below zero silently
destroys the shadow detail a lift operation needs.

*Derivation: catalogue table, splits per row; yields 4 microtasks, one per ACES encoding.*

| Colour space | Primaries and white point | Encoding | Job in the default binding |
|---|---|---|---|
| `ACES2065-1` | AP0 primaries, which enclose the visible spectral locus, on the ACES white point at chromaticity x 0.32168, y 0.33767 | scene-linear | Archival and interchange only. The space the interchange role of [STU-COL-316] resolves to, and the space an ACES container writes. Not a working space. |
| `ACEScg` | AP1 primaries, narrower than AP0 and close to but not identical with BT.2020, on the same white point | scene-linear | The DEFAULT working and compositing space. AP1 is chosen over AP0 because render and composite arithmetic on AP0 primaries produces negative components and wastes precision. |
| `ACEScc` | AP1 primaries, same white point | logarithmic, pure log with no toe; cannot represent values at or below zero | Offered for grading compatibility, and NOT the default. A lift applied in this encoding clips at zero. |
| `ACEScct` | AP1 primaries, same white point | logarithmic with a toe below the breakpoint 0.0078125; identical to `ACEScc` above that breakpoint | The DEFAULT log grading encoding. The toe is what lets it carry true black and negative values through a lift, which is the difference an operator actually feels. |

**[STU-COL-312] Role bindings under the default.** [STU-COL-152] fixes the role vocabulary Studio
MUST recognise and states it as a minimum. This clause states what those roles RESOLVE TO under the
scene-linear default, which is the part that makes two hosts agree. A role is still an indirection
([STU-COL-152]): a document naming a role and opened under a different satisfying configuration
resolves to that configuration's binding, not to the names below.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Role | Resolves to under the default | Why this binding |
|---|---|---|
| `scene_linear` | `ACEScg` | The working space every render, composite and adjustment evaluates in. |
| `compositing_log` | `ACEScct` | The log encoding a log-domain operation converts through. |
| `color_timing` | `ACEScct` | The encoding the grade stack of [STU-COL-160] operates in, so wheels and curves behave as a colourist expects. |
| `matte_paint` | `ACEScct` | Paint work needs the same log response as grading, not a linear one. |
| `texture_paint` | A display-encoded BT.709 space | Texture authoring happens against a display encoding, not a scene encoding. |
| `color_picking` | A display-encoded BT.709 space | The picker of [STU-COL-230] shows the operator display-referred numbers; picking in scene-linear presents values no operator can reason about. |
| `data` | The configuration's raw pass-through space | Non-colour channels. A transform applied to a data channel corrupts it, which is why the role exists at all. |
| `aces_interchange` | `ACES2065-1` | The scene-referred interchange anchor of [STU-COL-316]. |
| `cie_xyz_d65_interchange` | The configuration's display-referred CIE XYZ D65 space | The display-referred interchange anchor of [STU-COL-316]. |

The last two extend the minimum vocabulary of [STU-COL-152], which does not name them. They are
added here because without a declared interchange anchor on each side, a value cannot be carried
from one configuration to another at all, and [STU-COL-157] requires ICC and OCIO to be two paths
through one engine rather than two closed worlds.

**[STU-COL-313] Display targets and their view transforms.** A view transform under a
version-2-semantics configuration is a TWO-STAGE conversion and both stages are named separately:
the view transform converts the scene-referred reference to a display-referred reference, and the
display colour space then converts that to the physical display's own encoding ([STU-COL-151]
exposes both as `view_transforms` and `display_colorspaces`). Collapsing them into one baked
transform per display is what makes a pipeline unable to add a display target without re-authoring
every view. Studio MUST support both stages, and MUST support a shared view referenced from more
than one display rather than duplicated per display.

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per display target.*

| Display target | Standard it encodes | Tone-mapped view required | Colorimetric and pass-through views required |
|---|---|---|---|
| `srgb_display` | IEC 61966-2-1 sRGB | An SDR view at the 100 cd/m2 reference presentation | An untone-mapped colorimetric view and a raw data pass-through view |
| `rec1886_rec709_display` | ITU-R BT.1886 EOTF on ITU-R BT.709 primaries | An SDR view at the 100 cd/m2 reference presentation | An untone-mapped colorimetric view and a raw data pass-through view |
| `gamma22_rec709_display` | A pure 2.2 power EOTF on ITU-R BT.709 primaries, kept separate from `rec1886_rec709_display` because the two EOTFs differ in the shadows | An SDR view at the 100 cd/m2 reference presentation | An untone-mapped colorimetric view and a raw data pass-through view |
| `p3_d65_display` | DCI-P3 primaries on a D65 white point | An SDR view rendered for the P3 D65 gamut | An untone-mapped colorimetric view and a raw data pass-through view |
| `display_p3_display` | DCI-P3 primaries on D65 with the sRGB piecewise EOTF, kept separate from `p3_d65_display` because the EOTF differs | An SDR view rendered for the P3 D65 gamut | An untone-mapped colorimetric view and a raw data pass-through view |
| `rec2100_pq_display` | ITU-R BT.2100 with the perceptual quantiser transfer function | HDR views at more than one peak luminance, selected by `aces_view.peak_luminance` below, and rendered for a declared limiting gamut | An untone-mapped colorimetric view and a raw data pass-through view |
| `rec2100_hlg_display` | ITU-R BT.2100 with the hybrid log-gamma transfer function | An HDR view at the peak luminance the target declares | An untone-mapped colorimetric view and a raw data pass-through view |

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `aces_view.peak_luminance` | UNKNOWN | UNKNOWN | 100 | 4000 | 100 | nits | 0 |
| `aces_view.limiting_primaries_index` | 0 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | index | 0 |

The soft bounds on `aces_view.peak_luminance` are OBSERVED under [STU-COL-108], not declared: the
published reference configuration examined for this module presents one SDR view at 100 and HDR views
at 500, 1000, 2000 and 4000. No source declares a hard bound on either side, so both are `UNKNOWN`
and are NOT mirrored from the soft bounds. This parameter is distinct from the document-level
`hdr_peak_luminance` of [STU-COL-122]: that one describes the DOCUMENT's dynamic range, this one
selects which VIEW is presented, and a document may be graded at one and viewed at another.

**[STU-COL-314] Configuration provenance on the document.** A grade is only reproducible if the
document says what it was graded through. A document working scene-linear MUST therefore carry a
configuration provenance record, and it MUST be carried even when the configuration was resolved by
a named identifier rather than from a file, because an identifier that resolves differently on
another host is exactly the failure this record exists to catch.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Meaning |
|---|---|
| `config_identifier` | The configuration's published identifier string, which by field convention carries the configuration version, the ACES version and the configuration-syntax version in one token, of the shape `<variant>-config-v<config>_aces-v<aces>_ocio-v<syntax>`. Stored verbatim as read; Studio does not reformat it. |
| `config_name` | The configuration's own declared name key, which may differ from the file name it arrived in. |
| `config_syntax_version` | The configuration's declared profile-syntax version, which is what decides whether the two-stage display model of [STU-COL-313] is available at all. |
| `aces_version` | The ACES release the configuration declares. `UNKNOWN` where the configuration declares none; Studio MUST NOT infer one from the identifier string, and MUST NOT substitute the version it happens to know. |
| `config_content_hash` | Content hash of the serialised configuration bytes, which is the identity [STU-COL-150] already requires. |
| `config_closure_id` | A hash covering the serialised configuration AND every external LUT file it references. A configuration whose text is unchanged but whose referenced LUT file changed is a DIFFERENT transform graph, and `config_content_hash` alone cannot see that. |
| `resolution_mode` | `operator_supplied_file`, `named_identifier`, or `absent`. |
| `display_view_selection` | The (display, view) pair in force, which is the `ocio_display` selection [STU-COL-154] already defines. |

**[STU-COL-315] Configuration absence, mismatch and re-resolution.** Three cases, each with a
required behaviour and none of them a silent one. Where a document names a configuration that does
not resolve on this host, Studio MUST open the document, MUST mark it configuration-absent, MUST
report the recorded `config_identifier` so the operator knows what to supply, and MUST NOT
substitute another configuration - a grade re-rendered through a different transform graph is a
wrong image that looks like a working one. Where a configuration resolves but its
`config_closure_id` differs from the recorded one, Studio MUST report the mismatch, name which side
differs, and require an explicit operator or model decision to proceed. Where a configuration
resolves and satisfies [STU-COL-312] and [STU-COL-313] but the recorded `aces_version` is `UNKNOWN`
on either side, the document opens and the unknown is reported as unknown; an unrecorded version is
a missing fact, not a mismatch, and MUST NOT be reported as one.

**[STU-COL-316] Interchange anchors.** A configuration is a closed world unless it declares what
its spaces mean in terms something outside it can read. Studio MUST require, of any configuration
satisfying the scene-linear default, a scene-referred interchange anchor bound to `ACES2065-1` and
a display-referred interchange anchor bound to a display-referred CIE XYZ D65 space, and MUST use
those two anchors when transferring a value between two configurations or between the OCIO path and
the ICC path of [STU-COL-157]. A configuration missing either anchor is reported as non-satisfying
under [STU-COL-315]. This is also what lets a colour picked in a colour-managed ICC document and a
colour graded in a scene-linear document be compared at all, which the accuracy metric
of [STU-COL-320] depends on.

**[STU-COL-317] What this module does NOT fix about the ACES release.** Stated so an implementer
does not read a version number into a contract that does not carry one. This module fixes the ROLE
BINDINGS, the DISPLAY TARGETS and the PROVENANCE RECORD. It does NOT fix an ACES release number, and
`aces_version` is recorded from what the configuration declares rather than asserted by Studio, for a
reason recorded in [STU-COL-340]: at the time this module was written the current release was
documented by the publishing project's own changelog and announcement but was not marked by a
corresponding repository release tag, so the release identifier is a fact to be READ from a
configuration and never a constant to be COMPILED IN. A build that hardcodes a version string here
will be wrong within one release cycle and will be wrong silently.

---

### 14.8.22 Colour Accuracy Metric and Reference Patch Sets

**[STU-COL-320] The colour-accuracy metric (CLOSING A NAMED GAP).** This module requires colour to be
CORRECT in more than thirty places and, until this clause, stated no number anywhere: no colour
difference formula, no perceptual colour space in which to compute one, and no tolerance. An
obligation with no number cannot be gated by a validator, cannot fail a regression, and cannot be
argued about with evidence, which means "the colour is wrong" was an opinion. Studio therefore fixes
ONE metric. The colour difference between two colours is CIEDE2000, computed on CIELAB values, and
reported as `delta_e_2000`. CIELAB is the colour space of ISO/CIE 11664-4; CIEDE2000 is the formula
of ISO/CIE 11664-6, which originates in CIE technical report 142-2001. The simple Euclidean CIELAB
difference of 1976 MUST NOT be used as the acceptance metric, because it over-weights chroma
differences and a pipeline tuned to pass it is tuned to the wrong thing; Studio MAY report it
alongside as `delta_e_76` for comparison with legacy figures, clearly labelled, and MUST NOT gate on
it. Every reported difference carries its reference white and observer, which are never implicit:
a measurement on an ICC path uses the D50 profile-connection-space white with the 2-degree observer,
whose tristimulus values are X 0.9642, Y 1.0000, Z 0.8249, and a measurement on a display or video
path uses D65 with the 2-degree observer. Comparing a number computed under one white point with a
number computed under the other is a category error, so the white point is part of the value.

**[STU-COL-321] Metric parametric factors.** CIEDE2000 carries three parametric weighting factors.
Their standard reference-condition values are all 1, and a value other than 1 is an application
convention, not a different formula. Studio stores all three explicitly so a reported difference can
never be reproduced incorrectly by an implementation that assumed a convention.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `delta_e.k_l` | UNKNOWN | UNKNOWN | 1 | 2 | 1 | dimensionless | 2 |
| `delta_e.k_c` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 1 | dimensionless | 2 |
| `delta_e.k_h` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 1 | dimensionless | 2 |

The defaults are the standard's reference conditions and are DECLARED, not chosen. The soft range on
`delta_e.k_l` is OBSERVED under [STU-COL-108]: the only other value in general use is 2, which is a
textile-industry convention and not part of the standard's default. No source declares a hard bound
on any of the three, so all six hard fields are `UNKNOWN` and are NOT mirrored from the soft fields.

**[STU-COL-322] Metric implementation conformance.** A CIEDE2000 implementation that is subtly wrong
produces plausible numbers, so the metric itself MUST be proven before anything is gated on it.
Studio's implementation MUST reproduce the published supplementary test data for the formula: 34
CIELAB colour pairs with their computed reference differences, published by Sharma, Wu and Dalal in
2005 precisely because the standard's own worked examples were too sparse to expose the branch bugs
below. Reproducing all 34 is an acceptance obligation, not a suggestion.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Implementation trap | What goes wrong, and the required handling |
|---|---|
| Hue-difference quadrant | The hue difference branches on whether the absolute difference exceeds 180 degrees, subtracting or adding 360 depending on which hue is larger. Getting the branch wrong is invisible on most pairs and badly wrong on a few. The mean hue has its OWN, different branch and MUST NOT reuse the first one. |
| Undefined hue at the achromatic point | With both the chroma-corrected `a` and `b` at zero the two-argument arctangent is undefined in most languages. The hue MUST be set to zero by convention rather than left to whatever the platform returns. |
| The chroma correction factor | The `a` axis is scaled by a chroma-dependent factor computed from the MEAN chroma of the PAIR before hue and chroma are taken. An implementation that computes hue from the uncorrected `a` is a different formula. |
| The blue-region rotation term | A rotation term centred near hue 275 degrees couples the chroma and hue terms. Omitting it leaves the formula numerically close on most pairs and wrong exactly where the formula was designed to be right. |

**[STU-COL-323] Per-path accuracy tolerances.** A tolerance is only meaningful per PATH, because the
four paths have different irreducible error. A conversion inside the engine is arithmetic and should
be near-exact; a soft proof simulates a device that does not exist on the screen it is shown on, and
cannot be. Studio therefore declares a tolerance per path, as a mean and a maximum over the reference
patch set of [STU-COL-324], and a validator gates on both.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `accuracy.document_conversion.mean_delta_e` | 0 | UNKNOWN | 0 | 2.5 | 0.5 | dimensionless | 2 |
| `accuracy.document_conversion.max_delta_e` | 0 | UNKNOWN | 0 | 5.0 | 1.0 | dimensionless | 2 |
| `accuracy.display_transform.mean_delta_e` | 0 | UNKNOWN | 0 | 3.0 | 1.0 | dimensionless | 2 |
| `accuracy.display_transform.max_delta_e` | 0 | UNKNOWN | 0 | 5.0 | 2.0 | dimensionless | 2 |
| `accuracy.soft_proof.mean_delta_e` | 0 | UNKNOWN | 0 | 5.0 | 2.5 | dimensionless | 2 |
| `accuracy.soft_proof.max_delta_e` | 0 | UNKNOWN | 0 | 10.0 | 5.0 | dimensionless | 2 |
| `accuracy.export_conversion.mean_delta_e` | 0 | UNKNOWN | 0 | 5.0 | 2.5 | dimensionless | 2 |
| `accuracy.export_conversion.max_delta_e` | 0 | UNKNOWN | 0 | 10.0 | 5.0 | dimensionless | 2 |

Which of these are MEASURED and which are JUDGEMENTS is stated here rather than left for a reader
to guess, because a validator gating on a judgement the reader believed was a standard is worse
than no gate. The soft-proof and export defaults, mean 2.5 and maximum 5.0, are taken from the
certified proofing standard ISO 12647-7, whose 2016 edition states its tolerances in CIEDE2000
rather than in the 1976 difference its earlier editions used; that same edition states 3.0 for
substrate and for process-colour solids and 2.5 for spot inks, and specifies viewing under ISO
3664. Those figures were read for this module from a full-text mirror rather than from an official
issue of the standard, and [STU-COL-340] carries that as an open item: the microtask derived from
this clause MUST confirm each cited cell against an official issue before the gate is enforced, and
MUST record the confirmed values. `hard_min` 0 is declared by the formula, since a colour
difference cannot be negative. No source surveyed declares a hard upper bound on a tolerance, so
`hard_max` is `UNKNOWN` throughout and is NOT mirrored from `soft_max`. The document-conversion and
display-transform defaults are Studio NORMATIVE CHOICES under [STU-COL-107] and are labelled as
such: no standard surveyed states a numeric colour-difference gate for an in-engine conversion, and
none states one for display-calibration verification either. Figures that circulate for
professional display work - a mean under about 2 and a maximum under about 3 to 4 - are
practitioner practice, not a standard, and this module does not cite them as one. The
displayed-calibration standard ISO 12646 does state a display uniformity tolerance, but it states
it as a chromaticity radius rather than as a colour difference, and translating it gives a figure
that varies by more than a factor of five across the lightness range, so it is not usable as a
single gate. An operator or a house standard MAY tighten any row; the defaults exist so that a
regression is VISIBLE, which is the property the module lacked entirely.

**[STU-COL-324] The reference patch corpus.** A tolerance needs something to measure. Studio ships a
reference corpus of known patches with expected values, and the corpus is layered: synthetic sets
prove the arithmetic with no measurement uncertainty at all, and measured sets prove the device paths.
A corpus row is a separately buildable thing, so each is its own unit of work.

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per reference patch set.*

| Reference patch set | What it contains | Where the expected values come from | The caveat that makes naive use wrong |
|---|---|---|---|
| `patches.metric_conformance` | 34 CIELAB colour pairs with their reference CIEDE2000 differences | Published supplementary test data for the formula ([STU-COL-322]) | It proves the METRIC, not the pipeline. Passing it says nothing about any transform. |
| `patches.synthetic_ramp` | Neutral ramps and per-channel ramps at the document precision, plus the primaries and secondaries of each supported working space | Computed analytically from the space definitions, with no measurement | Expected values MUST be computed independently of the engine under test ([STU-COL-325]), or the corpus is the engine's own output and proves nothing. |
| `patches.synthetic_gamut_edge` | Values deliberately outside the destination gamut, at and just past the boundary, for each rendering intent | Computed from the destination profile's own gamut tag or its inverse transform | Two intents legitimately give two different answers here, so a single expected value per patch is wrong; the expected value is per intent ([STU-COL-135]). |
| `patches.reflective_chart_24` | The 24-patch reflective reference chart in general use for camera and display checks | The chart manufacturer's published colorimetric reference file for the chart's own production era | The chart's pigment formulation changed, and the two eras differ by up to more than one unit of difference on several patches. A corpus that does not RECORD which era's reference file it used is unusable, and the era is not inferable from the chart. |
| `patches.print_characterisation` | The print characterisation patch set of ISO 12642-2, 1617 patches, which is a superset of the 928-patch set of ISO 12642-1 | Measured from the operator's own printed and measured proof; there is no universal published expected table | The expected values are the operator's own measurement, so this set proves REPEATABILITY and conformance to a proofing tolerance, not agreement with a universal reference. |
| `patches.scene_referred_reference_images` | The published scene-referred reference images for the transform suite of 14.8.21, as image files rather than as patches | The publishing project's own reference renders per transform | The publisher states no numeric match tolerance, so any tolerance Studio applies to these is Studio's own choice and MUST be recorded as such ([STU-COL-340]). |

**[STU-COL-325] Expected values are computed independently of the engine under test.** A golden corpus
generated by running the engine and recording what came out proves that the engine is
self-consistent, which it already was. Every expected value in [STU-COL-324] MUST come from an
independent source: a published reference table, an independently implemented reference computation,
or a physical measurement. Where an expected value is produced by a second implementation rather than
read from a publication, the corpus record MUST name that implementation and its version, so a later
disagreement can be attributed rather than argued. This clause is the accuracy-domain instance of the
rule already stated for determinism in [STU-COL-143]: an engine cannot be its own authority.

**[STU-COL-326] Accuracy is queryable state, not a test log.** A measured difference MUST be readable
as structured inspection state under [STU-COL-250], not only printed by a test harness, because the
operator question "is this document's display path accurate right now" is asked at runtime and not at
build time. Studio MUST expose, per path of [STU-COL-323] and per corpus of [STU-COL-324]: the mean
and maximum `delta_e_2000`, the patch identifier of the worst patch, the reference white and observer
used, the corpus identifier and its recorded era or version, and a pass or fail against the tolerance
in force. A headless model MUST be able to read "the soft-proof path is at mean 2.9 against a
tolerance of 2.5, worst patch `p_17`" rather than infer accuracy from a picture, which is the same
argument [STU-COL-164] already makes for gamut coverage and [STU-COL-154] makes for the display path.

---

### 14.8.23 Video Scopes and Legal Range

**[STU-COL-330] Scopes are a colour-analysis contract owned here (CLOSING A NAMED GAP).** Section 14
contained no video scope of any kind: every waveform in the timeline sub-section is an AUDIO
waveform, and there was no vectorscope, no parade, no histogram over a video signal and no
legal-range checking anywhere. That is a hole in COLOUR, not in the timeline: a scope measures a
colour signal, in a declared colour space, against a declared standard, and all three of those are
this module's vocabulary. This sub-section therefore specifies the MEASUREMENT contract - what is
measured, where in the pipeline, in what space, against which standard, and how the result is read.
Where a scope appears as a panel, that panel is an operator surface belonging to the surrounding
application shell and to the monitor surface of 14.25; this sub-section does not edit 14.25 and
takes nothing from it. A scope is an OVERLAY-class computation under [STU-COL-164]: it is computed
from the pipeline and MUST NOT alter an authority value.

**[STU-COL-331] The scope catalogue.** Six scopes, each measuring a different quantity. They are
separately implementable and each is its own unit of work.

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per scope.*

| Scope | What it measures | Axes | Graticule and targets |
|---|---|---|---|
| `scope.waveform_luma` | Signal amplitude of the luma component | Horizontal picture position against amplitude | Reference black and nominal peak white at the levels of [STU-COL-333], plus the preferred and total range limits of [STU-COL-335] |
| `scope.rgb_parade` | Signal amplitude of R, G and B, drawn as three cells side by side | Horizontal picture position against amplitude, per cell | Same levels as the luma waveform, drawn in each cell, because a channel can be out of range while luma is not |
| `scope.ycbcr_parade` | Signal amplitude of the luma and the two colour-difference components, as three cells | Horizontal picture position against amplitude, per cell | Luma cell uses the luma levels; the colour-difference cells use the achromatic centre and the plus and minus peak levels of [STU-COL-333] |
| `scope.vectorscope` | Chrominance as a polar plot | Hue as the angle, chroma as the radius from the centre | Boxes for the three primaries and the three secondaries, positioned where a correctly encoded colour-bar signal lands, with complementaries diametrically opposite. A flesh-tone reference direction MAY be drawn and, if drawn, its angle MUST be declared as a Studio choice, because no standards document surveyed for this module fixes one ([STU-COL-340]). |
| `scope.histogram` | Distribution of code values per component over the whole frame, with no spatial dimension | Code value against pixel count, linear or logarithmic | Clipping markers at the extremes of the declared range |
| `scope.chromaticity` | Distribution of pixel chromaticity on a chromaticity diagram | The two chromaticity coordinates | Gamut boundary outlines for the working space, the target display and the target delivery standard, drawn as separate outlines so the operator sees which boundary is being crossed |

**[STU-COL-332] The measurement point is declared, never assumed.** The single most consequential
scope decision is WHERE in the pipeline of [STU-COL-161] the measurement is taken, and no standard
surveyed for this module specifies it: the colour-management library this module builds on is
deliberately agnostic about it and leaves it to the host, and the applications that document a
convention document it as an operator-selectable setting. A scope that does not say where it measured
is therefore not a measurement, it is a picture. Studio MUST make the point EXPLICIT, selectable, and
reported with every reading.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| `scope_measurement_point` | Where it taps the pipeline of [STU-COL-161] | What it answers |
|---|---|---|
| `working_space_pre_grade` | After stage 3, before the grade stack | What the source actually contains, independent of the grade |
| `working_space_post_grade` | After stage 5, still in the timeline working space of [STU-COL-156] | What the grade produced. The DEFAULT, because it is the value that will be encoded on delivery. |
| `display_referred` | After stage 6a, past the display transform or the view transform of [STU-COL-313] | What the operator's monitor is being sent, which is what a viewer sees and is NOT what gets delivered |
| `proof_referred` | On the 6c proof branch | What the simulated output condition of [STU-COL-241] would produce |
| `export_encoding` | After stage 6b, in the destination encoding with its declared range | Whether the DELIVERABLE is legal, which is the only measurement point a delivery specification accepts |

Measuring at `display_referred` and reporting the result as a delivery check is the specific error
this enumeration exists to prevent: the display path carries a view transform and a monitor profile
that the deliverable does not, so a signal can look legal on that scope and be illegal in the file.

**[STU-COL-333] Scales, units and reference levels.** A waveform is read in one of several scales and
they are not interchangeable. Studio MUST expose the scale as an explicit selection and MUST label
every reading with it.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| `scope_scale` | Meaning |
|---|---|
| `code_value` | The raw quantised integer at the signal's bit depth. The only scale in which a legal-range check is exact. |
| `percent` | Nominal range mapped to 0 to 100, with sub-black and super-white expressed as negative and above-100 values rather than clipped away |
| `ire` | The traditional analogue-referenced scale |
| `millivolts` | The analogue voltage reference, where reference black is 0 and reference white is 700, and the colour-difference components run plus and minus 350 |
| `nits` | Absolute display luminance in cd/m2, for the high-dynamic-range readings of [STU-COL-336] |

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Quantisation level | 8-bit | 10-bit | 12-bit |
|---|---|---|---|
| Reference black, luma | 16 | 64 | 256 |
| Achromatic, colour-difference components | 128 | 512 | 2048 |
| Nominal peak white, luma | 235 | 940 | 3760 |
| Colour-difference peak, minus and plus | 16 and 240 | 64 and 960 | 256 and 3840 |
| Total video-data range, both ends reserved | 1 to 254 | 4 to 1019 | 16 to 4079 |
| Full-range coding, both components | 0 to 255 | 0 to 1023 | 0 to 4095 |

These levels are the narrow-range quantisation of the ITU-R broadcast recommendations, stated
identically in BT.709 for standard dynamic range and in BT.2100 for high dynamic range, and they are
DECLARED values rather than Studio choices. Sub-black is the region below reference black down to the
bottom of the video-data range, and super-white the region above nominal peak white up to its top;
both are legal signal and MUST be displayed by a scope rather than clipped out of the trace, because
a scope that hides them cannot show the operator the thing that is wrong.

**[STU-COL-334] Legal, full and valid are three different things.** Conflating them is the usual
cause of a delivery rejection that the operator's own scope said was fine.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Term | Meaning | What it does NOT tell you |
|---|---|---|
| `full_range` | The signal uses the whole code-value space at its bit depth, with black at 0 and white at the maximum | Nothing about broadcast legality; a full-range signal delivered as narrow-range is simply wrong, not illegal |
| `narrow_range` | The signal uses the reference black and nominal peak white of [STU-COL-333], leaving sub-black and super-white headroom | Nothing about whether the component values are individually within their limits |
| `legal` | Every component of the signal, in its own encoding, is within the range that encoding permits | Nothing about what happens after conversion. A legal signal can still be invalid. |
| `valid` | The signal remains within 0 to 100 percent on every component AFTER conversion to the target RGB encoding | Nothing about the source encoding, which may have been legal all along |

A signal whose luma and colour-difference components are each individually within range can still
de-matrix into red, green or blue values outside 0 to 100 percent. That is the legal-but-not-valid
case, and it is what a gamut error means in a delivery specification. Studio MUST report the two
conditions SEPARATELY, because the fix is different: an out-of-range component is clipped or rescaled,
whereas an invalid combination needs a colour change.

**[STU-COL-335] Range and gamut error reporting against the target standard.** A scope MUST check the
signal against a declared TARGET, and the target is a delivery standard rather than a preference.
Studio MUST support the widely used broadcast tolerance recommendation, whose preferred limits sit
inside the total video-data range and are the limits a delivery check actually applies, and MUST
report an error as an AREA FRACTION rather than as a boolean, because a single stray pixel is not a
delivery failure and a specification that treats it as one trains the operator to ignore the warning.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `legal_range.preferred_min` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | -5 | percent | 1 |
| `legal_range.preferred_max` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 105 | percent | 1 |
| `legal_range.error_area_threshold` | 0 | 100 | 0 | 100 | 1 | percent | 2 |

The preferred limits are DECLARED by the broadcast tolerance recommendation, whose current version
tabulates them as code values per bit depth - 5 to 246 at 8 bits, 20 to 984 at 10 bits, 80 to 3936 at
12 bits and 1280 to 62976 at 16 bits - which are the minus 5 and plus 105 percent of nominal recorded
above. The same recommendation states that measuring equipment should indicate an out-of-gamut
occurrence only after the error exceeds 1 percent of the image, which is where the default area
threshold comes from; it is a declared figure, not a Studio judgement. No source declares a hard bound
on any of the three, except that an area fraction is bounded 0 to 100 by being a fraction, so the two
percent-of-nominal rows carry `UNKNOWN` on all four bound fields and those are NOT mirrored from the
defaults. That recommendation additionally specifies a filter to apply BEFORE gamut measurement - a
seven-tap horizontal quarter-band filter with coefficients 1, 2, 3, 4, 3, 2, 1 over sixteen, and a
three-tap vertical half-band filter with coefficients 1, 2, 1 over four - so that transient overshoot
and noise do not register as gamut errors. Studio MUST apply that filtering when measuring gamut error
and MUST NOT apply it to the displayed trace, which is a different question. Finally, an automatic
range legaliser is available but MUST NOT be applied silently: the same recommendation warns that a
legaliser can create artefacts more disturbing than the errors it corrects, so legalising is an
explicit operator or model action with a recorded history entry, never a repair Studio performs on
its own.

**[STU-COL-336] High-dynamic-range readings.** In high dynamic range a percentage scale is close to
meaningless to a colourist, who works in absolute luminance. Studio MUST offer the `nits` scale
of [STU-COL-333] for a high-dynamic-range signal, MUST draw the reference levels below on it, and MUST
report the two content light-level figures that a delivery specification asks for.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `scope.hdr_reference_white` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 203 | nits | 0 |
| `scope.hdr_grey_card_level` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 26 | nits | 0 |

Both defaults are DECLARED, not chosen. The high-dynamic-range reference white - the nominal luminance
obtained from a 100 percent reflectance white card - is stated as 203 cd/m2 in the ITU-R
high-dynamic-range image-parameter recommendation and again in its companion operational report, which
also gives the 18 percent grey card at 26 cd/m2 and a 90 percent greyscale step at 179 cd/m2. Neither
source states a bound on either value, so all eight bound fields are `UNKNOWN`. The two transfer
functions of that recommendation, perceptual quantisation and hybrid log-gamma, place the same
reference white at different signal levels, so a nits scale MUST be derived through the transfer
function actually in force and never through a fixed table. Maximum content light level is the
luminance of the brightest single colour component over every pixel of every frame, and maximum
frame-average light level is the highest per-frame average of the brightest component per pixel; both
are computed over the whole delivered sequence, both are reported as `nits`, and both MUST be
recomputed rather than carried forward when the grade changes. The consumer-electronics standard that
defines those two figures was not read directly for this module and [STU-COL-340] records that;
the DEFINITIONS above are corroborated across multiple implementation sources and are what Studio
implements.

**[STU-COL-337] A scope reading is structured state.** Every scope MUST publish its reading as
queryable inspection state under [STU-COL-250], not only as a rendered trace, for the same
reason [STU-COL-164] gives for gamut coverage: a headless model cannot read a graph. At minimum, per
component and per frame: minimum, low, average, high and maximum levels; the saturation and hue
statistics a vectorscope reading implies; the out-of-range area fraction and the out-of-gamut area
fraction as SEPARATE numbers per [STU-COL-334]; the `scope_measurement_point` and the `scope_scale`
in force; the declared target standard; and, for high dynamic range, the two content light-level
figures of [STU-COL-336]. A model MUST be able to read "0.4 percent of the frame is out of gamut at
the export encoding, below the 1 percent threshold" as values.

---

### 14.8.24 Declared Open Items for 14.8.20 Through 14.8.23

**[STU-COL-340] Open items of record (NON-YIELDING).** Each item below is a question this module
could not close from a source it could read, and each is already stated inline in the clause it
affects, with a contract written so the unknown does not become an invented value. This clause is an
INDEX of those statements and yields no microtask of its own; the work of closing each one belongs to
the microtask of the clause that carries it, which is why no item here is orphaned. Listing them
together exists so a reviewer can audit the additions of 14.8.20 through 14.8.23 for invented facts in
one pass instead of reading four sub-sections.

*Derivation: reading aid inside a non-yielding clause; yields no microtask. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Open item | Clause that carries it | How the clause is written so the unknown is safe |
|---|---|---|
| The compositor colour-management protocol was published in the staging area of its protocol collection rather than the stable area, and was revised more than once while there | [STU-COL-301] | The acquisition path is a runtime probe with a declared fallback, not a fixed interface version |
| Docking and multi-stream topologies are reported in the field to change monitor enumeration and cache descriptors, but no primary specification for that behaviour was found | [STU-COL-302] | The host enumeration key is declared volatile by contract, so the behaviour cannot matter to identity |
| The current release identifier of the scene-linear transform suite is documented by its publisher's changelog and announcement but was not marked by a corresponding repository release tag | [STU-COL-317] | The version is READ from the configuration into `aces_version` and is `UNKNOWN` when the configuration declares none; nothing is compiled in |
| The certified-proofing tolerance cells were read from a full-text mirror rather than an official issue of the standard | [STU-COL-323] | The microtask of that clause MUST confirm each cited cell against an official issue before the gate is enforced, and record the confirmed values |
| No standard states a numeric colour-difference gate for an in-engine conversion or for display-calibration verification | [STU-COL-323] | Those two defaults are labelled Studio normative choices under [STU-COL-107], not measurements, in the clause itself |
| The publisher of the scene-referred reference images states no numeric match tolerance | [STU-COL-324] | Any tolerance applied to that corpus is recorded as a Studio choice rather than presented as the publisher's |
| No standards document surveyed fixes an angle for the flesh-tone reference direction on a vectorscope graticule | [STU-COL-331] | The line is optional, and if drawn its angle is declared as a Studio choice |
| The consumer-electronics standard defining the two content light-level figures was not read directly | [STU-COL-336] | The definitions are stated from corroborating implementation sources and the standard is not cited as if it had been read |
| Whether any candidate colour-management implementation can be forced onto a single scalar evaluation path by configuration alone | [STU-COL-351] | The obligation is stated on the PATH, not on a library: an implementation that cannot be forced scalar is one Studio implements for that path, which is a stated outcome rather than a blocker |
| Whether hardware three-dimensional texture sampling can be made to agree bit-for-bit with a software tetrahedral evaluation | [STU-COL-353] | It is assumed it cannot. The clause requires the rule to be declared per path and the discrepancy to be reported, rather than requiring an agreement no source claims |

---

### 14.8.25 Transform Materialisation and Application

**[STU-COL-350] Materialisation and application are two paths with two different determinism
obligations (CLOSING A DEFECT IN [STU-COL-143]).** [STU-COL-143] as originally written required "the
materialised transform" to be bit-identical on every host, and a reader could take that to govern the
whole engine including per-pixel evaluation. Under that reading the clause is unsatisfiable: every
colour-evaluation implementation in the field dispatches at runtime on the vector instruction sets it
finds - the several x86 generations, and the equivalents on other architectures - and none of them
states a bit-exactness guarantee across those paths. A requirement no implementation can meet is not
a strict requirement, it is a dead one, because it gets quietly waived the first time it fails.
Studio therefore splits the engine on the line [STU-COL-142] already draws, and each half carries the
obligation it can actually satisfy.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Path | What it does | Its determinism obligation | Who owns it |
|---|---|---|---|
| Materialisation | Turns profile bytes plus the key of [STU-COL-142] into the matrix-and-curve chain or the baked LUT, once per cache key | BIT-IDENTICAL across hosts, architectures and instruction-set availability ([STU-COL-351]). This is the artefact that gets cached, compared, promoted and shipped, so it is the thing that has to be identical. | `ColorEngine` |
| Application | Pushes pixels through an already-materialised artefact | CPU-and-GPU EQUIVALENCE within the declared tolerance of [STU-COL-352]. Weaker than bit-identity, and deliberately so, because it is what a vector or GPU path can meet. | `RenderEngine`, per [STU-COL-163] |

The split costs nothing in speed. Materialisation runs once per cache key and its result is reused
for every pixel, so the argument for vectorising it is negligible while the argument against it is
the whole determinism contract. This split is required whatever colour-management implementation is
eventually selected, which is why it belongs in the specification rather than in an implementation
note: it is a property of the architecture, not of a library.

**[STU-COL-351] The materialisation path is scalar and reproducible.** The materialisation path
MUST evaluate on a single scalar code path with runtime vector dispatch DISABLED, at a fixed
evaluation order, a fixed rounding mode and a fixed internal precision satisfying [STU-COL-144].
The same key materialised on two different architectures, and on one architecture with and without
a wider instruction set available, MUST produce byte-identical artefacts, and that comparison is an
acceptance obligation rather than an assumption. Where a chosen implementation cannot be forced
onto a single scalar path by its own configuration, Studio implements THAT PATH itself
under [STU-COL-141]; that is a stated outcome of this clause, not a failure of it, and it is the reason
this clause names an obligation and not a library. Studio MUST record, per materialised artefact,
the engine identity and version that produced it, so an artefact that stops reproducing can be
attributed to a change rather than argued about.

**[STU-COL-352] The application path is held to a numeric tolerance.** The application path MAY use
vector instructions on the CPU and MUST use the GPU where [STU-COL-163] puts it. It is held to
agreement, not to identity, and the agreement is expressed in the SAME metric as everything else in
this module so that one number means one thing: the colour difference of [STU-COL-320], measured over
the reference corpus of [STU-COL-324]. Any two application paths applying the SAME materialised
artefact - CPU scalar, CPU vector, and GPU - MUST agree within the tolerance below, and a path that
does not is a defect in that path rather than a licence to loosen the tolerance.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `apply.path_agreement.mean_delta_e` | 0 | UNKNOWN | 0 | 1.0 | 0.1 | dimensionless | 3 |
| `apply.path_agreement.max_delta_e` | 0 | UNKNOWN | 0 | 2.0 | 0.5 | dimensionless | 3 |

Both defaults are Studio NORMATIVE CHOICES under [STU-COL-107] and are labelled as such: no source
surveyed states a numeric agreement tolerance between a vector and a scalar evaluation of the same
transform, nor between a software and a hardware one. They are set an order of magnitude below the
document-conversion tolerance of [STU-COL-323] because this is not a perceptual question at all - two
paths applying the identical artefact should differ only by accumulated rounding, so a difference
approaching a perceptible one is evidence that the paths are not applying the same transform.
`hard_min` 0 is declared by the metric. No hard upper bound is declared by any source, so `hard_max`
is `UNKNOWN` and is NOT mirrored from `soft_max`.

**[STU-COL-353] The interpolation rule is part of the transform's identity.** This clause closes a
second defect in [STU-COL-142]. The two interpolation rules in general use for a 3D colour lookup,
enumerated by [STU-COL-158] as `trilinear` and `tetrahedral`, are NOT equivalent: tetrahedral is
more accurate at the same grid size, and the two give different results on the same grid and the
same data. The gap this clause closes is that [STU-COL-142] originally declared grid size and
domain but said nothing about the rule, so two materialisations differing only in it could collide
in one cache entry and a viewport could disagree with a proof render while both correctly claimed
to be applying "the same transform". The concrete mechanism is well documented in the field:
hardware three-dimensional texture sampling provides trilinear only, so a tetrahedral rule on the
GPU has to be written explicitly rather than sampled, and at least one widely used
colour-management implementation records its own GPU path falling back to the linear rule where its
CPU path uses the tetrahedral one, and records that the two are not always equivalent.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Obligation | Requirement |
|---|---|
| Declared field | `interpolation` is a REQUIRED field of every materialised transform carrying a lookup table, taking its values from the enumeration of [STU-COL-158]. It has no default and MUST NOT be inferred from the sampler. |
| Cache key | `interpolation` is part of the cache key of [STU-COL-142]. Two artefacts differing only in it are different artefacts. |
| Path binding | Every application path of [STU-COL-352] MUST apply the rule the artefact declares. A path that cannot MUST NOT silently apply a different one. |
| Declared discrepancy | Where a path genuinely cannot apply the declared rule, the artefact is marked with the rule that path actually applied, that fact is reported as inspection state under [STU-COL-250], and the resulting difference is measured against [STU-COL-352] rather than assumed small. Hiding the substitution is the defect; substituting and declaring it is a stated, bounded condition. |
| Proof and viewport | A proof render under [STU-COL-240] and the viewport MUST resolve to the same declared rule, or the proof MUST report that it did not. A soft proof that silently differs from the viewport is worse than no soft proof, because it is trusted. |

**[STU-COL-354] Where a baked lookup table stops being an acceptable materialisation.** A baked table
is an approximation and there are five cases where the approximation is not merely coarse but WRONG,
and no grid size fixes any of them. Each is a separate engine behaviour with its own resolution.

*Derivation: catalogue table, splits per row; yields 5 microtasks, one per non-bakeable case.*

| Non-bakeable case | Why a baked three-dimensional table cannot serve it | Required resolution |
|---|---|---|
| `bake.four_or_more_input_channels` | A CMYK or N-channel SOURCE needs a four-dimensional or higher lookup; a hardware three-dimensional texture is three-dimensional | Evaluate the source-side transform on the CPU, or split the chain and put only the connection-space-to-display half on the GPU. A CMYK soft-proof viewport is the concrete case and MUST NOT be served from a three-dimensional table. |
| `bake.gamut_clip_reporting` | [STU-COL-138] requires clipping to be reportable per pixel or per object, and the clipping happens inside the bake, so the signal is gone before a pixel is sampled | Bake a companion channel or companion table carrying the clip flag, or evaluate the report on the CPU. Not solvable by raising the grid size. |
| `bake.named_and_spot_values` | A spot resolved through a named-colour profile ([STU-COL-185]) is an exact table lookup, not a point in a continuous space, and interpolating it produces a different colour | Named and spot values bypass the lookup table entirely and resolve against their own table. |
| `bake.out_of_domain_float` | In a 32-bit float scene-linear pipeline, values below 0 and above 1 are ordinary and lie outside the declared domain; sampling there is extrapolation and clamping there destroys highlight and negative-lobe data | The behaviour outside the declared domain is part of the engine contract and MUST be declared explicitly, never left to a sampler's clamp mode. This is what the required domain of [STU-COL-158] and [STU-COL-142] is for. |
| `bake.hard_compression_near_black_or_white` | Absolute-colorimetric paper-white simulation and strong black point compensation both compress a narrow input region hard, which is exactly where a uniform grid has least resolution | A shaper table is mandatory here, and the grid takes the upper size of [STU-COL-355] rather than the default. |

**[STU-COL-355] Grid size, shaper and storage precision for a baked materialisation.** The shaper
requirement of [STU-COL-142] is MANDATORY, not an optimisation, for any input that is not
perceptually uniform - every log encoding, every scene-linear encoding and every high-dynamic-range
encoding - because a uniform grid over a non-uniform input spends its resolution where the eye cannot
see it and starves the region where it can. Storage precision of a baked table MUST be at least the
document precision required by [STU-COL-144]: an 8-bit or 10-bit normalised-integer table is not an
acceptable materialisation for a 16-bit or float document, half-float is the floor for
high-dynamic-range and scene-linear work, and a pipeline that is float throughout stores float.
Where a chosen implementation defaults to a fixed-point evaluation path, that default MUST be
explicitly overridden and the override proven for float documents rather than assumed.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `materialised_transform.grid_size` | 2 | UNKNOWN | 33 | 65 | 33 | count | 0 |
| `materialised_transform.preview_grid_size` | 2 | UNKNOWN | 17 | 33 | 17 | count | 0 |

`hard_min` 2 follows from the format, exactly as it does for `lut.grid_size` in [STU-COL-158], and no
source declares a hard upper bound, so `hard_max` is `UNKNOWN` on both rows and is NOT mirrored from
`soft_max`. The soft ranges are OBSERVED under [STU-COL-108]: lookup-table interchange in the field
settled on 17, 33 and 65 points per side, 33 is the practical floor for a display transform, 65 is
what a strongly non-linear transform needs, and 17 is a preview grid rather than an output grid. The
defaults are Studio choices within that observed set and are labelled as such. Memory is not a reason
to choose a coarser grid: a 65-per-side three-channel float table is a few megabytes, which is
negligible against the error a coarse grid introduces.

