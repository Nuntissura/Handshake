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
declared_yields_total: 225
yields_ledger_clause: "STU-COL-272"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
anchor_prefix: "STU-COL"
anchor_range_new: "STU-COL-100 .. STU-COL-273 (plus STU-COL-104A)"
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
microtask set (14.8.22), from this module plus the shared contracts it names (14.0 storage,
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

**SELF-AUDIT.** This module carries 120 numeric parameter rows across 28 parameter tables. **35**
carry a complete seven-field set with no `UNKNOWN`; **85** carry at least one stated `UNKNOWN`; 331
of the 840 individual fields are `UNKNOWN`. Colour has a materially higher completion rate than
typography (which has zero complete rows, see [STU-TYP-238]) for one reason: the captured grading
surface declares hard and soft bounds SEPARATELY on many of its parameters, and the colour-component
type libraries declare bounds and defaults. Those declarations are the reason the seven-field
contract exists at all ([STU-COL-105]).

Every one of those 120 rows carries `hard_min`, `hard_max`, `soft_min`, `soft_max`, `default`,
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
transform is non-linear. Pixels MUST NOT reach `RenderEngine` with an unresolved profile reference.
The materialised transform is CACHEABLE and MUST be keyed by
(source profile hash, destination profile hash, intent, BPC, precision, grid size).

**[STU-COL-143] Transform determinism.** For a fixed key under [STU-COL-142], the materialised
transform MUST be bit-identical on every host. Interpolation, rounding mode and clamping behaviour
are part of the engine contract, not implementation freedom. This is a promotion-equivalence
requirement of 14.24: a colour conversion that differs by one least-significant bit across hosts
fails promotion.

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
MUST include the twelve descriptors below. This table SPAWNS TWELVE microtasks, one per row.

*Derivation: catalogue table, splits per row; yields 12 microtasks, one per validation descriptor. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

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

**[STU-COL-261] No-platform-CMM tripwire.** The build MUST fail if any crate in the `studio-engine`
or `handshake_core` dependency graph links a platform colour-management module or a
system-installed colour library. The tripwire runs alongside the SQLite tripwire of [STU-OVR-003]
and the text-engine tripwire of [STU-TYP-231].

**[STU-COL-262] Mandatory acceptance cases.** Acceptance MUST include the eighteen cases below.
This table SPAWNS EIGHTEEN microtasks, one per row.

*Derivation: catalogue table, splits per row; yields 18 microtasks, one per acceptance case. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Acceptance case | What it proves | Governing clause |
|---|---|---|
| `acc.model_round_trip` | A value round-trips through every model pair with bounded error | [STU-COL-111] |
| `acc.assign_vs_convert` | Assign leaves components unchanged; convert changes them and preserves appearance within a declared tolerance | [STU-COL-131] |
| `acc.four_intents_differ` | All four intents give four different results for an out-of-gamut source, and BPC changes the result independently of intent | [STU-COL-135] |
| `acc.device_link_inert` | A device-link profile reports intent and BPC as inert rather than applying them | [STU-COL-134] |
| `acc.cross_host_bit_identical` | The same conversion on two hosts is bit-identical | [STU-COL-143] |
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
the grading model; soft proof, gamut warning and colour-vision simulation.

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
146 clause anchors, every one of them as a paragraph opening with its bold anchor at line start,
none inside a table cell, none inside a blockquote and none inside a fenced block. Subtracting the
17 anchors of the non-yielding set above leaves 129 yielding clauses, and 129 is exactly what
the clause rows of the ledger in [STU-COL-272] sum to. A tool that reaches a different
yielding-clause count for 14.8 is either not seeing all 146 definitions or honouring more than
17 exclusions, and this arithmetic says which. Note that the non-yielding set names only anchors
this module defines: an anchor from the superseded v02.205 module cannot be excluded here because it
was never counted here in the first place.

**Rule 0a -- anchors inside table cells are never definitions here.** Every one of the 146 clauses
in 14.8 is defined as a PARAGRAPH opening with its bold anchor at line start; not one is defined
inside a table cell. 126 distinct anchors appear in cells of this sub-section, and they fall into
exactly two categories, neither of which is a definition. 91 are cross-references to clauses
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

**Declared non-yielding set (NORMATIVE, by anchor).** These seventeen clauses yield NOTHING. They
are authority bookkeeping, scope statements, pure cross-references, or obligations that attach to
every other microtask rather than forming one:
`STU-COL-100`, `STU-COL-101`, `STU-COL-102`, `STU-COL-103`, `STU-COL-104`, `STU-COL-104A`,
`STU-COL-250`, `STU-COL-265`, `STU-COL-266`, `STU-COL-267`, `STU-COL-268`, `STU-COL-270`,
`STU-COL-271`, `STU-COL-272`, `STU-COL-273`, `STU-COL-274`, `STU-COL-275`.
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
| Numeric parameter tables (28 tables, 120 rows) | throughout 14.8.2-14.8.15 | parameter table | 28 |
| Rendering intents | [STU-COL-135] | enumeration | 1 |
| Swatch kinds | [STU-COL-171] | catalogue row | 7 |
| Gradient geometries | [STU-COL-190] | catalogue row | 7 |
| Colour adjustment catalogue | [STU-COL-229] | catalogue row | 23 |
| Validation descriptors | [STU-COL-260] | validator | 12 |
| Mandatory acceptance cases | [STU-COL-262] | acceptance case | 18 |
| Declared non-yielding clauses | [STU-COL-270] non-yielding set | excluded | 0 |
| **TOTAL** | **14.8 whole** | **all kinds** | **225** |

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
| All numeric parameter tables | throughout | 120 | parameter table, taken whole (each) | 28 |
| Anchor | [STU-COL-102] | 34 | reading aid in a non-yielding clause | 0 |
| Field | [STU-COL-106] | 7 | contract table carried into its clause | 0 |
| Model | [STU-COL-121] | 8 | contract table carried into its clause | 0 |
| Studio `intent` | [STU-COL-135] | 4 | enumeration table, taken whole | 1 |
| Stage | [STU-COL-161] | 9 | contract table carried into its clause | 0 |
| `swatch_kind` | [STU-COL-171] | 7 | catalogue, splits per row (one per swatch kind) | 7 |
| Gradient geometry | [STU-COL-190] | 7 | catalogue, splits per row (one per gradient geometry) | 7 |
| Adjustment | [STU-COL-229] | 23 | catalogue, splits per row (one per colour adjustment) | 23 |
| Validation descriptor | [STU-COL-260] | 12 | catalogue, splits per row (one per validation descriptor) | 12 |
| Acceptance case | [STU-COL-262] | 18 | catalogue, splits per row (one per acceptance case) | 18 |
| Unit group | [STU-COL-272] | 26 | reading aid in a non-yielding clause | 0 |
| Table (first column) | [STU-COL-274] | 15 | reading aid in a non-yielding clause | 0 |
| Missed unit group | [STU-COL-275] | 6 | reading aid in a non-yielding clause | 0 |
| **TOTAL TABLE UNITS** | **all tables** | **296** | **computed from the markers above** | **96** |

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

