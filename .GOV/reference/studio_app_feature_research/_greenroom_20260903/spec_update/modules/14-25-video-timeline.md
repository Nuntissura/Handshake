---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
bundle_status: "staged_draft_not_yet_in_bundle"
module_id: "14-25"
section_id: "14.25"
title: "14.25 Studio -- Video Editing, Sequences & the Clip Timeline"
supersedes_clause: "[STU-OVR-015]"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 14.25 Video Editing, Sequences & the Clip Timeline

## 14.25.0 Supersession, scope and authority

### 1. [STU-OVR-015] is superseded

**[STU-VID-001] [STU-OVR-015] IS SUPERSEDED AND INVERTED.** That clause, added at v02.200, reads
that editing imported video footage as a clip timeline -- "trimming/sequencing/encoding video clips
as raster-video layers, i.e. a non-linear video editor" -- is OUT of Studio scope, and routes
footage clip editing to a separate mechanical engine. That is no longer true. The operator's
instruction of 2026-09-04 is recorded verbatim: *"full blown video capabilitie for profesional
editors and vfx artists."* Timeline clip editing is IN scope at professional depth. Compositing and
visual effects are IN scope at professional depth (14.27). Nothing from the video-editing or
compositing capability surface is excluded on scope grounds.

**[STU-VID-001a]** The consequences of the supersession, stated so nothing is left dangling:

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| What [STU-OVR-015] said | What now holds |
|---|---|
| Footage clip editing is out of Studio scope. | Studio owns it. This sub-section is its normative specification. |
| Studio's motion surface is keyframe animation plus motion export only. | Studio's motion surface is a full keyframe and property-tree animation system (14.26) AND a clip timeline (this sub-section) AND a compositing system (14.27). They are three surfaces, not one. |
| Footage clip editing belongs to `engine.director` (Spec 11.8). | `engine.director` remains a mechanical engine with its own purpose. Studio's clip timeline is an operator-and-model-facing creative surface with a document model, history, CRDT collaboration and a typed command surface. Where the two touch, `engine.director` is a consumer of `StudioSequence` output, not the owner of clip editing. Any clause in 11.8 that claims ownership of interactive clip editing is superseded to that extent. |
| Studio MAY place and render video media but does not own footage clip editing. | Studio places, trims, sequences, composites, grades, mixes and encodes video media. |

**[STU-VID-001b]** The domain list in 14.1 section 2 is amended: the row reading "Prototyping, motion &
interaction | 14.11" no longer stands alone as Studio's motion answer. Three domains join the
catalogue: **Video editing, sequences & the clip timeline (14.25)**, **Motion graphics & keyframing
(14.26)**, and **Compositing & visual effects (14.27)**. 14.11 retains prototyping and interactive
documents; the phrase "prototyping/motion" in 14.1's opening paragraph is amended to
"prototyping, motion graphics, video editing and compositing", because motion graphics is not clip
assembly and neither of them is compositing.

### 2. What this sub-section owns

**[STU-VID-002] Ownership boundaries.** This sub-section owns the CLIP TIMELINE: sequences,
tracks, clips, edit points, trimming, the source and program monitors, multicam, media import,
transcoding and export. It does NOT own the keyframe timeline, the property tree, the expression
language or compositions -- those are 14.26. It does NOT own layer compositing, keying, mattes,
tracking or 3D -- those are 14.27. It does NOT own effect parameters -- those are 14.9 as replaced.
It does NOT own colour grading -- that is 14.8. The two timelines are distinct surfaces and [STU-VID-022]
states how they relate.

**[STU-VID-003] No sidecar authority.** Every enumeration, default, range, unit and structural
contract below is stated here. The green-room captures are derivation provenance recorded in the
accompanying `.provenance.json` and are not required reading for an implementer
([STU-SECTION-002] as amended).

---

## 14.25.1 The Sequence and its settings

[STU-VID-010] **`StudioSequence` (schema id `hsk.studio.sequence@1`) is the timeline document
primitive.** A sequence is a member of the unified `StudioDocument` ([STU-DOC-001]), not a separate
file type: one Handshake project may hold sequences, compositions ([STU-MOT-001]), artboards and
page spreads side by side, sharing one selection surface, one history, one colour pipeline and one
export surface ([STU-DOC-004]). A sequence owns an ordered set of `StudioTrack`s, a settings
record, a time model, in/out points, a work area, and a marker list. A sequence MAY be nested as a
clip inside another sequence ([STU-VID-030]) and MAY be referenced as a layer source inside a
composition ([STU-CMP-004]).

**[STU-VID-011] The normative sequence settings record.** Every field below is required. Values
marked "Studio default" are Studio's own choice from the evidence; values marked "declared" are read
from shipped configurations.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Type | Contract |
|---|---|---|
| `frame_size` | `{width: u32, height: u32}` | Composition raster size in pixels. 37 distinct sizes appear across the shipped configurations, from 512x256 to 8192x8192. Studio default 1920x1080 (the most common, 89 of 392). |
| `pixel_aspect_ratio` | `{numerator: u32, denominator: u32}` | A RATIONAL, never a float. Eight distinct ratios are declared: 1:1 (332 configurations), 1920:1440, 1920:1920, 10:11, 40:33, 768:702, 1024:702, 3:2. Studio default 1:1. Storing a float loses exactness and breaks round-trip. |
| `frame_rate` | `{ticks_per_frame: u64}` | See [STU-VID-012]. Stored as ticks, never as a float. |
| `field_order` | enum | `progressive`, `upper_field_first`, `lower_field_first`. Declared distribution: 346 progressive, 34 upper-first, 12 lower-first. Studio default `progressive`. |
| `video_time_display` | enum | See [STU-VID-012]. |
| `audio_time_display` | enum | `audio_samples` or `milliseconds`; independent of the video display. All 392 shipped configurations declare `audio_samples`. |
| `audio_sample_rate_hz` | u32 | 386 of 392 declare 48000; 6 declare 32000. Studio default 48000. |
| `audio_master_channel_type` | enum | See [STU-FX-137]. |
| `working_color_space` | `StudioColorProfile` ref | Declared values in the shipped set: `BT.709 RGB Full` (22 configurations) and `BT.2100 HLG RGB Full` (5). Studio default `BT.709 RGB Full`. Carries a `is_linearized` boolean, declared 0 in every shipped configuration read. |
| `auto_tone_map` | bool | Declared true in the shipped HDR-capable configurations. Governs automatic tone-mapping of out-of-gamut source media into the working space. |
| `use_maximum_bit_depth` | bool | Studio default false. When true the render pipeline runs at the document's maximum channel depth rather than the preview depth. |
| `use_maximum_render_quality` | bool | Studio default false. Selects the higher-cost scaling and resampling path. |
| `allow_linear_compositing` | bool | Studio default true in the shipped configurations read. Selects linear-light blending for the sequence's composite ([STU-CMP-020]). |
| `initial_video_track_count` | u8 | Studio default 3. |
| `initial_audio_track_count` | u8 | Studio default 4, matching the shipped stereo configurations. |
| `preview_render_format` | record | See [STU-VID-019]. |
| `immersive` | record | See [STU-VID-018]. |

### The time model

**[STU-VID-012] Time is integer ticks, and the tick rate is a constant.** A sequence's time base is
`254016000000` ticks per second. Every time value in a sequence -- clip in and out points, edit
points, marker positions, the playhead, work-area bounds, keyframe times on clip effects -- is
stored as an integer tick count. Floating-point seconds are a display and API convenience only and
MUST NOT be the stored form: at 254016000000 ticks per second, a 64-bit integer represents just over
two years of media exactly, while a float loses frame-accuracy inside an hour. This constant is
chosen precisely because it divides evenly by every common frame rate and audio sample rate, which
is what makes 23.976, 29.97 and 48000 Hz exact rather than approximate.

**[STU-VID-013] The frame-rate table is normative and is expressed in ticks per frame.**

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| ticks_per_frame | fps | Notes |
|---|---|---|
| 10594584000 | 23.976024 | 24000/1001. 75 shipped configurations. |
| 10584000000 | 24 | 43 shipped configurations. |
| 10160640000 | 25 | 85 shipped configurations. |
| 8475667200 | 29.97003 | 30000/1001. 103 shipped configurations; the most common. |
| 8467200000 | 30 | 6 shipped configurations. |
| 5292000000 | 48 | 5 shipped configurations. |
| 5080320000 | 50 | 35 shipped configurations. |
| 4237833600 | 59.94006 | 60000/1001. 34 shipped configurations. |
| 4233600000 | 60 | 5 shipped configurations. |
| 20321280000 | 12.5 | declared by the decode table; not used by the shipped configurations. |
| 16934400000 | 15 | declared by the decode table; not used by the shipped configurations. |

**[STU-VID-013a]** Two additional tick values decode to 23.976 and 29.97 as exact decimals rather than
as the 1001-denominator ratios (`10594594594` and `8475675675`). Studio treats the 1001-denominator
values as canonical and the exact-decimal values as a legacy import shape that is normalised on
import, because only the ratio form keeps timecode drift correct. The composition-settings surface
additionally offers a fixed frame-rate picker whose members are 8, 12, 15, 23.976, 24, 25, 29.97,
30, 50, 59.94, 60 and 120 fps, with a free numeric entry bounded `hard_min` 1, `hard_max` 999.

**[STU-VID-012a] The video time-display enumeration is normative.** It selects how a tick position
is rendered to the operator and how typed timecode entry is parsed. It is INDEPENDENT of the frame
rate: a 25 fps sequence may legally display 24 fps timecode, and a sequence may display frames or
film footage instead of timecode.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Code | Display | Observed in shipped configurations |
|---|---|---|
| 100 | 24 fps timecode | 40 |
| 101 | 25 fps timecode | 83 |
| 102 | 29.97 fps drop-frame timecode | 91 |
| 103 | 29.97 fps non-drop-frame timecode | 19 |
| 104 | 30 fps timecode | 6 |
| 105 | 50 fps timecode | 38 |
| 106 | 59.94 fps drop-frame timecode | 37 |
| 107 | 59.94 fps non-drop-frame timecode | -- |
| 108 | 60 fps timecode | 2 |
| 109 | frames | -- |
| 110 | 23.976 fps timecode | 71 |
| 111 | 16 mm feet + frames | -- |
| 112 | 35 mm feet + frames | -- |
| 200 | audio samples | 392 (audio display) |
| 201 | milliseconds | -- |

**[STU-VID-012b]** Code 113 appears in 5 shipped video configurations and has no member in the
recovered enumeration. Studio MUST NOT guess its meaning; it is declared gap [STU-VID-081]. An
importer encountering it falls back to the sequence's frame-rate-matched timecode display and
records a warning rather than silently choosing.

**[STU-VID-012c] Drop-frame is a display property, not a time property.** It changes only how ticks
are formatted and parsed; it never changes a stored position, a duration, or a frame count. The
composition-settings surface exposes the choice as an explicit `Drop Frame` / `Non-Drop Frame`
selection alongside the frame rate. Timecode arithmetic in Studio operates on ticks and formats at
the boundary ([STU-DOC-003] unit law).

### Sequence presets

**[STU-VID-016] `StudioSequencePreset` is a named, complete sequence settings record.** Studio
ships a preset library organised as a category path, exactly as the effect preset registry is
([STU-FX-143]). 392 shipped configurations across 82 groups are reproduced in full below, because a
preset's usefulness is entirely in its field values and a preset list without them is a list of
names. One preset is exactly one settings record; there is no partial preset.

**[STU-VID-016a]** The preset library is data in the `StudioStyleRegistry`, is portable
([STU-FX-039a]), and a sequence's current settings are savable as a new preset. Creating a preset
from a sequence is a typed command, not a UI-only affordance.

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Preset | Group | Frame size | PAR | fps | Field order | Time display | Audio | Video tracks | Working colour space |
|---|---|---|---|---|---|---|---|---|---|
| 23.98p 4 mono discrete | Broadcast | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 23.98p 6 mono discrete | Broadcast | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 23.98p 8 mono discrete | Broadcast | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 25i 4 mono discrete | Broadcast | 1920x1080 | 1:1 | 25 | upper field first | 25 fps timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 25i 6 mono discrete | Broadcast | 1920x1080 | 1:1 | 25 | upper field first | 25 fps timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 25i 8 mono discrete | Broadcast | 1920x1080 | 1:1 | 25 | upper field first | 25 fps timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 29.97i 4 mono discrete | Broadcast | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 29.97i 6 mono discrete | Broadcast | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| 29.97i 8 mono discrete | Broadcast | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | BT.709 RGB Full |
| HD 1080p 23.976 fps | HD 1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| HD 1080p 25 fps | HD 1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| HD 1080p 29.97 fps | HD 1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| HD 1080p 50 fps | HD 1080p | 1920x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| HD 1080p 59.94 fps | HD 1080p | 1920x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| ARRI 1080p 23.976 | Legacy/ARRI/1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 1080p 24 | Legacy/ARRI/1080p | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 1080p 25 | Legacy/ARRI/1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 1080p 29.97 | Legacy/ARRI/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| ARRI 1080p 30 | Legacy/ARRI/1080p | 1920x1080 | 1:1 | 30 | progressive (no fields) | 30 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 2880p 23.976 | Legacy/ARRI/2880p | 2880x1620 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 2880p 24 | Legacy/ARRI/2880p | 2880x1620 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 2880p 25 | Legacy/ARRI/2880p | 2880x1620 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| ARRI 2880p 29.97 | Legacy/ARRI/2880p | 2880x1620 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| ARRI 2880p 30 | Legacy/ARRI/2880p | 2880x1620 | 1:1 | 30 | progressive (no fields) | 30 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 1080i50 | Legacy/AVC-Intra/1080i | 1920x1080 | 1:1 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 1080i60 | Legacy/AVC-Intra/1080i | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 1080i50 | Legacy/AVC-Intra/1080i | 1440x1080 | 1920:1440 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 1080i60 | Legacy/AVC-Intra/1080i | 1440x1080 | 1920:1440 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 1080p24 | Legacy/AVC-Intra/1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 1080p25 | Legacy/AVC-Intra/1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 1080p30 | Legacy/AVC-Intra/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 1080p24 | Legacy/AVC-Intra/1080p | 1440x1080 | 1920:1440 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 1080p25 | Legacy/AVC-Intra/1080p | 1440x1080 | 1920:1440 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 1080p30 | Legacy/AVC-Intra/1080p | 1440x1080 | 1920:1440 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 720p24 | Legacy/AVC-Intra/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 720p25 | Legacy/AVC-Intra/720p | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 720p30 | Legacy/AVC-Intra/720p | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 720p50 | Legacy/AVC-Intra/720p | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 100 720p60 | Legacy/AVC-Intra/720p | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 720p24 | Legacy/AVC-Intra/720p | 960x720 | 1920:1440 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 720p25 | Legacy/AVC-Intra/720p | 960x720 | 1920:1440 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 720p30 | Legacy/AVC-Intra/720p | 960x720 | 1920:1440 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 720p50 | Legacy/AVC-Intra/720p | 960x720 | 1920:1440 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| AVC-I 50 720p60 | Legacy/AVC-Intra/720p | 960x720 | 1920:1440 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080i25 (50i) | Legacy/AVCHD/1080i | 1920x1080 | 1920:1920 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080i25 (50i) Anamorphic | Legacy/AVCHD/1080i | 1440x1080 | 1920:1440 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080i30 (60i) | Legacy/AVCHD/1080i | 1920x1080 | 1920:1920 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080i30 (60i) Anamorphic | Legacy/AVCHD/1080i | 1440x1080 | 1920:1440 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p24 | Legacy/AVCHD/1080p | 1920x1080 | 1920:1920 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p24 Anamorphic | Legacy/AVCHD/1080p | 1440x1080 | 1920:1440 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p25 | Legacy/AVCHD/1080p | 1920x1080 | 1920:1920 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p25 Anamorphic | Legacy/AVCHD/1080p | 1440x1080 | 1920:1440 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p30 | Legacy/AVCHD/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p50 | Legacy/AVCHD/1080p | 1920x1080 | 1920:1920 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 1080p60 | Legacy/AVCHD/1080p | 1920x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 720p24 | Legacy/AVCHD/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 720p25 | Legacy/AVCHD/720p | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 720p30 | Legacy/AVCHD/720p | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 720p50 | Legacy/AVCHD/720p | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| AVCHD 720p60 | Legacy/AVCHD/720p | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 1080i25 (50i) | Legacy/Canon XF MPEG2/1080i | 1920x1080 | 1:1 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 1080i30 (60i) | Legacy/Canon XF MPEG2/1080i | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 1080p24 | Legacy/Canon XF MPEG2/1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 1080p24N | Legacy/Canon XF MPEG2/1080p | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 1080p25 | Legacy/Canon XF MPEG2/1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 1080p30 | Legacy/Canon XF MPEG2/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 720p24 | Legacy/Canon XF MPEG2/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 720p24N | Legacy/Canon XF MPEG2/720p | 1280x720 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 720p25 | Legacy/Canon XF MPEG2/720p | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 720p30 | Legacy/Canon XF MPEG2/720p | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 720p50 | Legacy/Canon XF MPEG2/720p | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| Canon XF MPEG2 720p60 | Legacy/Canon XF MPEG2/720p | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080i 50 | Legacy/DNxHD/1080i 50 | 1920x1080 | 1:1 | 25 | upper field first | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080i 50 | Legacy/DNxHD/1080i 50 | 1920x1080 | 1:1 | 25 | upper field first | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080i 50 | Legacy/DNxHD/1080i 50 | 1920x1080 | 1:1 | 25 | upper field first | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080i 59.94 | Legacy/DNxHD/1080i 59.94 | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080i 59.94 | Legacy/DNxHD/1080i 59.94 | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080i 59.94 | Legacy/DNxHD/1080i 59.94 | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 23.976 | Legacy/DNxHD/1080p 23.976 | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 23.976 | Legacy/DNxHD/1080p 23.976 | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX LB 1080p 23.976 | Legacy/DNxHD/1080p 23.976 | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX RGB 444 1080p 23.976 | Legacy/DNxHD/1080p 23.976 | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 23.976 | Legacy/DNxHD/1080p 23.976 | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 24 | Legacy/DNxHD/1080p 24 | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 24 | Legacy/DNxHD/1080p 24 | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX LB 1080p 24 | Legacy/DNxHD/1080p 24 | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX RGB 444 1080p 24 | Legacy/DNxHD/1080p 24 | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 24 | Legacy/DNxHD/1080p 24 | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 25 | Legacy/DNxHD/1080p 25 | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 25 | Legacy/DNxHD/1080p 25 | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX LB 1080p 25 | Legacy/DNxHD/1080p 25 | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX RGB 444 1080p 25 | Legacy/DNxHD/1080p 25 | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 25 | Legacy/DNxHD/1080p 25 | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 29.97 | Legacy/DNxHD/1080p 29.97 | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 29.97 | Legacy/DNxHD/1080p 29.97 | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX LB 1080p 29.97 | Legacy/DNxHD/1080p 29.97 | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX RGB 444 1080p 29.97 | Legacy/DNxHD/1080p 29.97 | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 29.97 | Legacy/DNxHD/1080p 29.97 | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 50 | Legacy/DNxHD/1080p 50 | 1920x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 50 | Legacy/DNxHD/1080p 50 | 1920x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 50 | Legacy/DNxHD/1080p 50 | 1920x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 59.94 | Legacy/DNxHD/1080p 59.94 | 1920x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 59.94 | Legacy/DNxHD/1080p 59.94 | 1920x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 59.94 | Legacy/DNxHD/1080p 59.94 | 1920x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 1080p 60 | Legacy/DNxHD/1080p 60 | 1920x1080 | 1:1 | 60 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 1080p 60 | Legacy/DNxHD/1080p 60 | 1920x1080 | 1:1 | 60 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 1080p 60 | Legacy/DNxHD/1080p 60 | 1920x1080 | 1:1 | 60 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 720p 23.976 | Legacy/DNxHD/720p 23.976 | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 720p 23.976 | Legacy/DNxHD/720p 23.976 | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 720p 23.976 | Legacy/DNxHD/720p 23.976 | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 720p 25 | Legacy/DNxHD/720p 25 | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 720p 25 | Legacy/DNxHD/720p 25 | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 720p 25 | Legacy/DNxHD/720p 25 | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 720p 29.97 | Legacy/DNxHD/720p 29.97 | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 720p 29.97 | Legacy/DNxHD/720p 29.97 | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 720p 29.97 | Legacy/DNxHD/720p 29.97 | 1280x720 | 1:1 | -- | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 720p 50 | Legacy/DNxHD/720p 50 | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 720p 50 | Legacy/DNxHD/720p 50 | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 720p 50 | Legacy/DNxHD/720p 50 | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNX HQ 720p 59.94 | Legacy/DNxHD/720p 59.94 | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX HQX 720p 59.94 | Legacy/DNxHD/720p 59.94 | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNX SQ 720p 59.94 | Legacy/DNxHD/720p 59.94 | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 23.976 | Legacy/DNxHR/2K 23.976 | 2048x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 23.976 | Legacy/DNxHR/2K 23.976 | 2048x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 23.976 | Legacy/DNxHR/2K 23.976 | 2048x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 23.976 | Legacy/DNxHR/2K 23.976 | 2048x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 23.976 | Legacy/DNxHR/2K 23.976 | 2048x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 24 | Legacy/DNxHR/2K 24 | 2048x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 24 | Legacy/DNxHR/2K 24 | 2048x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 24 | Legacy/DNxHR/2K 24 | 2048x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 24 | Legacy/DNxHR/2K 24 | 2048x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 24 | Legacy/DNxHR/2K 24 | 2048x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 25 | Legacy/DNxHR/2K 25 | 2048x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 25 | Legacy/DNxHR/2K 25 | 2048x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 25 | Legacy/DNxHR/2K 25 | 2048x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 25 | Legacy/DNxHR/2K 25 | 2048x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 25 | Legacy/DNxHR/2K 25 | 2048x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 29.97 | Legacy/DNxHR/2K 29.97 | 2048x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 29.97 | Legacy/DNxHR/2K 29.97 | 2048x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 29.97 | Legacy/DNxHR/2K 29.97 | 2048x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 29.97 | Legacy/DNxHR/2K 29.97 | 2048x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 29.97 | Legacy/DNxHR/2K 29.97 | 2048x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 48 | Legacy/DNxHR/2K 48 | 2048x1080 | 1:1 | 48 | progressive (no fields) | unknown | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 48 | Legacy/DNxHR/2K 48 | 2048x1080 | 1:1 | 48 | progressive (no fields) | unknown | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 48 | Legacy/DNxHR/2K 48 | 2048x1080 | 1:1 | 48 | progressive (no fields) | unknown | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 48 | Legacy/DNxHR/2K 48 | 2048x1080 | 1:1 | 48 | progressive (no fields) | unknown | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 48 | Legacy/DNxHR/2K 48 | 2048x1080 | 1:1 | 48 | progressive (no fields) | unknown | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 50 | Legacy/DNxHR/2K 50 | 2048x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 50 | Legacy/DNxHR/2K 50 | 2048x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 50 | Legacy/DNxHR/2K 50 | 2048x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 50 | Legacy/DNxHR/2K 50 | 2048x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 50 | Legacy/DNxHR/2K 50 | 2048x1080 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 2K 59.94 | Legacy/DNxHR/2K 59.94 | 2048x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 2K 59.94 | Legacy/DNxHR/2K 59.94 | 2048x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 2K 59.94 | Legacy/DNxHR/2K 59.94 | 2048x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 2K 59.94 | Legacy/DNxHR/2K 59.94 | 2048x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 2K 59.94 | Legacy/DNxHR/2K 59.94 | 2048x1080 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 4K 23.976 | Legacy/DNxHR/4K 23.976 | 4096x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 4K 23.976 | Legacy/DNxHR/4K 23.976 | 4096x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 4K 23.976 | Legacy/DNxHR/4K 23.976 | 4096x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 4K 23.976 | Legacy/DNxHR/4K 23.976 | 4096x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 4K 23.976 | Legacy/DNxHR/4K 23.976 | 4096x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 4K 24 | Legacy/DNxHR/4K 24 | 4096x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 4K 24 | Legacy/DNxHR/4K 24 | 4096x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 4K 24 | Legacy/DNxHR/4K 24 | 4096x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 4K 24 | Legacy/DNxHR/4K 24 | 4096x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 4K 24 | Legacy/DNxHR/4K 24 | 4096x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 4K 25 | Legacy/DNxHR/4K 25 | 4096x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 4K 25 | Legacy/DNxHR/4K 25 | 4096x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 4K 25 | Legacy/DNxHR/4K 25 | 4096x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 4K 25 | Legacy/DNxHR/4K 25 | 4096x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 4K 25 | Legacy/DNxHR/4K 25 | 4096x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 4K 29.97 | Legacy/DNxHR/4K 29.97 | 4096x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 4K 29.97 | Legacy/DNxHR/4K 29.97 | 4096x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 4K 29.97 | Legacy/DNxHR/4K 29.97 | 4096x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 4K 29.97 | Legacy/DNxHR/4K 29.97 | 4096x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 4K 29.97 | Legacy/DNxHR/4K 29.97 | 4096x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 4K 50 | Legacy/DNxHR/4K 50 | 4096x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 4K 50 | Legacy/DNxHR/4K 50 | 4096x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 4K 50 | Legacy/DNxHR/4K 50 | 4096x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 4K 50 | Legacy/DNxHR/4K 50 | 4096x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 4K 50 | Legacy/DNxHR/4K 50 | 4096x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ 4K 59.94 | Legacy/DNxHR/4K 59.94 | 4096x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX 4K 59.94 | Legacy/DNxHR/4K 59.94 | 4096x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB 4K 59.94 | Legacy/DNxHR/4K 59.94 | 4096x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 4K 59.94 | Legacy/DNxHR/4K 59.94 | 4096x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ 4K 59.94 | Legacy/DNxHR/4K 59.94 | 4096x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ UHD 23.976 | Legacy/DNxHR/UHD 23.976 | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX UHD 23.976 | Legacy/DNxHR/UHD 23.976 | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB UHD 23.976 | Legacy/DNxHR/UHD 23.976 | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 UHD 23.976 | Legacy/DNxHR/UHD 23.976 | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ UHD 23.976 | Legacy/DNxHR/UHD 23.976 | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ UHD 24 | Legacy/DNxHR/UHD 24 | 3840x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX UHD 24 | Legacy/DNxHR/UHD 24 | 3840x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB UHD 24 | Legacy/DNxHR/UHD 24 | 3840x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 UHD 24 | Legacy/DNxHR/UHD 24 | 3840x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ UHD 24 | Legacy/DNxHR/UHD 24 | 3840x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ UHD 25 | Legacy/DNxHR/UHD 25 | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX UHD 25 | Legacy/DNxHR/UHD 25 | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB UHD 25 | Legacy/DNxHR/UHD 25 | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 UHD 25 | Legacy/DNxHR/UHD 25 | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ UHD 25 | Legacy/DNxHR/UHD 25 | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ UHD 29.97 | Legacy/DNxHR/UHD 29.97 | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX UHD 29.97 | Legacy/DNxHR/UHD 29.97 | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB UHD 29.97 | Legacy/DNxHR/UHD 29.97 | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 UHD 29.97 | Legacy/DNxHR/UHD 29.97 | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ UHD 29.97 | Legacy/DNxHR/UHD 29.97 | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ UHD 50 | Legacy/DNxHR/UHD 50 | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX UHD 50 | Legacy/DNxHR/UHD 50 | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB UHD 50 | Legacy/DNxHR/UHD 50 | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 UHD 50 | Legacy/DNxHR/UHD 50 | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ UHD 50 | Legacy/DNxHR/UHD 50 | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQ UHD 59.94 | Legacy/DNxHR/UHD 59.94 | 3840x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR HQX UHD 59.94 | Legacy/DNxHR/UHD 59.94 | 3840x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR LB UHD 59.94 | Legacy/DNxHR/UHD 59.94 | 3840x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR RGB 444 UHD 59.94 | Legacy/DNxHR/UHD 59.94 | 3840x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DNxHR SQ UHD 59.94 | Legacy/DNxHR/UHD 59.94 | 3840x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Standard 32kHz | Legacy/DV - 24P | 720x480 | 10:11 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 32000 Hz stereo | 3 | -- |
| Standard 48kHz | Legacy/DV - 24P | 720x480 | 10:11 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| Widescreen 32kHz | Legacy/DV - 24P | 720x480 | 40:33 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 32000 Hz stereo | 3 | -- |
| Widescreen 48kHz | Legacy/DV - 24P | 720x480 | 40:33 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| Standard 32kHz | Legacy/DV - NTSC | 720x480 | 10:11 | 29.97003 | lower field first | 29.97 fps drop-frame timecode | 32000 Hz stereo | 3 | -- |
| Standard 48kHz | Legacy/DV - NTSC | 720x480 | 10:11 | 29.97003 | lower field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Widescreen 32kHz | Legacy/DV - NTSC | 720x480 | 40:33 | 29.97003 | lower field first | 29.97 fps drop-frame timecode | 32000 Hz stereo | 3 | -- |
| Widescreen 48kHz | Legacy/DV - NTSC | 720x480 | 40:33 | 29.97003 | lower field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Standard 32kHz | Legacy/DV - PAL | 720x576 | 768:702 | 25 | lower field first | 25 fps timecode | 32000 Hz stereo | 3 | -- |
| Standard 48kHz | Legacy/DV - PAL | 720x576 | 768:702 | 25 | lower field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| Widescreen 32kHz | Legacy/DV - PAL | 720x576 | 1024:702 | 25 | lower field first | 25 fps timecode | 32000 Hz stereo | 3 | -- |
| Widescreen 48kHz | Legacy/DV - PAL | 720x576 | 1024:702 | 25 | lower field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPRO50 24p Standard | Legacy/DVCPRO50/480i | 720x480 | 10:11 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPRO50 24p Widescreen | Legacy/DVCPRO50/480i | 720x480 | 40:33 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPRO50 NTSC Standard | Legacy/DVCPRO50/480i | 720x480 | 10:11 | 29.97003 | lower field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DVCPRO50 NTSC Widescreen | Legacy/DVCPRO50/480i | 720x480 | 40:33 | 29.97003 | lower field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DVCPRO50 PAL Standard | Legacy/DVCPRO50/576i | 720x576 | 768:702 | 25 | lower field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPRO50 PAL Widescreen | Legacy/DVCPRO50/576i | 720x576 | 1024:702 | 25 | lower field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPROHD 1080i50 | Legacy/DVCPROHD/1080i | 1440x1080 | 1920:1440 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPROHD 1080i60 | Legacy/DVCPROHD/1080i | 1280x1080 | 3:2 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DVCPROHD 1080p24 | Legacy/DVCPROHD/1080p | 1280x1080 | 3:2 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPROHD 720p24 | Legacy/DVCPROHD/720p | 960x720 | 1920:1440 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPROHD 720p50 | Legacy/DVCPROHD/720p | 960x720 | 1920:1440 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DVCPROHD 720p60 | Legacy/DVCPROHD/720p | 960x720 | 1920:1440 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DSLR 1080p24 | Legacy/Digital SLR/1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DSLR 1080p25 | Legacy/Digital SLR/1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| DSLR 1080p30 | Legacy/Digital SLR/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DSLR 640x480p50 | Legacy/Digital SLR/480p | 640x480 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DSLR 640x480p60 | Legacy/Digital SLR/480p | 640x480 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| DSLR 720p24 | Legacy/Digital SLR/720p | 1280x720 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| DSLR 720p24 @ 23.976 | Legacy/Digital SLR/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| DSLR 720p50 | Legacy/Digital SLR/720p | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| DSLR 720p60 | Legacy/Digital SLR/720p | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| HDV 1080i25 (50i) | Legacy/HDV | 1440x1080 | 1920:1440 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| HDV 1080i30 (60i) | Legacy/HDV | 1440x1080 | 1920:1440 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| HDV 1080p24 | Legacy/HDV | 1440x1080 | 1920:1440 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| HDV 1080p25 | Legacy/HDV | 1440x1080 | 1920:1440 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| HDV 1080p30 | Legacy/HDV | 1440x1080 | 1920:1440 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| HDV 720p24 | Legacy/HDV | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| HDV 720p25 | Legacy/HDV | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| HDV 720p30 | Legacy/HDV | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 23.976 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 24 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 25 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 29.97 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 30 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 30 | progressive (no fields) | 30 fps timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 50 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 59.94 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| ProRes RAW 4K 60 | Legacy/ProRes RAW/4K | 4096x2160 | 1:1 | 60 | progressive (no fields) | 60 fps timecode | 48000 Hz stereo | 3 | -- |
| 1080p 16x9 23.976 | Legacy/RED R3D/1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 1080p 16x9 24 | Legacy/RED R3D/1080p | 1920x1080 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 1080p 16x9 25 | Legacy/RED R3D/1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 1080p 16x9 29.97 | Legacy/RED R3D/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 1K 16x9 23.976 | Legacy/RED R3D/1K | 1024x576 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 1K 16x9 24 | Legacy/RED R3D/1K | 1024x576 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 1K 16x9 25 | Legacy/RED R3D/1K | 1024x576 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 1K 16x9 29.97 | Legacy/RED R3D/1K | 1024x576 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 1K 2x1 23.976 | Legacy/RED R3D/1K | 1024x512 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 1K 2x1 24 | Legacy/RED R3D/1K | 1024x512 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 1K 2x1 25 | Legacy/RED R3D/1K | 1024x512 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 1K 2x1 29.97 | Legacy/RED R3D/1K | 1024x512 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 2K 16x9 23.976 | Legacy/RED R3D/2K | 2048x1152 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 2K 16x9 24 | Legacy/RED R3D/2K | 2048x1152 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 2K 16x9 25 | Legacy/RED R3D/2K | 2048x1152 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 2K 16x9 29.97 | Legacy/RED R3D/2K | 2048x1152 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 2K 2x1 23.976 | Legacy/RED R3D/2K | 2048x1024 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 2K 2x1 24 | Legacy/RED R3D/2K | 2048x1024 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 2K 2x1 25 | Legacy/RED R3D/2K | 2048x1024 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 2K 2x1 29.97 | Legacy/RED R3D/2K | 2048x1024 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 3K 16x9 23.976 | Legacy/RED R3D/3K | 3072x1728 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 3K 16x9 24 | Legacy/RED R3D/3K | 3072x1728 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 3K 16x9 25 | Legacy/RED R3D/3K | 3072x1728 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 3K 16x9 29.97 | Legacy/RED R3D/3K | 3072x1728 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 3K 2x1 23.976 | Legacy/RED R3D/3K | 3072x1536 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 3K 2x1 24 | Legacy/RED R3D/3K | 3072x1536 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 3K 2x1 25 | Legacy/RED R3D/3K | 3072x1536 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 3K 2x1 29.97 | Legacy/RED R3D/3K | 3072x1536 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4K 16x9 23.976 | Legacy/RED R3D/4K | 4096x2304 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K 16x9 24 | Legacy/RED R3D/4K | 4096x2304 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K 16x9 25 | Legacy/RED R3D/4K | 4096x2304 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K 16x9 29.97 | Legacy/RED R3D/4K | 4096x2304 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4K 2x1 23.976 | Legacy/RED R3D/4K | 4096x2048 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K 2x1 24 | Legacy/RED R3D/4K | 4096x2048 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K 2x1 25 | Legacy/RED R3D/4K | 4096x2048 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K 2x1 29.97 | Legacy/RED R3D/4K | 4096x2048 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4.5K 2.33x1 23.976 | Legacy/RED R3D/4_5K | 4480x1920 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 4.5K 2.33x1 24 | Legacy/RED R3D/4_5K | 4480x1920 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 4.5K 2.33x1 25 | Legacy/RED R3D/4_5K | 4480x1920 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 4.5K 2.33x1 29.97 | Legacy/RED R3D/4_5K | 4480x1920 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 512 16x9 23.976 | Legacy/RED R3D/512 | 512x288 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 512 16x9 24 | Legacy/RED R3D/512 | 512x288 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 512 16x9 25 | Legacy/RED R3D/512 | 512x288 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 512 16x9 29.97 | Legacy/RED R3D/512 | 512x288 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 512 2x1 23.976 | Legacy/RED R3D/512 | 512x256 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 512 2x1 24 | Legacy/RED R3D/512 | 512x256 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 512 2x1 25 | Legacy/RED R3D/512 | 512x256 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 512 2x1 29.97 | Legacy/RED R3D/512 | 512x256 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K 2.4x1 23.976 | Legacy/RED R3D/5K | 5120x2160 | 1:1 | 23.976024 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K 2.4x1 24 | Legacy/RED R3D/5K | 5120x2160 | 1:1 | 24 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K 2.4x1 25 | Legacy/RED R3D/5K | 5120x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 5K 2.4x1 29.97 | Legacy/RED R3D/5K | 5120x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K 2x1 23.976 | Legacy/RED R3D/5K | 5120x2560 | 1:1 | 23.976024 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K 2x1 24 | Legacy/RED R3D/5K | 5120x2560 | 1:1 | 24 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K 2x1 25 | Legacy/RED R3D/5K | 5120x2560 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 5K 2x1 29.97 | Legacy/RED R3D/5K | 5120x2560 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K FF 23.976 | Legacy/RED R3D/5K | 5120x2700 | 1:1 | 23.976024 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K FF 24 | Legacy/RED R3D/5K | 5120x2700 | 1:1 | 24 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 5K FF 25 | Legacy/RED R3D/5K | 5120x2700 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 5K FF 29.97 | Legacy/RED R3D/5K | 5120x2700 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 720p 16x9 23.976 | Legacy/RED R3D/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 720p 16x9 24 | Legacy/RED R3D/720p | 1280x720 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 720p 16x9 25 | Legacy/RED R3D/720p | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 720p 16x9 29.97 | Legacy/RED R3D/720p | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4K HD 16x9 23.976 | Legacy/RED R3D/HD 4K | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K HD 16x9 24 | Legacy/RED R3D/HD 4K | 3840x2160 | 1:1 | 24 | progressive (no fields) | 24 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K HD 16x9 25 | Legacy/RED R3D/HD 4K | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| 4K HD 16x9 29.97 | Legacy/RED R3D/HD 4K | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps non-drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 1920x960 | Legacy/VR/Monoscopic 29.97 | 1920x960 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 1920x960 - Ambisonics | Legacy/VR/Monoscopic 29.97 | 1920x960 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 3840x1920 | Legacy/VR/Monoscopic 29.97 | 3840x1920 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 3840x1920 - Ambisonics | Legacy/VR/Monoscopic 29.97 | 3840x1920 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 4096x2048 | Legacy/VR/Monoscopic 29.97 | 4096x2048 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4096x2048 - Ambisonics | Legacy/VR/Monoscopic 29.97 | 4096x2048 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 8192x4096 | Legacy/VR/Monoscopic 29.97 | 8192x4096 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 8192x4096 - Ambisonics | Legacy/VR/Monoscopic 29.97 | 8192x4096 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 2048x2048 | Legacy/VR/Stereoscopic 29.97 | 2048x2048 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 2048x2048 - Ambisonics | Legacy/VR/Stereoscopic 29.97 | 2048x2048 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 2880x2880 | Legacy/VR/Stereoscopic 29.97 | 2880x2880 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 2880x2880 - Ambisonics | Legacy/VR/Stereoscopic 29.97 | 2880x2880 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 4096x2304 | Legacy/VR/Stereoscopic 29.97 | 4096x2304 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4096x2304 - Ambisonics | Legacy/VR/Stereoscopic 29.97 | 4096x2304 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 4096x4096 | Legacy/VR/Stereoscopic 29.97 | 4096x4096 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 4096x4096 - Ambisonics | Legacy/VR/Stereoscopic 29.97 | 4096x4096 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 6144x6144 | Legacy/VR/Stereoscopic 29.97 | 6144x6144 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 6144x6144 - Ambisonics | Legacy/VR/Stereoscopic 29.97 | 6144x6144 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| 8192x8192 | Legacy/VR/Stereoscopic 29.97 | 8192x8192 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| 8192x8192 - Ambisonics | Legacy/VR/Stereoscopic 29.97 | 8192x8192 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz multichannel / adaptive | 3 | -- |
| XDCAM EX 1080i50 (HQ) | Legacy/XDCAM EX/1080i | 1920x1080 | 1920:1920 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 1080i50 (SP) | Legacy/XDCAM EX/1080i | 1440x1080 | 1920:1440 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 1080i60 (HQ) | Legacy/XDCAM EX/1080i | 1920x1080 | 1920:1920 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 1080i60 (SP) | Legacy/XDCAM EX/1080i | 1440x1080 | 1920:1440 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 1080p24 (HQ) | Legacy/XDCAM EX/1080p | 1920x1080 | 1920:1920 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 1080p25 (HQ) | Legacy/XDCAM EX/1080p | 1920x1080 | 1920:1920 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 1080p30 (HQ) | Legacy/XDCAM EX/1080p | 1920x1080 | 1920:1920 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 720p24 | Legacy/XDCAM EX/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 720p25 | Legacy/XDCAM EX/720p | 1280x720 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 720p30 | Legacy/XDCAM EX/720p | 1280x720 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 720p50 | Legacy/XDCAM EX/720p | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM EX 720p60 | Legacy/XDCAM EX/720p | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD 1080i25 (50i) | Legacy/XDCAM HD/1080i | 1440x1080 | 1920:1440 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD 1080i30 (60i) | Legacy/XDCAM HD/1080i | 1440x1080 | 1920:1440 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD 1080p24 | Legacy/XDCAM HD/1080p | 1440x1080 | 1920:1440 | 23.976024 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD 1080p25 | Legacy/XDCAM HD/1080p | 1440x1080 | 1920:1440 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD 1080p30 | Legacy/XDCAM HD/1080p | 1440x1080 | 1920:1440 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 1080i25 (50i) | Legacy/XDCAM HD422/1080i | 1920x1080 | 1:1 | 25 | upper field first | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 1080i30 (60i) | Legacy/XDCAM HD422/1080i | 1920x1080 | 1:1 | 29.97003 | upper field first | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 1080p24 | Legacy/XDCAM HD422/1080p | 1920x1080 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 1080p25 | Legacy/XDCAM HD422/1080p | 1920x1080 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 1080p30 | Legacy/XDCAM HD422/1080p | 1920x1080 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 720p24 | Legacy/XDCAM HD422/720p | 1280x720 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 720p50 | Legacy/XDCAM HD422/720p | 1280x720 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | -- |
| XDCAM HD422 720p60 | Legacy/XDCAM HD422/720p | 1280x720 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | -- |
| Social Media Portrait 4x5 30 fps | Social | 864x1080 | 1:1 | 30 | progressive (no fields) | 30 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| Social Media Portrait 9x16 30 fps | Social | 1080x1920 | 1:1 | 30 | progressive (no fields) | 30 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| Social Media Square 1x1 30 fps | Social | 1080x1080 | 1:1 | 30 | progressive (no fields) | 30 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| UHD (4K) 2160p 23.976 fps | UHD (4K) | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| UHD (4K) 2160p 25 fps | UHD (4K) | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| UHD (4K) 2160p 29.97 fps | UHD (4K) | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| UHD (4K) 2160p 50 fps | UHD (4K) | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| UHD (4K) 2160p 59.94 fps | UHD (4K) | 3840x2160 | 1:1 | 59.94006 | progressive (no fields) | 59.94 fps drop-frame timecode | 48000 Hz stereo | 3 | BT.709 RGB Full |
| UHD (4K) HDR 2160p 23.976 fps | UHD (4K) HDR | 3840x2160 | 1:1 | 23.976024 | progressive (no fields) | 23.976 fps timecode | 48000 Hz stereo | 3 | BT.2100 HLG RGB Full |
| UHD (4K) HDR 2160p 25 fps | UHD (4K) HDR | 3840x2160 | 1:1 | 25 | progressive (no fields) | 25 fps timecode | 48000 Hz stereo | 3 | BT.2100 HLG RGB Full |
| UHD (4K) HDR 2160p 29.97 fps | UHD (4K) HDR | 3840x2160 | 1:1 | 29.97003 | progressive (no fields) | 29.97 fps drop-frame timecode | 48000 Hz stereo | 3 | BT.2100 HLG RGB Full |
| UHD (4K) HDR 2160p 50 fps | UHD (4K) HDR | 3840x2160 | 1:1 | 50 | progressive (no fields) | 50 fps timecode | 48000 Hz stereo | 3 | BT.2100 HLG RGB Full |
| UHD (4K) HDR 2160p 59.94 fps | UHD (4K) HDR | 3840x2160 | 1:1 | 60 | progressive (no fields) | 60 fps timecode | 48000 Hz stereo | 3 | BT.2100 HLG RGB Full |
### Editing modes

**[STU-VID-017] `StudioEditingMode` is a named constraint set over sequence settings.** An editing
mode declares which frame rates, frame sizes, pixel aspect ratios and field orders a sequence may
take, and which playback and capture devices may service it. A sequence references exactly one
editing mode; `custom` is the mode that declares no constraints. 53 modes are declared, of which 40
are referenced by at least one shipped preset and 166 presets bind to a named mode.

[STU-VID-017a] **The constraint lists themselves are a declared gap, and Studio does not invent
them.** The mode records carry the constraint-list STRUCTURE -- 188 frame-rate slots and 52
frame-rect slots across the 53 modes -- but every slot decoded empty in the capture. Zero of the 53
modes yielded a resolved allowed-frame-rate, allowed-frame-size, allowed-pixel-aspect or
allowed-field-type value. Studio therefore ships the mode set with `constraints_known = false` on
every mode, enforces no constraint until a mode's lists are authored deliberately, and MUST NOT
back-fill constraints by inferring them from the presets that reference the mode, because a preset
demonstrates one legal combination and says nothing about the boundary. This is declared gap [STU-VID-080].
What IS specified is the shape: a mode's constraint list is an explicit allow-list
per axis, and an empty allow-list means unconstrained on that axis, not "nothing allowed".

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Editing mode | Windows GUID | Platforms | frame-rate slots | frame-rect slots | PAR slots | field-type slots |
|---|---|---|---|---|---|---|
| Custom | `9678AF98-A7B7-4bdb-B477-7AC9C8DF4A4E` | All | 12 | 1 | 1 | 0 |
| QuickTime DV NTSC | `0F3C7317-CF5D-44e8-8B19-1BC6D7B05CE6` | Mac, Win | 1 | 1 | 2 | 3 |
| QuickTime DV PAL | `E2E6D6B0-7E3D-4455-BF93-0D9C3C791E4E` | Mac, Win | 1 | 1 | 2 | 3 |
| QuickTime DV 24p | `7429196E-B800-4662-BCE0-8AB2B5C223F1` | Mac, Win | 1 | 1 | 2 | 1 |
| DV NTSC | `0F3C7317-CF5D-44e8-8B19-1BC6D7B05CE6` | Mac, Win | 1 | 1 | 2 | 0 |
| DV PAL | `E2E6D6B0-7E3D-4455-BF93-0D9C3C791E4E` | Mac, Win | 1 | 1 | 2 | 0 |
| DV 24p | `7429196E-B800-4662-BCE0-8AB2B5C223F1` | Mac, Win | 1 | 1 | 2 | 1 |
| HDV 1080i | `` | All | 2 | 1 | 1 | 1 |
| HDV 1080p | `` | All | 3 | 1 | 1 | 1 |
| HDV 720p | `` | All | 3 | 1 | 1 | 1 |
| P2 DVCPRO50 NTSC | `` | All | 2 | 1 | 2 | 2 |
| P2 DVCPRO50 PAL | `` | All | 1 | 1 | 2 | 2 |
| P2 720p 60Hz DVCPROHD | `` | All | 3 | 1 | 1 | 1 |
| P2 720p 50Hz DVCPROHD | `` | All | 2 | 1 | 1 | 1 |
| P2 1080i/1080p 60Hz DVCPROHD | `` | All | 3 | 1 | 1 | 2 |
| P2 1080i/1080p 50Hz DVCPROHD | `` | All | 2 | 1 | 1 | 2 |
| Sony XDCAM EX 1080i (HQ) | `` | All | 2 | 1 | 1 | 1 |
| Sony XDCAM EX 1080p (HQ) | `` | All | 3 | 1 | 1 | 1 |
| Sony XDCAM EX 720p | `` | All | 5 | 1 | 1 | 1 |
| Sony XDCAM EX 1080i (SP) | `` | All | 2 | 1 | 1 | 1 |
| Sony XDCAM HD 1080p | `` | All | 3 | 1 | 1 | 1 |
| AVCHD 1080i square pixel | `` | All | 2 | 1 | 1 | 1 |
| AVCHD 1080i anamorphic | `` | All | 2 | 1 | 1 | 1 |
| AVCHD 1080p square pixel | `` | All | 5 | 1 | 1 | 1 |
| AVCHD 1080p anamorphic | `` | All | 3 | 1 | 1 | 1 |
| AVCHD 720p square pixel | `` | All | 5 | 1 | 1 | 1 |
| AVC-Intra 50 720p | `` | All | 5 | 1 | 1 | 1 |
| AVC-Intra 100 720p | `` | All | 5 | 1 | 1 | 1 |
| AVC-Intra 50 1080i | `` | All | 2 | 1 | 1 | 1 |
| AVC-Intra 100 1080i | `` | All | 2 | 1 | 1 | 1 |
| AVC-Intra 50 1080p | `` | All | 3 | 1 | 1 | 1 |
| AVC-Intra 100 1080p | `` | All | 3 | 1 | 1 | 1 |
| Sony XDCAM HD422 1080 NTSC | `` | All | 3 | 1 | 1 | 2 |
| Sony XDCAM HD422 720p PAL | `` | All | 1 | 1 | 1 | 1 |
| RED Cinema | `D8484CF3-C96C-4622-AB1F-AC1A16E196F9` | All | 5 | 1 | 1 | 0 |
| Canon XF MPEG2 1080i/p | `` | All | 6 | 1 | 1 | 2 |
| Canon XF MPEG2 720p | `` | All | 6 | 1 | 1 | 1 |
| DSLR | `35D109DB-457B-43C1-9452-9CB7BE9F121C` | All | 12 | 1 | 1 | 0 |
| ARRI Cinema | `cc7991f5-c236-4db1-957e-2c71f924e81c` | All | 5 | 1 | 1 | 0 |
| DNX 720p | `` | All | 5 | 1 | 1 | 1 |
| DNX 1080i | `` | All | 2 | 1 | 1 | 1 |
| DNX 1080p | `` | All | 7 | 1 | 1 | 1 |
| Sony XDCAM HD422 1080 PAL | `` | All | 2 | 1 | 1 | 2 |
| Sony XDCAM HD422 720p NTSC | `` | All | 2 | 1 | 1 | 1 |
| Sony XDCAM HD422 1080i/p | `` | All | 5 | 1 | 1 | 2 |
| Sony XDCAM HD422 720p | `` | All | 3 | 1 | 1 | 1 |
| Sony XDCAM HD 1080 PAL | `` | All | 1 | 1 | 1 | 2 |
| Sony XDCAM HD 1080 NTSC | `` | All | 2 | 1 | 1 | 2 |
| DNxHR 2K | `` | All | 8 | 1 | 1 | 1 |
| DNxHR UHD | `` | All | 7 | 1 | 1 | 1 |
| DNxHR 4K | `` | All | 7 | 1 | 1 | 1 |
| ProRes RAW | `5125b1eb-7925-433f-beea-f5d884227812` | All | 8 | 1 | 1 | 0 |
| _(unnamed)_ | `` | -- | 0 | 0 | 0 | 0 |
### Immersive and preview settings

**[STU-VID-018] Immersive video settings are part of the sequence record, not an effect.** The
declared fields are: `projection_type` (integer; 0 in every shipped configuration read),
`stereoscopic_type`, `stereoscopic_eye`, `captured_horizontal_view`, `captured_vertical_view`,
`field_of_horizontal_view` and `field_of_vertical_view` (both declared 108 degrees in the shipped
VR configurations), `ambisonics_monitoring_type`, and an `ambisonics_hrir` reference. Studio carries
all nine. The projection-type and stereoscopic-type enumerations were not recovered as member lists
and are declared gap [STU-VID-082]; the fields are present and typed, and their member sets are
authored deliberately.

**[STU-VID-019] Preview render settings are part of the sequence record.** A sequence declares the
codec and frame size used for rendered timeline previews, separately from its export settings. The
shipped configurations use intra-frame mezzanine codecs for this purpose -- the most common are the
ProRes-family fourccs `apcs` (98 configurations), `apcn` (40) and `apch` (5), and an I-frame-only
MPEG variant (43) -- because a preview codec is optimised for scrub and seek, not for size. Studio's
contract is: the preview format is per-sequence, is independent of the export recipe, and its frame
size MAY differ from the sequence frame size (all shipped configurations read declare them equal).
Which specific codecs Studio's own preview tier ships is an implementation decision constrained
only by [STU-VID-051].

---

## 14.25.2 The clip timeline

### 1. The two timelines are distinct surfaces

[STU-VID-022] **The clip timeline and the keyframe timeline are DIFFERENT surfaces with different
primitives, and conflating them is a specification error.** The distinction is normative:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| | Clip timeline (this sub-section) | Keyframe timeline (14.26) |
|---|---|---|
| Unit of arrangement | `StudioClip` -- a windowed reference to a media item, with source in/out and timeline in/out. | `StudioProperty` -- a value stream on a layer, holding keyframes or an expression. |
| Vertical axis | `StudioTrack` -- an ordered lane holding non-overlapping clips. | The property tree of the selected layers, hierarchically disclosed. |
| Horizontal edit | trim, ripple, roll, slip, slide, razor, lift, extract, insert, overwrite. | keyframe move, interpolation change, tangent edit, time remap. |
| What a drag does | changes which frames of a source play, and when. | changes a value at a time. |
| Time basis | sequence ticks ([STU-VID-012]). | composition time, in seconds and frames ([STU-MOT-020]). |
| Selection | clips and edit points. | keyframes and property ranges. |

**[STU-VID-022a]** The two surfaces MEET at exactly two places and nowhere else. First, a clip carries
an effect stack ([STU-FX-002]) whose parameters are keyframable, so selecting a clip populates a
keyframe timeline scoped to that clip's effects and its intrinsic transform. Second, a composition
([STU-MOT-001]) may be placed as a clip in a sequence, and a sequence may be placed as a layer
source in a composition ([STU-CMP-004]); each is opaque to the other's editing operations and is
re-entered by an explicit navigation command. There is no third coupling, and in particular a
clip's position on a track is NOT a keyframable property.

**[STU-VID-022b]** Both surfaces are dockable panels in the same editor, may be visible simultaneously,
and share one history stream ([STU-VID-040]). They are two views, not two applications, and a
single undo step is whichever operation the operator or model last performed on either.

### 2. Tracks and clips

**[STU-VID-021] `StudioTrack` (schema id `hsk.studio.track@1`).** An ordered lane in a sequence.
Required fields: `track_id` (stable, prefixed `STRK-{uuid_v7}`), `kind`
(`video` | `audio` | `caption` | `submix`), `index` (position within its kind's stack),
`name`, `enabled` (output on/off), `locked`, `sync_locked`, `targeted`, `muted`, `soloed`,
`height`, and for audio tracks the routing fields of [STU-FX-137]. A track holds clips that MUST
NOT overlap in time; overlapping content is expressed by additional tracks, and the compositing
order of video tracks is bottom-to-top ([STU-CMP-010]).

**[STU-VID-020] `StudioClip` (schema id `hsk.studio.clip@1`).** A windowed, retimed reference to a
media item placed on a track. Required fields:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Contract |
|---|---|
| `clip_id` | Stable, prefixed `SCLP-{uuid_v7}`. |
| `source_ref` | Content-addressed reference to the media item, or to a nested `StudioSequence` or `StudioComposition`. Never a filesystem path ([GLOBAL-PORTABILITY] posture). |
| `source_in`, `source_out` | Tick positions in the SOURCE's time base. |
| `timeline_start` | Tick position of the clip's first frame on the track. |
| `duration` | Derived: `(source_out - source_in) / speed`. Stored redundantly for index efficiency, and validated against the derivation. |
| `speed` | Rational multiplier. See [STU-VID-031]. |
| `reversed` | bool. Reversal is a speed sign, and the source in/out do not swap. |
| `enabled` | bool. A disabled clip occupies its time and renders nothing. |
| `link_group` | Optional id joining a video clip and its audio clips as one selectable unit ([STU-VID-029]). |
| `effect_stack` | `StudioEffectStack` ([STU-FX-002]). Includes the clip's intrinsic transform and opacity, which are ordinary keyframable properties, not special cases. |
| `transitions` | Optional head and tail transition references ([STU-VID-032]). |
| `markers` | Clip-local marker list. |
| `label_color` | One of the 16 declared label colours: Violet, Iris, Caribbean, Lavender, Cerulean, Forest, Rose, Mango, Purple, Blue, Teal, Magenta, Tan, Green, Brown, Yellow. |

**[STU-VID-020a]** A clip's `source_in`/`source_out` window is the ONLY thing trimming changes about
its relationship to the source. The source media is never modified, never copied, and never
re-encoded by an edit operation. This is the clip-timeline equivalent of the non-destructive
guarantee [STU-FX-001] makes for effects.

### 3. Monitors, in/out points and assembly

**[STU-VID-023] Two monitors, one contract.** Studio provides a SOURCE monitor showing a media item
or a clip in isolation, and a PROGRAM monitor showing the sequence at the playhead. Both expose:
transport (play, stop, step forward, step back, play in-to-out, loop), an in point and an out
point, playback and paused resolution selection (`full`, `1/2`, `1/4`, `1/8`, `1/16` --
five declared steps for each, independently settable), a zoom selection (`fit`, 10, 25, 50, 75,
100, 150, 200, 400, 800, 1600 percent), safe-margin and guide overlays, a transparency grid, rulers
in pixels or percent, and an output channel selection (`composite`, `alpha`, `red`, `green`,
`blue`, `audio_waveform`, `video_and_audio_split`, `multicam`). The two monitors may be ganged so
one transport drives both.

**[STU-VID-024] Three-point and four-point editing are the normative assembly model.** An edit
takes up to four points -- source in, source out, sequence in, sequence out -- and derives the
missing one. With three points the fourth is computed. With four points where the durations
disagree, Studio MUST offer an explicit fit choice (change clip speed, trim head, trim tail, ignore
sequence out) rather than choosing silently. The two edit modes are normative:

- **Insert**: the incoming clip is placed at the sequence in point and everything at or after that
  point on the affected tracks shifts later by the clip's duration. Sync-locked tracks
  ([STU-VID-029]) shift with it; unlocked tracks do not.
- **Overwrite**: the incoming clip replaces sequence content over its own duration. No downstream
  content moves.

[STU-VID-025] **Track targeting and source patching decide where an edit lands, and they are two
different things.** Source patching maps each of the incoming media's streams (its video stream,
each of its audio streams) to a destination track. Track targeting marks which tracks respond to
timeline-relative operations such as paste, match-frame and edit navigation. Both are per-track
toggles, both are addressable by typed command per track index, and Studio ships the declared
convenience operations over them: toggle all sources, toggle all targets, move all sources up or
down, move all targets up or down, set a source to a gap (so a stream is skipped), add tracks to
match the source's stream count, and save and recall named source-assignment presets.

### 4. Trim operations

**[STU-VID-026] The normative trim vocabulary.** Each is a typed command on `event_family =
studio.sequence`, each is one reversible history step, and each has a defined behaviour when it
would produce an illegal result.

*Derivation: catalogue table, splits per row; yields 9 microtasks, one per trim operation.*

| Operation | Definition | Illegal-result behaviour |
|---|---|---|
| Trim in | Move a clip's head. Sequence duration unchanged; a gap opens or the clip shortens into empty space. | Refuses past `source_in = 0` or past the clip's own tail. |
| Trim out | Move a clip's tail. Same properties. | Refuses past source end or past the head. |
| Ripple trim in / out | Move a clip's head or tail and shift all later content on sync-locked tracks by the same amount, so no gap opens. | Refuses when a sync-locked track cannot absorb the shift. |
| Roll | Move the edit point BETWEEN two adjacent clips: one lengthens by exactly what the other shortens. Sequence duration unchanged. | Refuses when either side would exceed its available source. |
| Slip | Change a clip's `source_in` and `source_out` by the same delta, keeping `timeline_start` and `duration` fixed. Changes WHICH frames play, not when. | Refuses when either bound would leave the source. |
| Slide | Move a clip along the timeline, lengthening the previous clip and shortening the next (or vice versa) by the same amount. The slid clip's source window is unchanged. | Refuses when a neighbour would exceed its available source. |
| Extend edit to playhead | Roll the nearest edit on targeted tracks to the playhead. | Refuses when the roll is not available. |
| Rate stretch | Change a clip's `duration` by changing its `speed`, keeping the full source window. | Bounded by the speed limits of [STU-VID-031]. |
| Trim forward / backward by one or by many | Nudge the current trim by one frame or by the configured multi-frame step. | Clamps to the operation's own limits above. |

**[STU-VID-026a] Trim has a modal surface as well as a direct one.** A trim session may be entered
on a selected edit point, showing the outgoing and incoming frames side by side, with plus and minus
adjustments in both a minor (one frame) and a major (multi-frame) step, an in-only, out-only or
both-sides focus, a composite preview during the trim, and an explicit rollback that abandons the
session. The trim type toggles between ripple and roll on the selected edit. Every one of these is
reachable as a typed command and therefore headlessly ([STU-VID-090]).

**[STU-VID-026b] Selecting an edit point is itself a typed operation with a stated mode**: select
the nearest edit as a trim-in, as a trim-out, as a ripple-in, as a ripple-out, or as a roll. The
mode is part of the selection, not a modifier applied later, so a model can express "roll the next
edit by 12 frames" as two deterministic commands.

### 5. Cutting, removing and closing

**[STU-VID-027] Razor / add edit.** Splits the clip under the playhead into two clips sharing the
source, with the second's `source_in` continuing where the first's `source_out` ended, so the pair
renders identically to the original. An "add edit to all tracks" variant applies to every targeted
track at once. The inverse, "join through edits", removes an edit whose two sides are contiguous in
the same source and restores one clip; a "show through edits" display state marks such edits so
they can be found.

**[STU-VID-028] Removal has two forms and they are not interchangeable.**

- **Lift**: removes the selection (or the in-to-out range on targeted tracks) and leaves a gap.
  Sequence duration unchanged.
- **Extract**: removes it and closes the gap, shifting later content on sync-locked tracks earlier.
  Sequence duration shortens.

`Ripple delete` is extract applied to a selection. `Close gap` finds and removes an empty span;
navigation commands exist for next and previous gap, both within a track and across the sequence.

### 6. Linking, grouping and sync

**[STU-VID-029] Three distinct relationships, deliberately not merged.**

- **Link**: a video clip and the audio clips captured with it form one `link_group`. Selecting one
  selects all; trimming one trims all. Unlink dissolves the group. A linked-selection master toggle
  suspends the behaviour without dissolving the groups.
- **Group**: an arbitrary set of clips selected and moved together, with no sync implication.
- **Sync lock**: a per-track flag deciding whether a track participates in the shift caused by an
  insert, an extract or a ripple trim on another track. Sync lock is what keeps parallel tracks in
  step; it is NOT a selection relationship, and merging it with link or group would break both.

**[STU-VID-029a]** A `merge clips` operation combines separately-recorded video and audio into one
linked unit; a `synchronize` operation aligns selected clips by in point, out point, timecode,
marker or audio waveform. Both produce ordinary linked clips, not a new primitive.

### 7. Nesting and subsequences

**[STU-VID-030] A sequence may be a clip.** `Nest` replaces a selection with a single clip whose
source is a newly created sequence containing that selection. `Make subsequence` creates a sequence
from a selection or from the in-to-out range without replacing it. A nested sequence clip is opaque
to timeline operations -- trimming it trims the window onto the nested sequence, not the clips
inside -- and is re-entered by an explicit reveal command that opens it in the timeline. Nesting is
recursive with no fixed depth limit; a cycle is a validation error.

**[STU-VID-030a]** Two nesting-adjacent behaviours are declared because they change results: a nested
sequence MAY be set to preserve its own frame rate and its own resolution rather than inheriting the
parent's, and multicam clips MAY be set to follow or ignore the nest setting. Both are explicit
booleans on the nested clip, never implicit.

### 8. Speed, retiming and frame interpolation

**[STU-VID-031] `speed` is a rational multiplier and retiming is a stated algorithm choice.** A
clip's speed and duration are two views of one value: fixing either derives the other. Reverse is a
negative speed. `Maintain audio pitch` is an independent boolean. The frame-interpolation choice
governs what happens between source frames and is normative:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Method | Behaviour |
|---|---|
| `frame_sampling` | Nearest source frame. Cheapest, judders. |
| `frame_blending` | Weighted blend of the two neighbouring source frames. |
| `optical_flow` | Motion-estimated synthesis of intermediate frames. Highest cost; may produce artefacts on occlusion boundaries and MUST be marked as an estimation in any diagnostic that compares it to a source frame. |

**[STU-VID-031a]** `Frame hold` freezes a clip at a chosen source frame -- at the playhead, at a clip
marker, at the in point, or at the out point -- as an explicit property, and `insert frame hold
segment` splits out a held span as its own clip. `Time remapping` on a clip is the keyframed form
of speed: the clip's speed becomes a keyframable property evaluated on the keyframe timeline
([STU-MOT-045]), which is the one place where a clip property crosses into the keyframe surface,
and it does so as an ordinary property, not as a special case.

### 9. Transitions

[STU-VID-032] **A transition is a timed blend applied AT an edit point, and it is a distinct
primitive from an effect.** `StudioTransition` (schema id `hsk.studio.transition@1`) carries:
`transition_kind` (a `filter_kind` in the `transition` category of [STU-FX-126]), `duration`
(ticks), `alignment` (`centered_on_cut` | `start_at_cut` | `end_at_cut` | `custom`),
`center_offset`, and a typed parameter set obeying 14.9.1 in full. A transition requires HANDLES --
source frames beyond the trimmed in or out point -- and when insufficient handle exists Studio MUST
report the shortfall and offer the declared choices (shorten the transition, shift the alignment,
repeat the end frame) rather than silently repeating frames.

**[STU-VID-032a]** Studio ships a default video transition and a default audio transition, both
operator-configurable, with a configurable default duration, and applies them by typed command:
apply default to selection, apply video transition, apply audio transition, apply from the playhead,
apply to the playhead. A cross-dissolve and an audio crossfade are the shipped defaults. The
transition catalogue itself is the `transition` category of 14.9, deduped there rather than
redefined here ([STU-FX-127]).

### 10. Multicam

**[STU-VID-033] Multicam is a clip mode, not a separate editor.** A multicam source sequence holds
N synchronised angles; a clip referencing it exposes an `active_angle` that is switchable over
time, producing a cut list. The normative contract:

- Angle count is at least 16 in the addressable command surface; Studio imposes no fixed maximum
  beyond a document performance budget.
- Two switch modes: switch WITH a cut (creates an edit at the playhead and changes the angle
  after it) and switch WITHOUT a cut (changes the angle for the whole clip).
- A record mode captures live angle switches during playback into a cut list; toggling record is a
  single command.
- A preview surface shows all angles in a paged grid with first/last/next/previous page navigation,
  next/previous angle, and next/previous edit navigation.
- `audio_follows_video` is a boolean: when true an angle switch also switches the audio.
- `flatten` bakes the current cut list into ordinary clips and discards the multicam mode; it is
  one reversible history step.
- A selection-order option (top-down versus source order) decides angle numbering.
- Playback quality may auto-decimate under load; whether it does is an explicit toggle, never
  automatic without the toggle.

### 11. Markers, snapping, playhead and work area

**[STU-VID-034] Markers exist at three scopes and they are not the same object.** A CLIP marker
travels with the clip and moves when the clip moves. A SEQUENCE marker is positioned in sequence
time and does not move with clips. A PROJECT-ITEM marker lives on a media item and appears on every
clip cut from it. Each carries a name, a comment, a colour, a duration (a marker may be a span, not
only a point), and an optional typed payload for chapter, web-link, cue-point and caption roles.

**[STU-VID-035] Snapping, the playhead and the work area.** Snapping is a toggleable magnet that
attracts dragged edges and the playhead to edit points, markers, the work-area bounds and the
sequence start. The playhead is addressable by typed command (to a timecode, to the next or
previous edit on targeted tracks, to the next or previous edit on any track, to the next or
previous selected edit, to a marker, to the in or out point) and its position is part of the
sequence's persisted state. The work area is a named in/out span used to scope rendering and export;
setting its bounds and rendering within it are typed commands.

**[STU-VID-035a]** Nudging is normative and quantised: a selected clip nudges left or right by one
frame or by the configured multi-frame step, and up or down by one track. Slip and slide have their
own one-and-many nudge pairs. The multi-frame step is a single preference shared by every "many"
variant so that one setting governs the whole surface.

### 12. Track display, captions, proxies and previews

**[STU-VID-036] Track display state is persisted per sequence and is model-visible.** The declared
display switches are: video thumbnails (off, head only, head and tail, continuous), clip names,
FX badges, proxy badges, content-credential badges, duplicate-frame markers, clip markers, source
clip name and label, through-edits, audio waveforms (with logarithmic or dynamic scaling, and a
rectified option), audio names, audio channel labels, audio type badges, audio and video fade
handles, audio and video keyframes, and audio clip headers on small tracks. Track heights are
adjustable per kind with save-and-recall named presets, plus expand-all and minimize-all.

**[STU-VID-037] Captions are a track kind, not an overlay effect.** A caption track holds caption
segments with text, timing and styling; multiple caption tracks coexist for multiple languages;
one is active at a time for display; navigation, show-all, hide-all and show-active-only are typed
commands; and a validation-warning display state marks segments that violate the track's declared
caption constraints. Caption tracks participate in export ([STU-VID-061], facet `captions`).

**[STU-VID-038] Proxies and offline media are first-class states, not error states.** A media item
MAY carry an attached proxy at reduced resolution; a per-project toggle switches all playback
between full and proxy; badges show which is in use. A media item MAY be OFFLINE -- referenced but
unavailable -- and an offline clip renders a determinate offline placeholder, keeps every edit
decision intact, and relinks by content hash or by a located path without any edit being lost.
`Make offline` and `Link media` are typed commands. Proxy creation and full-resolution reconnection
are queued background operations subject to the headless and quiet law ([STU-FX-038]).

**[STU-VID-039] Timeline preview rendering is explicit and scoped.** Render the work area, render
the entire work area, render a selection, or render audio only; delete render files, or delete only
the work area's render files; restore unrendered. Render state is visible per span on the timeline
as a three-state indicator (rendered, will play unrendered, requires render). A `render and replace`
operation bakes a clip's effect stack into a new media item and swaps the clip to it, retaining a
`restore unrendered` inverse -- the clip-timeline analogue of [STU-FX-003]'s bake, and reversible on
the same terms.

### 13. History

**[STU-VID-040] Every operation in this sub-section is one reversible `StudioHistoryEntry`**
(14.19). A trim is one step, not one step per frame dragged. A multi-track ripple is one step. A
multicam record pass is one step. A model-authored sequence mutation traverses sandbox ->
`StudioValidationDescriptor` -> `PromotionGate` exactly as every other Studio mutation does
([STU-ARC-005]); there is no fast path for timeline edits.

### 14. The command surface

**[STU-VID-041] The tables below are the normative Studio timeline command vocabulary.** Every row
is a typed, model-invokable, parallel-safe, deterministic command per [STU-CON-007]. The command
identifiers are given in their recovered form as import keys and as a completeness check on this
sub-section; Studio's own command ids are Handshake-native and namespaced
`STUDIO_SEQUENCE_*` / `STUDIO_TIMELINE_*` / `STUDIO_CLIP_*` per [STU-ARC-003]. A row whose label is
absent is a command with no menu presence in the source; it is still a real operation and is still
in scope, and several of the most important trim operations are in exactly that category. Default
key bindings are recorded because they are a real usability contract, not decoration: an editor's
speed comes from the keyboard, and a video surface without a keyboard-first edit vocabulary is not
a professional one.

**[STU-VID-041a] Two command labels named a vendor product and have been renamed to name the
capability instead.** [STU-SECTION-003] forbids a source product name as a Studio command, panel or
manual name, and a label is exactly such a name. The behaviour, the command identity and the binding
are unchanged; only the operator-facing label changed. The captured label is preserved here as
provenance so an importer and a shortcut-migration path can still match on it:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Command id | Studio label | Captured source label (provenance) |
|---|---|---|
| `cmd.clip.aeify` | Replace With Composition | `Replace With After Effects Composition` |
| `cmd.clip.openstockaudiosearch` | Search Stock Audio Library | `Find Adobe Stock Audio` |

Neither rename creates or removes a capability. `Replace With Composition` is the sequence-side
entry to the composition/sequence interoperation of [STU-CMP-004] and produces an ordinary nested
composition clip. `Search Stock Audio Library` is an asset-library search; per [STU-FX-032]'s
posture and [STU-OVR-002] no stock or account-bound service is native behaviour, so the command
resolves against whatever asset library is bound and MUST NOT require an account or a network.


**`timeline` namespace** (127 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.timeline.activate.next.caption.track` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.activate.prev.caption.track` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.audio.clip.keyframes.show` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.audio.show.audioheaderonsmalltracks` | Show Audio Clip Header on Small Tracks | -- |
| `cmd.timeline.audio.show.channel.labels` | Show Audio Channel Labels | -- |
| `cmd.timeline.audio.show.esp.badges` | Show Audio Type Badges | -- |
| `cmd.timeline.audio.show.fadehandles` | Show Audio Fade Handles | -- |
| `cmd.timeline.audio.show.keyframes` | Show Audio Keyframes | -- |
| `cmd.timeline.audio.show.names` | Show Audio Names | -- |
| `cmd.timeline.audio.show.waveform` | Show Audio Waveform | -- |
| `cmd.timeline.audio.track.keyframes.show` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.caption.opentextpanel` | Open Text panel | -- |
| `cmd.timeline.caption.show.warning` | Show Caption Warning | -- |
| `cmd.timeline.composite.preview.during.trim` | Composite Preview During Trim | -- |
| `cmd.timeline.customize.audio.header` | Customize Audio Header... | -- |
| `cmd.timeline.customize.video.header` | Customize Video Header... | -- |
| `cmd.timeline.decrease.audio.tracks.height` | _(no menu label; executable-only)_ | `Alt+-` |
| `cmd.timeline.decrease.video.tracks.height` | _(no menu label; executable-only)_ | `Ctrl+-` |
| `cmd.timeline.default.source.assignment` | Default Source Assignment | -- |
| `cmd.timeline.editpoint.rippletrimin` | Ripple Trim In | -- |
| `cmd.timeline.editpoint.rippletrimout` | Ripple Trim Out | -- |
| `cmd.timeline.editpoint.rolledit` | Roll Edit | -- |
| `cmd.timeline.editpoint.trimin` | Trim In | -- |
| `cmd.timeline.editpoint.trimout` | Trim Out | -- |
| `cmd.timeline.expand.all.tracks` | Expand All Tracks | `Shift+=` |
| `cmd.timeline.goto.next.caption.trackitem` | Go to &Next Caption Segment | `Ctrl+Alt+Down Arrow` |
| `cmd.timeline.goto.prev.caption.trackitem` | Go to &Previous Caption Segment | `Ctrl+Alt+Up Arrow` |
| `cmd.timeline.hide.all.caption.tracks` | &Hide All Caption Tracks | -- |
| `cmd.timeline.increase.audio.tracks.height` | _(no menu label; executable-only)_ | `Alt+=` |
| `cmd.timeline.increase.video.tracks.height` | _(no menu label; executable-only)_ | `Ctrl+=` |
| `cmd.timeline.interpolation.bezier` | Bezier | -- |
| `cmd.timeline.interpolation.bezier.auto` | Auto Bezier | -- |
| `cmd.timeline.interpolation.bezier.continuous` | Continuous Bezier | -- |
| `cmd.timeline.interpolation.delete` | Delete | -- |
| `cmd.timeline.interpolation.easein` | Ease In | -- |
| `cmd.timeline.interpolation.easeout` | Ease Out | -- |
| `cmd.timeline.interpolation.hold` | Hold | -- |
| `cmd.timeline.interpolation.linear` | Linear | -- |
| `cmd.timeline.manage.source.assignment.presets` | Manage Presets... | -- |
| `cmd.timeline.manage.track.height.presets` | Manage Presets... | -- |
| `cmd.timeline.minimize.all.tracks` | Minimize All Tracks | `Shift+-` |
| `cmd.timeline.move.cti.to.cursor` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.nudge.down` | _(no menu label; executable-only)_ | `Alt+Down Arrow` |
| `cmd.timeline.nudge.left.one` | _(no menu label; executable-only)_ | `Alt+Left Arrow` |
| `cmd.timeline.nudge.left.several` | _(no menu label; executable-only)_ | `Alt+Shift+Left Arrow` |
| `cmd.timeline.nudge.right.one` | _(no menu label; executable-only)_ | `Alt+Right Arrow` |
| `cmd.timeline.nudge.right.several` | _(no menu label; executable-only)_ | `Alt+Shift+Right Arrow` |
| `cmd.timeline.nudge.up` | _(no menu label; executable-only)_ | `Alt+Up Arrow` |
| `cmd.timeline.paste.to.same.track` | _(no menu label; executable-only)_ | `Ctrl+V` |
| `cmd.timeline.pasteinsert.to.same.track` | _(no menu label; executable-only)_ | `Ctrl+Shift+V` |
| `cmd.timeline.ripple.delete` | _(no menu label; executable-only)_ | `Alt+Backspace` |
| `cmd.timeline.save.source.assignment.preset` | Save Preset... | -- |
| `cmd.timeline.save.track.height.preset` | Save Preset... | -- |
| `cmd.timeline.sequence.audiounits` | Show Audio Time Units | -- |
| `cmd.timeline.sequence.createpreset` | Create Preset from Sequence... | -- |
| `cmd.timeline.sequence.labelcolor` | Show Sequence Label Color | -- |
| `cmd.timeline.sequence.logwaveformscaling` | Logarithmic Waveform Scaling | -- |
| `cmd.timeline.sequence.rectifiedwaveforms` | Rectified Audio Waveforms | -- |
| `cmd.timeline.sequence.revealinproject` | Reveal Sequence in Project | -- |
| `cmd.timeline.sequence.showworkarea` | Work Area Bar | -- |
| `cmd.timeline.sequence.volumewaveformscaling` | Dynamic Audio &Waveforms | -- |
| `cmd.timeline.sequence.waveformsuselabel` | Audio Waveforms Use Label Color | -- |
| `cmd.timeline.sequence.zeropoint` | Start Time... | -- |
| `cmd.timeline.setttransitionduration` | Set Transition Duration... | -- |
| `cmd.timeline.show.active.caption.track.only` | Show &Active Caption Tracks Only | -- |
| `cmd.timeline.show.aigenerated.labels` | Show AI-generated Labels | -- |
| `cmd.timeline.show.all.caption.tracks` | &Show All Caption Tracks | -- |
| `cmd.timeline.show.content.credentials.badges` | Show Content Credentials Badges | -- |
| `cmd.timeline.show.direct.clip.manipulation` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.show.duplicate.frames` | Show Duplicate Frame Markers | -- |
| `cmd.timeline.show.next.screen` | _(no menu label; executable-only)_ | `Page Down` |
| `cmd.timeline.show.previous.screen` | _(no menu label; executable-only)_ | `Page Up` |
| `cmd.timeline.show.proxy.badges` | Show Proxy Badges | -- |
| `cmd.timeline.show.sourceclip.name.label` | Show Source Clip Name and Label | -- |
| `cmd.timeline.slide.left.one` | _(no menu label; executable-only)_ | `Alt+,` |
| `cmd.timeline.slide.left.several` | _(no menu label; executable-only)_ | `Alt+Shift+,` |
| `cmd.timeline.slide.right.one` | _(no menu label; executable-only)_ | `Alt+.` |
| `cmd.timeline.slide.right.several` | _(no menu label; executable-only)_ | `Alt+Shift+.` |
| `cmd.timeline.slip.left.one` | _(no menu label; executable-only)_ | `Ctrl+Alt+Left Arrow` |
| `cmd.timeline.slip.left.several` | _(no menu label; executable-only)_ | `Ctrl+Alt+Shift+Left Arrow` |
| `cmd.timeline.slip.right.one` | _(no menu label; executable-only)_ | `Ctrl+Alt+Right Arrow` |
| `cmd.timeline.slip.right.several` | _(no menu label; executable-only)_ | `Ctrl+Alt+Shift+Right Arrow` |
| `cmd.timeline.source.assignment.preset.0` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.1` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.2` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.3` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.4` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.5` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.6` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.7` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.8` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.source.assignment.preset.9` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.togglelockaudiotracks` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.togglelockvideotracks` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.0` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.1` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.2` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.3` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.4` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.5` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.6` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.7` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.8` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.track.height.preset.9` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.trackitem.linkmedia` | Link Media... | -- |
| `cmd.timeline.trackitem.unlinkmedia` | Make Offline... | -- |
| `cmd.timeline.transition.apply.audio.crossfade` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.default.audio.from.playhead` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.default.audio.to.playhead` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.default.video.from.playhead` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.default.video.to.playhead` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.video.crossfade` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.video.diptowhite` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.transition.apply.video.wipe` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.video.keyframes.hide` | Hide Keyframes | -- |
| `cmd.timeline.video.keyframes.show` | Show Keyframes | -- |
| `cmd.timeline.video.show.fadehandles` | Show Video Fade Handles | -- |
| `cmd.timeline.video.show.keyframes` | Show Video Keyframes | -- |
| `cmd.timeline.video.show.names` | Show Video Names | -- |
| `cmd.timeline.video.show.thumbnails` | Show Video Thumbnails | -- |
| `cmd.timeline.video.style.frames` | Continuous Video Thumbnails | -- |
| `cmd.timeline.video.style.head` | Video Head Thumbnails | -- |
| `cmd.timeline.video.style.headandtail` | Video Head and Tail Thumbnails | -- |
| `cmd.timeline.video.style.showmarkers` | Show Clip Markers | -- |
| `cmd.timeline.voiceover.track.record` | _(no menu label; executable-only)_ | -- |
| `cmd.timeline.workbar.set.in` | _(no menu label; executable-only)_ | `Alt+[` |
| `cmd.timeline.workbar.set.out` | _(no menu label; executable-only)_ | `Alt+]` |

**`trim` namespace** (17 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.trim.focusin` | _(no menu label; executable-only)_ | -- |
| `cmd.trim.focusinandout` | _(no menu label; executable-only)_ | -- |
| `cmd.trim.focusout` | _(no menu label; executable-only)_ | -- |
| `cmd.trim.minus.major` | _(no menu label; executable-only)_ | -- |
| `cmd.trim.minus.minor` | _(no menu label; executable-only)_ | -- |
| `cmd.trim.monitor.paused.resolution.eighth` | 1/8 | -- |
| `cmd.trim.monitor.paused.resolution.full` | Full | -- |
| `cmd.trim.monitor.paused.resolution.half` | 1/2 | -- |
| `cmd.trim.monitor.paused.resolution.quarter` | 1/4 | -- |
| `cmd.trim.monitor.paused.resolution.sixteenth` | 1/16 | -- |
| `cmd.trim.monitor.playback.resolution.eighth` | 1/8 | -- |
| `cmd.trim.monitor.playback.resolution.full` | Full | -- |
| `cmd.trim.monitor.playback.resolution.half` | 1/2 | -- |
| `cmd.trim.monitor.playback.resolution.quarter` | 1/4 | -- |
| `cmd.trim.monitor.playback.resolution.sixteenth` | 1/16 | -- |
| `cmd.trim.plus.major` | _(no menu label; executable-only)_ | -- |
| `cmd.trim.plus.minor` | _(no menu label; executable-only)_ | -- |

**`tlnav` namespace** (56 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.tlnav.add.tracks.to.match.source` | Add Tracks to Match Source | -- |
| `cmd.tlnav.assign.black` | Gap | -- |
| `cmd.tlnav.go.to.next.selected.edit` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.go.to.prev.selected.edit` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.move.all.source.audio.down` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.move.all.source.audio.up` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.move.all.source.video.down` | Move All Sources Down | -- |
| `cmd.tlnav.move.all.source.video.up` | Move All Sources Up | -- |
| `cmd.tlnav.move.all.target.audio.down` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.move.all.target.audio.up` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.move.all.target.video.down` | Move All Targets Down | -- |
| `cmd.tlnav.move.all.target.video.up` | Move All Targets Up | -- |
| `cmd.tlnav.next.edit` | _(no menu label; executable-only)_ | `Down Arrow` |
| `cmd.tlnav.next.edit.any.track` | _(no menu label; executable-only)_ | `Shift+Down Arrow` |
| `cmd.tlnav.prev.edit` | _(no menu label; executable-only)_ | `Up Arrow` |
| `cmd.tlnav.prev.edit.any.track` | _(no menu label; executable-only)_ | `Shift+Up Arrow` |
| `cmd.tlnav.reveal.nested.sequence` | _(no menu label; executable-only)_ | `Ctrl+Alt+F` |
| `cmd.tlnav.select.clip.at.playhead` | _(no menu label; executable-only)_ | `D` |
| `cmd.tlnav.select.in.to.out` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.select.nearest.edit.as.ripple.in` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.select.nearest.edit.as.ripple.out` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.select.nearest.edit.as.roll` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.select.nearest.edit.as.trim.in` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.select.nearest.edit.as.trim.out` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.select.next.clip` | _(no menu label; executable-only)_ | `Ctrl+Down Arrow` |
| `cmd.tlnav.select.previous.clip` | _(no menu label; executable-only)_ | `Ctrl+Up Arrow` |
| `cmd.tlnav.targets.snap.to.edits` | Targets Follow Inserts and Overwrites | -- |
| `cmd.tlnav.toggle.all.source.audio` | _(no menu label; executable-only)_ | `Ctrl+Alt+9` |
| `cmd.tlnav.toggle.all.source.audio.silent` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.all.source.caption` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.all.source.caption.black` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.all.source.video` | Toggle All Sources | `Ctrl+Alt+0` |
| `cmd.tlnav.toggle.all.source.video.black` | Set All Sources To Gaps | -- |
| `cmd.tlnav.toggle.all.target.audio` | _(no menu label; executable-only)_ | `Ctrl+9` |
| `cmd.tlnav.toggle.all.target.video` | Toggle All Targets | `Ctrl+0` |
| `cmd.tlnav.toggle.target.audio.1` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.2` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.3` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.4` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.5` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.6` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.7` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.audio.8` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.1` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.2` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.3` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.4` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.5` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.6` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.7` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.target.video.8` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.toggle.timeerulernumbers` | Time Ruler Numbers | -- |
| `cmd.tlnav.trim.in.to.cti` | _(no menu label; executable-only)_ | `Ctrl+Alt+Q` |
| `cmd.tlnav.trim.out.to.cti` | _(no menu label; executable-only)_ | `Ctrl+Alt+W` |
| `cmd.tlnav.zoomto.frame` | _(no menu label; executable-only)_ | -- |
| `cmd.tlnav.zoomto.sequence` | _(no menu label; executable-only)_ | `\` |

**`sequence` namespace** (93 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.sequence.addaudiosubmixtrack` | Add Audio Submix Track | -- |
| `cmd.sequence.addtrack` | Add Track | -- |
| `cmd.sequence.addtracks` | Add &Tracks... | -- |
| `cmd.sequence.addvideotrack` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.applydefaultaudiotransition` | Apply &Audio Transition | `Ctrl+Shift+D` |
| `cmd.sequence.applydefaulttransitions` | Apply &Default Transitions to Selection | `Shift+D` |
| `cmd.sequence.applydefaultvideotransition` | Apply &Video Transition | `Ctrl+D` |
| `cmd.sequence.audiotrackoutputassignments` | Track Output Channel Assignments... | -- |
| `cmd.sequence.autoframesequence` | Auto Reframe Sequence... | -- |
| `cmd.sequence.caption.translatecaptions` | Translate captions... | -- |
| `cmd.sequence.captiontracksettings` | Track Settings... | -- |
| `cmd.sequence.close.gaps` | &Close Gap | -- |
| `cmd.sequence.copytrackeffects` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.customizetrackheaders` | Customize... | -- |
| `cmd.sequence.decreaseclipvolume` | _(no menu label; executable-only)_ | `[` |
| `cmd.sequence.decreaseclipvolumemany` | _(no menu label; executable-only)_ | `Shift+[` |
| `cmd.sequence.deletetrack` | Delete Track | -- |
| `cmd.sequence.deletetracks` | &Delete Tracks... | -- |
| `cmd.sequence.deletetracks.empty` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.deletevideopreviews` | &Delete Render Files | -- |
| `cmd.sequence.deleteworkareavideopreviews` | Delete &Work Area Render Files | -- |
| `cmd.sequence.edit.label.0` | Violet | -- |
| `cmd.sequence.edit.label.1` | Iris | -- |
| `cmd.sequence.edit.label.10` | Teal | -- |
| `cmd.sequence.edit.label.11` | Magenta | -- |
| `cmd.sequence.edit.label.12` | Tan | -- |
| `cmd.sequence.edit.label.13` | Green | -- |
| `cmd.sequence.edit.label.14` | Brown | -- |
| `cmd.sequence.edit.label.15` | Yellow | -- |
| `cmd.sequence.edit.label.2` | Caribbean | -- |
| `cmd.sequence.edit.label.3` | Lavender | -- |
| `cmd.sequence.edit.label.4` | Cerulean | -- |
| `cmd.sequence.edit.label.5` | Forest | -- |
| `cmd.sequence.edit.label.6` | Rose | -- |
| `cmd.sequence.edit.label.7` | Mango | -- |
| `cmd.sequence.edit.label.8` | Purple | -- |
| `cmd.sequence.edit.label.9` | Blue | -- |
| `cmd.sequence.extendnextedittoplayhead` | _(no menu label; executable-only)_ | `Shift+W` |
| `cmd.sequence.extendpreviousedittoplayhead` | _(no menu label; executable-only)_ | `Shift+Q` |
| `cmd.sequence.extendselectededittoplayhead` | E&xtend Selected Edit to Playhead | `E` |
| `cmd.sequence.extract` | Extract | `'` |
| `cmd.sequence.findnextsequencegap` | &Next in Sequence | `Shift+;` |
| `cmd.sequence.findnexttrackgap` | Next in &Track | -- |
| `cmd.sequence.findprevioussequencegap` | &Previous in Sequence | `Ctrl+Shift+;` |
| `cmd.sequence.findprevioustrackgap` | Previous in Trac&k | -- |
| `cmd.sequence.generatecaptions` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.increaseclipvolume` | _(no menu label; executable-only)_ | `]` |
| `cmd.sequence.increaseclipvolumemany` | _(no menu label; executable-only)_ | `Shift+]` |
| `cmd.sequence.joinallthroughedits` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.jointhroughedits` | Join Through Edits | -- |
| `cmd.sequence.lift` | Lift | `;` |
| `cmd.sequence.linkedselection` | &Linked Selection | -- |
| `cmd.sequence.makesubsequence` | Make Subsequence | `Shift+U` |
| `cmd.sequence.makesubsequencefromintoout` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.matchframe` | Match &Frame | `F` |
| `cmd.sequence.nestsourcesequence` | Nest Source Sequence | -- |
| `cmd.sequence.newsequencefromselection` | New Sequence From Clip | -- |
| `cmd.sequence.normalizetrack` | &Normalize Mix Track... | -- |
| `cmd.sequence.oneframeoffrippletrimpreviousedittoplayhead` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.pastetrackeffects` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.preview` | &Render Effects in Work Area | `Return / Enter` |
| `cmd.sequence.previewaudio` | Render &Audio | -- |
| `cmd.sequence.previewselection` | Render &Selection | -- |
| `cmd.sequence.previewyellow` | Render &Entire Work Area | -- |
| `cmd.sequence.razorateditline` | &Add Edit | `Ctrl+K` |
| `cmd.sequence.razorateditline.all` | Add Edit to All &Tracks | `Ctrl+Shift+K` |
| `cmd.sequence.renameaudiotrack` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.renamevideotrack` | Rename | -- |
| `cmd.sequence.reversematchframe` | Reverse Matc&h Frame | `Shift+R` |
| `cmd.sequence.rippletrimnextedittoplayhead` | _(no menu label; executable-only)_ | `W` |
| `cmd.sequence.rippletrimpreviousedittoplayhead` | _(no menu label; executable-only)_ | `Q` |
| `cmd.sequence.selectionfollowsplayhead` | Selection &Follows Playhead | -- |
| `cmd.sequence.sequencesettingsgeneral` | Sequence Settings... | -- |
| `cmd.sequence.setpancenter` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.setpanleft` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.setpanright` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.showfxbadges` | Show FX Badges | -- |
| `cmd.sequence.showthroughedits` | Show Through Edits | -- |
| `cmd.sequence.simplifysequence` | Simplify Sequence... | -- |
| `cmd.sequence.snap` | &Snap in Timeline | `S` |
| `cmd.sequence.splitclip` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.toggleaudiotrackmutes` | Toggle Mute for All Targeted Audio Tracks | -- |
| `cmd.sequence.toggleaudiotracksolos` | Toggle Solo for All Targeted Audio Tracks | -- |
| `cmd.sequence.toggletrimtype` | _(no menu label; executable-only)_ | `Ctrl+Shift+T` |
| `cmd.sequence.togglevideotrackoutputs` | Toggle Track Output for All Targeted Video Tracks | -- |
| `cmd.sequence.transcribeasset` | &Transcribe Sequence... | -- |
| `cmd.sequence.trim.restore` | _(no menu label; executable-only)_ | -- |
| `cmd.sequence.trimbackward` | _(no menu label; executable-only)_ | `Ctrl+Left Arrow` |
| `cmd.sequence.trimbackwardmany` | _(no menu label; executable-only)_ | `Ctrl+Shift+Left Arrow` |
| `cmd.sequence.trimforward` | _(no menu label; executable-only)_ | `Ctrl+Right Arrow` |
| `cmd.sequence.trimforwardmany` | _(no menu label; executable-only)_ | `Ctrl+Shift+Right Arrow` |
| `cmd.sequence.voiceoversettingtrackheader` | Voice-Over Record Settings... | -- |
| `menu.sequence` | &Sequence | -- |

**`clip` namespace** (77 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.clip.adjustmentlayer` | Adjustment Layer | -- |
| `cmd.clip.aeify` | Replace With Composition | -- |
| `cmd.clip.attachhires` | Reconnect Full Resolution Media... | -- |
| `cmd.clip.attachproxy` | Attach Proxies... | -- |
| `cmd.clip.audiocategorization` | Auto-Tag Audio Types | -- |
| `cmd.clip.audiooptions.breakouttomono` | &Breakout to Mono | -- |
| `cmd.clip.audiooptions.gain` | Audio Gain... | `G` |
| `cmd.clip.audiooptions.nudgevolumedown` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.audiooptions.nudgevolumedown3` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.audiooptions.nudgevolumeup` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.audiooptions.nudgevolumeup3` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.audiooptions.sourcechannelmappings` | Audio Channels... | `Shift+G` |
| `cmd.clip.clipsettings` | Source Settings... | -- |
| `cmd.clip.create.multicam` | Create Multi-Camera Source Sequence... | -- |
| `cmd.clip.createproxies` | Create Proxies... | -- |
| `cmd.clip.deleteeffects` | Remove Attributes... | -- |
| `cmd.clip.detachproxy` | Detach Proxies | -- |
| `cmd.clip.disable.masterclipeffects` | Disable Source Clip Effects | -- |
| `cmd.clip.editoffline` | Edit &Offline... | -- |
| `cmd.clip.editsubclip` | Edit Subclip... | -- |
| `cmd.clip.enable` | Enable | `Shift+E` |
| `cmd.clip.enableenhanceaudio` | Enable Enhance Audio | -- |
| `cmd.clip.enhanceaudiosplitintoclips` | Split into Clips... | -- |
| `cmd.clip.extractaudio` | E&xtract Audio | -- |
| `cmd.clip.fillframe` | Fill frame | -- |
| `cmd.clip.fittoframe` | Fit to frame | -- |
| `cmd.clip.frameblend` | Frame Blending | -- |
| `cmd.clip.framesample` | Frame Sampling | -- |
| `cmd.clip.generatepeakfile` | &Generate Audio Waveform | -- |
| `cmd.clip.group` | Group | `Ctrl+G` |
| `cmd.clip.insert` | Insert | `,` |
| `cmd.clip.keyframe.addremoveaudio` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.addremovevideo` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.decreaseaudiovalue` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.decreasevideovalue` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.increaseaudiovalue` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.increasevideovalue` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.moveaudiooneframeearlier` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.moveaudiooneframelater` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.moveaudiotenframesearlier` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.moveaudiotenframeslater` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.movevideooneframeearlier` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.movevideooneframelater` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.movevideotenframesearlier` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.movevideotenframeslater` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.selectnext` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.keyframe.selectprevious` | _(no menu label; executable-only)_ | -- |
| `cmd.clip.linkaudioandvideo` | Unlink | `Ctrl+L` |
| `cmd.clip.makesubclip` | Make Subclip... | `Ctrl+U` |
| `cmd.clip.merge` | Merge Clips... | -- |
| `cmd.clip.multicam.flatten` | Flatten | -- |
| `cmd.clip.multicam.toggle` | Enable | -- |
| `cmd.clip.nestify` | Nest... | -- |
| `cmd.clip.openstockaudiosearch` | Search Stock Audio Library | -- |
| `cmd.clip.opticalflow` | Optical Flow | -- |
| `cmd.clip.overlay` | Overwrite | `.` |
| `cmd.clip.remix.enable` | Enable Remix | -- |
| `cmd.clip.remix.properties` | Remix Properties | -- |
| `cmd.clip.remix.revert` | Revert Remix | -- |
| `cmd.clip.remix.splitintosegments` | Split Clip into Segments | -- |
| `cmd.clip.rename` | Rename | -- |
| `cmd.clip.renderandreplace` | Render and Replace... | -- |
| `cmd.clip.replacefootage` | Replace Footage... | -- |
| `cmd.clip.restorecaptions` | Restore Captions from Source Clip | -- |
| `cmd.clip.restoreunrendered` | Restore Unrendered | -- |
| `cmd.clip.scaletoframesize` | Scale to Frame Size | -- |
| `cmd.clip.separatespeakers` | Separate Speakers | -- |
| `cmd.clip.speed` | Speed/Duration... | `Ctrl+R` |
| `cmd.clip.synchronizeclips` | Synchronize | -- |
| `cmd.clip.transcribeasset` | Transcribe... | -- |
| `cmd.clip.ungroup` | Ungroup | `Ctrl+Shift+G` |
| `cmd.clip.updatemetadata` | &Update Metadata... | -- |
| `cmd.clip.videooptions.addframehold` | Add Frame Hold | -- |
| `cmd.clip.videooptions.field` | Field Options... | -- |
| `cmd.clip.videooptions.frameholdoptions` | Frame Hold Options... | -- |
| `cmd.clip.videooptions.insertframeholdsegment` | Insert Frame Hold Segment | -- |
| `cmd.clip.videoupscale` | Upscale | -- |

**`multicam` namespace** (53 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.multicam.audio.follows.video` | Multi-Camera Audio Follows Video | -- |
| `cmd.multicam.choose.camera.1` | _(no menu label; executable-only)_ | `Ctrl+1` |
| `cmd.multicam.choose.camera.10` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.11` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.12` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.13` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.14` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.15` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.16` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choose.camera.2` | _(no menu label; executable-only)_ | `Ctrl+2` |
| `cmd.multicam.choose.camera.3` | _(no menu label; executable-only)_ | `Ctrl+3` |
| `cmd.multicam.choose.camera.4` | _(no menu label; executable-only)_ | `Ctrl+4` |
| `cmd.multicam.choose.camera.5` | _(no menu label; executable-only)_ | `Ctrl+5` |
| `cmd.multicam.choose.camera.6` | _(no menu label; executable-only)_ | `Ctrl+6` |
| `cmd.multicam.choose.camera.7` | _(no menu label; executable-only)_ | `Ctrl+7` |
| `cmd.multicam.choose.camera.8` | _(no menu label; executable-only)_ | `Ctrl+8` |
| `cmd.multicam.choose.camera.9` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera1` | _(no menu label; executable-only)_ | `1` |
| `cmd.multicam.choosenocut.camera10` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera11` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera12` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera13` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera14` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera15` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera16` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.choosenocut.camera2` | _(no menu label; executable-only)_ | `2` |
| `cmd.multicam.choosenocut.camera3` | _(no menu label; executable-only)_ | `3` |
| `cmd.multicam.choosenocut.camera4` | _(no menu label; executable-only)_ | `4` |
| `cmd.multicam.choosenocut.camera5` | _(no menu label; executable-only)_ | `5` |
| `cmd.multicam.choosenocut.camera6` | _(no menu label; executable-only)_ | `6` |
| `cmd.multicam.choosenocut.camera7` | _(no menu label; executable-only)_ | `7` |
| `cmd.multicam.choosenocut.camera8` | _(no menu label; executable-only)_ | `8` |
| `cmd.multicam.choosenocut.camera9` | _(no menu label; executable-only)_ | `9` |
| `cmd.multicam.enable.auto.decimation` | Auto-Adjust Multi-Camera Playback Quality | -- |
| `cmd.multicam.follows.nest.setting` | Multi-Camera Follows Nest Setting | -- |
| `cmd.multicam.next.camera` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.next.edit` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.page.first` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.page.last` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.page.next` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.page.previous` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.play` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.prev.edit` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.previous.camera` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.selection.top.down` | Multi-Camera Selection Top Down | -- |
| `cmd.multicam.show.program` | Show Multi-Camera Preview Monitor | -- |
| `cmd.multicam.step.back` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.step.forward` | _(no menu label; executable-only)_ | -- |
| `cmd.multicam.switch.camera.audio` | Switch Camera Audio | -- |
| `cmd.multicam.toggle.multicam.view` | _(no menu label; executable-only)_ | `Shift+0` |
| `cmd.multicam.toggle.record` | _(no menu label; executable-only)_ | `0` |
| `cmd.multicam.transmit.gridview` | Transmit Multi-Camera View | -- |
| `menu.multicam` | Multi-Camera | -- |

**`tools` namespace** (29 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.tools.01pointer` | _(no menu label; executable-only)_ | `V` |
| `cmd.tools.02_5trackselectbackward` | _(no menu label; executable-only)_ | `Shift+A` |
| `cmd.tools.02trackselectforward` | _(no menu label; executable-only)_ | `A` |
| `cmd.tools.03ripple` | _(no menu label; executable-only)_ | `B` |
| `cmd.tools.04roll` | _(no menu label; executable-only)_ | `N` |
| `cmd.tools.05ratestretch` | _(no menu label; executable-only)_ | `R` |
| `cmd.tools.06razor` | _(no menu label; executable-only)_ | `C` |
| `cmd.tools.07slip` | _(no menu label; executable-only)_ | `Y` |
| `cmd.tools.08slide` | _(no menu label; executable-only)_ | `U` |
| `cmd.tools.09pen` | _(no menu label; executable-only)_ | `P` |
| `cmd.tools.10hand` | _(no menu label; executable-only)_ | `H` |
| `cmd.tools.11zoom` | _(no menu label; executable-only)_ | `Z` |
| `cmd.tools.12text` | _(no menu label; executable-only)_ | `T` |
| `cmd.tools.13rectshape` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.14verticaltype` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.15ellipseshape` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.16Remix` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.17polygonshape` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.18smartselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.19ellipseselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.20rectselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.21bezierselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.22hslselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.23luminanceselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.23rangeselection` | _(no menu label; executable-only)_ | `Ctrl+Alt+G` |
| `cmd.tools.24genextend` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.25rangeselection` | _(no menu label; executable-only)_ | -- |
| `cmd.tools.object.selection` | _(no menu label; executable-only)_ | `Ctrl+Alt+O` |
| `cmd.tools.object.selection.change.drawmode` | _(no menu label; executable-only)_ | `Ctrl+Alt+L` |

**`monitor` namespace** (110 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.monitor.addcliptoproject` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.addguide` | &Add Guide... | -- |
| `cmd.monitor.ambisonics.monitor.toggle` | Monitor Ambisonics | -- |
| `cmd.monitor.closeallclips` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.closeclip` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.colormanagement.ganged` | Gang to Active Sequence Color Management | -- |
| `cmd.monitor.colormanagement.off` | Show Wide Gamut Source Media As Log | -- |
| `cmd.monitor.colormanagement.on` | Show Source in Extended Dynamic Range | -- |
| `cmd.monitor.fields.both` | Display Both Fields | -- |
| `cmd.monitor.fields.first` | Display First Field | -- |
| `cmd.monitor.fields.second` | Display Second Field | -- |
| `cmd.monitor.firstclip` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.force.media.refresh` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.fx.mute` | Global FX Mute | -- |
| `cmd.monitor.gang.source.and.program` | Gang Source and Program | -- |
| `cmd.monitor.guide.edit` | Edit Guide... | -- |
| `cmd.monitor.guides` | Show Guides | `Ctrl+;` |
| `cmd.monitor.lastclip` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.lockguides` | &Lock Guides | `Ctrl+Alt+Shift+R` |
| `cmd.monitor.loop` | Loop | `Ctrl+L` |
| `cmd.monitor.manageguides` | &Manage Guides... | -- |
| `cmd.monitor.multicam.toggle` | Enable | -- |
| `cmd.monitor.nextclip` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.nudge.down.five` | Nudge Down by 5 | `Ctrl+Shift+Down Arrow` |
| `cmd.monitor.nudge.down.one` | Nudge Down by 1 | `Ctrl+Down Arrow` |
| `cmd.monitor.nudge.left.five` | Nudge Left by 5 | `Ctrl+Shift+Left Arrow` |
| `cmd.monitor.nudge.left.one` | Nudge Left by 1 | `Ctrl+Left Arrow` |
| `cmd.monitor.nudge.right.five` | Nudge Right by 5 | `Ctrl+Shift+Right Arrow` |
| `cmd.monitor.nudge.right.one` | Nudge Right by 1 | `Ctrl+Right Arrow` |
| `cmd.monitor.nudge.up.five` | Nudge Up by 5 | `Ctrl+Shift+Up Arrow` |
| `cmd.monitor.nudge.up.one` | Nudge Up by 1 | `Ctrl+Up Arrow` |
| `cmd.monitor.output.alpha` | Alpha | -- |
| `cmd.monitor.output.audio.video` | Video and Audio Waveform Split | -- |
| `cmd.monitor.output.blue` | Blue | -- |
| `cmd.monitor.output.composite` | Composite Video | -- |
| `cmd.monitor.output.green` | Green | -- |
| `cmd.monitor.output.red` | Red | -- |
| `cmd.monitor.output.zoom.10` | 10% | -- |
| `cmd.monitor.output.zoom.100` | 100% | -- |
| `cmd.monitor.output.zoom.150` | 150% | -- |
| `cmd.monitor.output.zoom.1600` | 1600% | -- |
| `cmd.monitor.output.zoom.200` | 200% | -- |
| `cmd.monitor.output.zoom.25` | 25% | -- |
| `cmd.monitor.output.zoom.400` | 400% | -- |
| `cmd.monitor.output.zoom.50` | 50% | -- |
| `cmd.monitor.output.zoom.75` | 75% | -- |
| `cmd.monitor.output.zoom.800` | 800% | -- |
| `cmd.monitor.output.zoom.fit` | Fit | -- |
| `cmd.monitor.outputaudiowaveform` | Audio Waveform | -- |
| `cmd.monitor.outputmulticam` | Multi-Camera | -- |
| `cmd.monitor.overlays` | Overlays | -- |
| `cmd.monitor.paused.resolution.eighth` | 1/8 | -- |
| `cmd.monitor.paused.resolution.full` | Full | -- |
| `cmd.monitor.paused.resolution.half` | 1/2 | -- |
| `cmd.monitor.paused.resolution.quarter` | 1/4 | -- |
| `cmd.monitor.paused.resolution.sixteenth` | 1/16 | -- |
| `cmd.monitor.playback.qualityishigh` | High Quality Playback | -- |
| `cmd.monitor.playback.resolution.eighth` | 1/8 | -- |
| `cmd.monitor.playback.resolution.full` | Full | -- |
| `cmd.monitor.playback.resolution.half` | 1/2 | -- |
| `cmd.monitor.playback.resolution.quarter` | 1/4 | -- |
| `cmd.monitor.playback.resolution.sixteenth` | 1/16 | -- |
| `cmd.monitor.playstoptoggle` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.previousclip` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.program.multicam.edit.cameras` | Edit Cameras... | -- |
| `cmd.monitor.program.showscrollbars` | Show Scroll Bars | -- |
| `cmd.monitor.program.showtransportcontrols` | Show Transport Controls | -- |
| `cmd.monitor.removeguides` | Clear Guides | -- |
| `cmd.monitor.ruler.percentages` | Percent | -- |
| `cmd.monitor.ruler.pixels` | Pixels | -- |
| `cmd.monitor.rulers` | Show Rulers | `Ctrl+R` |
| `cmd.monitor.saveguides` | &Save Guides as Template... | -- |
| `cmd.monitor.sequence.clip.toggle` | _(no menu label; executable-only)_ | `Ctrl+1` |
| `cmd.monitor.snapping` | Snap in Program Monitor | `Ctrl+Shift+;` |
| `cmd.monitor.solo.composite.toggle` | _(no menu label; executable-only)_ | `Ctrl+2` |
| `cmd.monitor.source.multicam.edit.cameras` | Edit Cameras... | -- |
| `cmd.monitor.source.revealinproject` | Reveal in Project | -- |
| `cmd.monitor.source.showscrollbars` | Show Scroll Bars | -- |
| `cmd.monitor.source.showtransportcontrols` | Show Transport Controls | -- |
| `cmd.monitor.source.viewcaptionstream.enable` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream1` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream10` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream2` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream3` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream4` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream5` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream6` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream7` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream8` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.source.viewcaptionstream.stream9` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.split.screen` | Comparison View | `Ctrl+3` |
| `cmd.monitor.step.backward` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.step.forward` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.timecodeoverlay` | Timecode Overlay During Edit | -- |
| `cmd.monitor.toggle.crop.dm` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.toggle.grade` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.toggle.selection.bounding.box` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.toggleaudio` | Show Audio Time Units | -- |
| `cmd.monitor.toggledroppedframeindicator` | Show Dropped Frame Indicator | -- |
| `cmd.monitor.togglemarkers` | Show Markers | -- |
| `cmd.monitor.togglesafearea` | Safe Margins | -- |
| `cmd.monitor.toggletimerulernumbers` | Time Ruler Numbers | -- |
| `cmd.monitor.toggletransparencygrid` | Transparency Grid | -- |
| `cmd.monitor.tracks.0` | Video 1 | -- |
| `cmd.monitor.view.sequences.in.timeline` | Open Sequence in Timeline | -- |
| `cmd.monitor.view.trimsession.rollback` | _(no menu label; executable-only)_ | -- |
| `cmd.monitor.vrviewer.minimalcontrols` | Show Controls | -- |
| `cmd.monitor.vrviewer.settings` | Settings... | -- |
| `cmd.monitor.vrviewer.toggle` | Enable | -- |
| `cmd.monitor.wingtip.safemargins` | _(no menu label; executable-only)_ | -- |

**`transport` namespace** (26 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.transport.fastforward` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.play.ctitoaudioout` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.play.ctitoout` | _(no menu label; executable-only)_ | `Ctrl+Space` |
| `cmd.transport.play.ctitovideoout` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.play.fat` | _(no menu label; executable-only)_ | `Shift+Space` |
| `cmd.transport.playaudiointoout` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.playedit` | _(no menu label; executable-only)_ | `Shift+K` |
| `cmd.transport.playintoout` | _(no menu label; executable-only)_ | `Ctrl+Shift+Space` |
| `cmd.transport.playvideointoout` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.record` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.rewind` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.selectedclip.end` | _(no menu label; executable-only)_ | `Shift+End` |
| `cmd.transport.selectedclip.start` | _(no menu label; executable-only)_ | `Shift+Home` |
| `cmd.transport.sequence.end` | _(no menu label; executable-only)_ | `End` |
| `cmd.transport.sequence.start` | _(no menu label; executable-only)_ | `Home` |
| `cmd.transport.shuttle.left` | _(no menu label; executable-only)_ | `J` |
| `cmd.transport.shuttle.right` | _(no menu label; executable-only)_ | `L` |
| `cmd.transport.shuttle.slow.left` | _(no menu label; executable-only)_ | `Shift+J` |
| `cmd.transport.shuttle.slow.right` | _(no menu label; executable-only)_ | `Shift+L` |
| `cmd.transport.shuttle.stop` | _(no menu label; executable-only)_ | `K` |
| `cmd.transport.step.back` | _(no menu label; executable-only)_ | `Left Arrow` |
| `cmd.transport.step.back.five` | _(no menu label; executable-only)_ | `Shift+Left Arrow` |
| `cmd.transport.step.forward` | _(no menu label; executable-only)_ | `Right Arrow` |
| `cmd.transport.step.forward.five` | _(no menu label; executable-only)_ | `Shift+Right Arrow` |
| `cmd.transport.stop` | _(no menu label; executable-only)_ | -- |
| `cmd.transport.toggleplay` | _(no menu label; executable-only)_ | `Space` |

**`marker` namespace** (62 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.marker.add.0` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.1` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.2` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.3` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.4` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.5` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.6` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.add.7` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.autogeneratedvdmarekrs` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.clearclipmarker.all` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.clearclipmarker.current` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.cleardvdmarker.all` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.cleardvdmarker.current` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.clearmarker.all` | Clear Markers | `Ctrl+Alt+Shift+M` |
| `cmd.marker.clearmarker.current` | Clear Selected Marker | `Ctrl+Alt+M` |
| `cmd.marker.clearsequencemarker.all` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.copypaste.includessequencemarkers` | Copy Paste &Includes Sequence Markers | -- |
| `cmd.marker.dvd.prev` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.edit` | Edit Marker... | -- |
| `cmd.marker.gotoclipmarker.audioin` | Audio In | -- |
| `cmd.marker.gotoclipmarker.audioout` | Audio Out | -- |
| `cmd.marker.gotoclipmarker.next` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.gotoclipmarker.numbered` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.gotoclipmarker.previous` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.gotoclipmarker.videoin` | Video In | -- |
| `cmd.marker.gotoclipmarker.videoout` | Video Out | -- |
| `cmd.marker.gotodvdmarker.next` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.gotomarker.next` | Go to Next Marker | `Shift+M` |
| `cmd.marker.gotomarker.previous` | Go to Previous Marker | `Ctrl+Shift+M` |
| `cmd.marker.gotosequencemarker.next` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.gotosequencemarker.previous` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.ignoretimelineselection` | Ignore Selection in Timeline | -- |
| `cmd.marker.nextrow` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.previousrow` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setchaptermarker` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setchaptermarkerdialog` | Add Chapter Marker... | -- |
| `cmd.marker.setclipmarker.audioin` | Audio In | -- |
| `cmd.marker.setclipmarker.audiout` | Audio Out | -- |
| `cmd.marker.setclipmarker.videoin` | Video In | -- |
| `cmd.marker.setclipmarker.videoout` | Video Out | -- |
| `cmd.marker.setcolor1` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor2` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor3` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor4` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor5` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor6` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor7` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setcolor8` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setdvdmarker` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setflashcuemarker` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.setflashcuemarkerdialog` | Add Flash Cue Marker... | -- |
| `cmd.marker.setsequenceinoutmarkeraroundselection.out` | Mark Selection | `/` |
| `cmd.marker.setsequenceinoutmarkeraroundtargetclip` | Mark Clip | `X` |
| `cmd.marker.setstopdvdmarker` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.show.allmarkers` | Show All Markers | -- |
| `cmd.marker.show.clipmarkers` | Show Clip Markers | -- |
| `cmd.marker.show.sequencemarkers` | Show Sequence Markers | -- |
| `cmd.marker.showdvdmarkers` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.showinout` | Show In and Out | -- |
| `cmd.marker.showmarker.all` | &Show All Marker Colors | -- |
| `cmd.marker.showmarkers` | _(no menu label; executable-only)_ | -- |
| `cmd.marker.style.ripplesequencemarkers` | &Ripple Sequence Markers | -- |

**`track` namespace** (1 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.track.bidirectional` | Track Mask | -- |

**`capture` namespace** (11 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.capture.eject` | _(no menu label; executable-only)_ | `E` |
| `cmd.capture.fastforward` | _(no menu label; executable-only)_ | `F` |
| `cmd.capture.fastrewind` | _(no menu label; executable-only)_ | `R` |
| `cmd.capture.goto.inpoint` | _(no menu label; executable-only)_ | `Q` |
| `cmd.capture.goto.outpoint` | _(no menu label; executable-only)_ | `W` |
| `cmd.capture.record` | _(no menu label; executable-only)_ | `G` |
| `cmd.capture.record.audio` | _(no menu label; executable-only)_ | `A` |
| `cmd.capture.record.video` | _(no menu label; executable-only)_ | `V` |
| `cmd.capture.step.back` | _(no menu label; executable-only)_ | `Left Arrow` |
| `cmd.capture.step.forward` | _(no menu label; executable-only)_ | `Right Arrow` |
| `cmd.capture.stop` | _(no menu label; executable-only)_ | `S` |

**`posterframe` namespace** (4 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.posterframe.clear` | Clear Poster Frame | `Ctrl+Shift+P` |
| `cmd.posterframe.move.backward` | _(no menu label; executable-only)_ | -- |
| `cmd.posterframe.move.forward` | _(no menu label; executable-only)_ | -- |
| `cmd.posterframe.set` | Set Poster Frame | `Shift+P` |

**`roughcut` namespace** (3 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.roughcut.add51audiotrack` | _(no menu label; executable-only)_ | -- |
| `cmd.roughcut.addmonoaudiotrack` | _(no menu label; executable-only)_ | -- |
| `cmd.roughcut.addstereoaudiotrack` | _(no menu label; executable-only)_ | -- |

**`clipgrid` namespace** (7 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command id | Label | Default binding (en) |
|---|---|---|
| `cmd.clipgrid.pastegradetoclip.andappend` | Paste and Append | -- |
| `cmd.clipgrid.pastegradetoclip.andreplace` | Paste and Replace | -- |
| `cmd.clipgrid.pastegradetoclip.asclipadjustments` | Paste as Clip Adjustments | -- |
| `cmd.clipgrid.pastegradetoclip.asclipoperations` | _(no menu label; executable-only)_ | -- |
| `cmd.clipgrid.pastegradetoclip.preservegrouprelationships` | Paste to Preserve Group Relationships | -- |
| `cmd.clipgrid.pastegradetoselection.andreplace` | _(no menu label; executable-only)_ | -- |
| `cmd.clipgrid.pastegradetoselection.preservegrouprelationships` | _(no menu label; executable-only)_ | -- |

**[STU-VID-042] The keyboard binding model is normative.** A binding is `(context, command,
key, modifiers)`. 31 contexts are declared, so the same key legally means different things in the
timeline, the program monitor, the trim session, the multicam surface, the project panel and a text
field; a flat global binding table is insufficient and MUST NOT be built. Key identity is a Unicode
code point of the key's unshifted character, not a platform virtual-key code, with a separate flag
for the numeric-keypad variant of the same character and a separate small enumeration for
non-character keys; this is what allows one binding set to survive a QWERTZ or Cyrillic layout.
Studio ships one default binding set, supports named alternative sets, and stores a set as portable
data.

---

## 14.25.3 Media import, codecs and containers

**[STU-VID-050] Import is a typed, enumerable capability surface, not a file-open dialog.** Studio
declares its importable media types explicitly, and an unsupported item produces a determinate
`MEDIA_IMPORT_UNSUPPORTED` result naming the detected type rather than a generic failure. 39
importer modules with 404 declared namespace strings are recovered and reproduced below as the
coverage target.

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Importer module | Display name | Namespace strings |
|---|---|---|
| `BarsAndTone` | -- | 39 |
| `ExporterGIF` | -- | 2 |
| `ImporterAIEPS` | Adobe Illustrator File | 1 |
| `ImporterAIFF` | Audio Interchange File Format | 2 |
| `ImporterARRIRAW` | ARRIRAW Files | 27 |
| `ImporterAVI` | AVI Movie | 37 |
| `ImporterBarsAndTone` | Bars and Tone | 5 |
| `ImporterBlackMatte` | Black Video | 1 |
| `ImporterColorMatte` | Color Matte | 3 |
| `ImporterDPX` | Cineon/DPX File | 2 |
| `ImporterDirectShow` | AVI Movie | 1 |
| `ImporterFFMPEG` | AVI Motion JPEG | 4 |
| `ImporterFastMPEG` | MPEG Movie | 2 |
| `ImporterFrameGen` | Frame Generator Test Tool | 1 |
| `ImporterGraphics` | Graphic | 1 |
| `ImporterHEIF` | -- | 2 |
| `ImporterJPEG` | JPEG File | 3 |
| `ImporterLeader` | Universal Counting Leader | 6 |
| `ImporterLumetri` | -- | 9 |
| `ImporterMP3` | MP3 Audio | 1 |
| `ImporterMP4` | MPEG Movie | 4 |
| `ImporterMPEG` | MPEG Movie | 17 |
| `ImporterMXF` | MXF | 112 |
| `ImporterMultiStill` | -- | 3 |
| `ImporterPNG` | PNG File | 1 |
| `ImporterPhotoshopProxy` | Photoshop | 1 |
| `ImporterQT` | -- | 43 |
| `ImporterQuickTime` | -- | 15 |
| `ImporterRed` | -- | 38 |
| `ImporterSVG` | -- | 1 |
| `ImporterSyntheticCaption` | SyntheticCaption | 1 |
| `ImporterTarga` | Truevision Targa File | 1 |
| `ImporterTiff` | TIFF image file | 9 |
| `ImporterTransparentMatte` | Transparent Video | 1 |
| `ImporterVFRaw` | VFRaw File | 1 |
| `ImporterWEBP` | -- | 2 |
| `ImporterWave` | Waveform Audio | 3 |
| `ImporterWindowsMedia` | Windows Media | 1 |
| `ImporterXDCAMEX` | -- | 1 |

**Camera / raw source-settings effects (21):** `ADBE ARRIRAW MXF.SourceSettings`, `ADBE ARRIRAW.SourceSettings`, `ADBE CanonRaw.SourceSettings`, `ADBE CinemaDNG.SourceSettings`, `ADBE DPX.SourceSettings`, `ADBE ImporterMXF.SourceSettings`, `ADBE MPEG.SourceSettings`, `ADBE ProResRaw.SourceSettings`, `ADBE RED.SourceSettings`, `ADBE SonyRawF65.SourceSettings`, `AE.ADBE ARRIRAW MXF.SourceSettings`, `AE.ADBE ARRIRAW.SourceSettings`, `AE.ADBE CanonRaw.SourceSettings`, `AE.ADBE CinemaDNG.SourceSettings`, `AE.ADBE DPX.SourceSettings`, `AE.ADBE GOP.SourceSettings`, `AE.ADBE ImporterMXF.SourceSettings`, `AE.ADBE MPEG.SourceSettings`, `AE.ADBE ProResRaw.SourceSettings`, `AE.ADBE RED.SourceSettings`, `AE.ADBE SonyRawF65.SourceSettings`

**Media-type vocabulary keys (201)**

[STU-VID-051] **Codec and container support is a declared
inventory with vendor and role, and it is an ADAPTER boundary.** 58 codec and container modules
across 17 vendors are recovered, in roles `decode` (19), `encode` (12), `decode/encode` (9),
`demux` (3), `mux` (2), `demux/mux` (1), `interchange` (3), `metadata` (3), `accelerate` (2),
`support` (2) and `transform` (1). Studio's contract:

1. Studio's own crates own the timeline, the compositor and the parameter surfaces. Codec
   implementation is NOT reimplemented natively; it is reached through a declared adapter with a
   typed capability descriptor stating which containers, codecs, bit depths, chroma subsamplings
   and colour transfer functions it can decode and encode.
2. Every adapter is enumerable at runtime, so a headless run and a model can both ask what is
   supported before attempting an operation ([STU-FX-038] posture).
3. An absent adapter yields a determinate `MEDIA_CODEC_UNAVAILABLE` result naming the codec, never
   a silent empty render.
4. No codec adapter may pull a GPU dependency into `handshake_core` ([STU-ARC-002]).
5. Which specific codecs Studio ships is a licensing and dependency decision outside this
   sub-section's authority; what this sub-section requires is the descriptor, the enumeration and
   the determinate failure.

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Module | Vendor | Role |
|---|---|---|
| `AAFCOAPI.dll` | AMWA | interchange |
| `ArriImageSdk.8.dll` | ARRI | decode |
| `AVCIntraEncoder.dll` | Adobe | encode |
| `AdobePDFL.dll` | Adobe | decode |
| `AdobeSVGAGM.dll` | Adobe | decode |
| `AdobeXMP.dll` | Adobe | metadata |
| `AdobeXMPFiles.dll` | Adobe | metadata |
| `JP2KLib.dll` | Adobe | decode/encode |
| `SVGExport.dll` | Adobe | encode |
| `SVGRE.dll` | Adobe | decode |
| `adobe_c2pa.dll` | Adobe | metadata |
| `jpeg_wrapper.dll` | Adobe | decode/encode |
| `ProResOpt.dll` | Apple | decode/encode |
| `ProResRAW.dll` | Apple | decode |
| `DNxSDK-vs2019.dll` | Avid | decode/encode |
| `DQomfToolkit64.dll` | Avid | interchange |
| `Pro4OMFdll64.dll` | Avid | interchange |
| `crxdec.dll` | Canon | decode |
| `codexhdedecoder.dll` | Codex | decode |
| `libavcodec.dll` | FFmpeg | decode/encode |
| `libavformat.dll` | FFmpeg | demux/mux |
| `libavutil.dll` | FFmpeg | support |
| `CFHDDecoder64.dll` | GoPro | decode |
| `CFHDEncoder64.dll` | GoPro | encode |
| `kdu_as85R.dll` | Kakadu | decode/encode |
| `kdu_vs85R.dll` | Kakadu | decode/encode |
| `libmp3lame.dll` | LAME | encode |
| `MOG_Framework_1.1.12.dll` | MOG | decode/encode |
| `MSDK_Pro_1.1.12.dll` | MOG | decode/encode |
| `mc_dec_aac.dll` | MainConcept | decode |
| `mc_dec_avc.dll` | MainConcept | decode |
| `mc_dec_dv100.dll` | MainConcept | decode |
| `mc_dec_mp2v.dll` | MainConcept | decode |
| `mc_dec_mp4v.dll` | MainConcept | decode |
| `mc_dec_mpa.dll` | MainConcept | decode |
| `mc_demux_mp2.dll` | MainConcept | demux |
| `mc_demux_mp4.dll` | MainConcept | demux |
| `mc_demux_mxf.dll` | MainConcept | demux |
| `mc_enc_aac.dll` | MainConcept | encode |
| `mc_enc_avc.dll` | MainConcept | encode |
| `mc_enc_avcsr.dll` | MainConcept | encode |
| `mc_enc_dv100.dll` | MainConcept | encode |
| `mc_enc_mp2sr.dll` | MainConcept | encode |
| `mc_enc_mp2v.dll` | MainConcept | encode |
| `mc_enc_mp4v.dll` | MainConcept | encode |
| `mc_enc_mpa.dll` | MainConcept | encode |
| `mc_mfimport.dll` | MainConcept | import |
| `mc_mux_mp2.dll` | MainConcept | mux |
| `mc_mux_mp4.dll` | MainConcept | mux |
| `mc_trans_video_colorspace.dll` | MainConcept | transform |
| `REDCuda-x64.dll` | RED | accelerate |
| `REDDecoder-x64.dll` | RED | decode |
| `REDOpenCL-x64.dll` | RED | accelerate |
| `REDR3D-x64.dll` | RED | decode |
| `SMDK-VC140-x64-4_26_0.dll` | Sony | support |
| `SonyRawDev.dll` | Sony | decode |
| `LibLTCWrapper.dll` | libltc | decode |
| `libmpg123.dll` | mpg123 | decode |

**[STU-VID-052] Camera and raw source settings are per-media-item, not per-clip.** 21 raw source-
settings surfaces are declared across the recovered formats. A raw media item carries a develop
state -- colour space, exposure, white balance and format-specific controls -- that applies to every
clip cut from it, and changing it re-renders all of them. This is the same non-destructive develop
model as 14.12 and MUST be the same primitive, not a video-only copy of it ([STU-DOC-004]); 14.12
owns the engine and this sub-section binds it to media items.

**[STU-VID-053] Conform, peak files and the media cache are explicit background operations.**
Generating audio waveform peak files, conforming audio to the sequence sample rate, and building
seek indices are queued, cancellable, observable operations subject to the headless and quiet law.
Their outputs are derived artifacts in the configured cache tier, are safe to delete, and MUST be
regenerable from the source; nothing in a project may depend on a cache artifact surviving.

---

## 14.25.4 Export

**[STU-VID-060] Video export is a `StudioExportRecipe` and 14.13 owns the format writers.** [STU-PRO-043]
already states that all output is produced through the export pipeline rather than by
a per-domain exporter, and that holds here. This group states the video export CONTRACT: what a
recipe contains and what the parameter surface is.

**[STU-VID-061] The export parameter dictionary is normative.** 453 distinct export parameter
identifiers across 62,670 parameter rows in 1,541 shipped recipes are recovered, grouped into 14
facets. Every one is a `StudioEffectParameter` under 14.9.1 -- same record, same hard/soft split,
same `bound_state`, same unit and precision fields. The facet grouping is the normative organising
axis of the export inspector:

*Derivation: catalogue table, splits per row; yields 14 microtasks, one per export facet.*

| Facet | Identifiers | What it governs |
|---|---|---|
| `video_codec` | 41 | Codec selection, profile, level, entropy coding, bit depth. |
| `rate_control` | 34 | Bitrate mode, target and maximum bitrate, quality, pass count. |
| `gop_structure` | 13 | Keyframe interval, B-frame count, reference frames, closed-GOP. |
| `video_frame` | 5 | Output frame size, frame rate, field order, pixel aspect. |
| `audio` | 36 | Codec, sample rate, channel layout, bitrate, mode. |
| `multiplexing` | 10 | Container muxing, stream interleave, transport-stream parameters. |
| `colour` | 22 | Output colour space, transfer, primaries, range, tone mapping. |
| `layout` | 16 | Scaling, cropping, padding, source and output rectangles. |
| `captions` | 4 | Caption stream inclusion, format, burn-in. |
| `vr_immersive` | 5 | Projection, stereoscopic layout, field of view. |
| `metadata` | 2 | Metadata and side-car inclusion. |
| `performance` | 1 | Encoder performance/quality trade-off. |
| `publishing_destination` | 86 | Destination bindings. **Studio ships NONE of these as native behaviour**; a publishing destination is an optional adapter under [STU-FX-032]'s posture and no export path may require an account or a network ([STU-OVR-002]). They are enumerated so the surface is understood, not so it is built. |
| `other` | 178 | Identifiers whose facet could not be determined; each requires deliberate classification before implementation. Declared gap [STU-VID-083]. |

**[STU-VID-061a] Only 44 of the 453 identifiers carry a shipped human label.** The remaining 409 are
identified but unlabelled. Studio MUST author operator-facing labels and manual prose for them
rather than exposing raw identifiers, and MUST NOT invent semantics for an identifier whose meaning
is not established. This is the export-side instance of the prose-coverage limit [STU-FX-150].

**[STU-VID-061b] Reading convention for the facet tables below, and what the export capture did NOT
declare.** Every facet table carries the seven parameter fields of [STU-FX-105] as SEPARATE columns
-- `hard_min`, `hard_max`, `soft_min`, `soft_max`, `default`, `unit`, `precision` -- followed by
three provenance columns that are not part of the parameter contract. `--` means the source declares
nothing and Studio declares nothing, exactly as [STU-FX-131a] defines it; it never means zero,
never means unbounded-by-decision, and never licenses a clamp.

1. `hard_min` and `hard_max` carry the capture's declared minimum and maximum. A boolean identifier
   declares `false`/`false` and a group header declares `0`/`0`; both are the declared domain of
   that identifier, not a numeric range.
2. `soft_min` and `soft_max` are `--` on every row of every facet table. The export capture declares
   a valid range and a CONTROL-IS-SCRUBBABLE flag, but no separate control range. The flag is
   preserved as the `slider` column, which is a boolean and is NOT a soft range; reading it as one
   would fabricate bounds. This is the opposite asymmetry to 14.9's effect records, where 745 rows
   declare both pairs and 505 of them differ ([STU-FX-105a]), and the two must not be conflated.
3. `default` is `--` on every row. Export defaults are carried by the 1,541 shipped recipes
   ([STU-VID-063]), not by the identifier dictionary; a per-identifier default would have to be
   derived from recipe frequency and would be an observation, never a declaration ([STU-FX-106]).
4. `unit` is `--` on every row. The export capture carries no unit tokens. An implementer authors
   the unit deliberately from the identifier's semantics and records that it was authored.
5. `precision` carries the capture's declared decimal count where one exists.

An implementer MUST NOT fill any `--` by copying its twin, and MUST NOT clamp to a value observed
across shipped recipes as though it had been declared. Authoring the missing soft ranges, defaults
and units for the 453 identifiers is explicit scope under [STU-VID-083] and [STU-FX-146], not a
detail to be improvised at implementation time.


**facet `audio`** (36 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEAACAudioMode` | Mode | signed integer | 0 | 3 | -- | -- | -- | -- | 2 | no | 34 | H264, HEVC |
| `ADBEAACAudioOversampled` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 134 | AAC, H264, HEVC, MP4 |
| `ADBEAACAudioPrecedence` | Precedence | signed integer | 0 | 1 | -- | -- | -- | -- | 2 | no | 134 | AAC, H264, HEVC, MP4 |
| `ADBEAudioAdvancedSettingsGroup` | Advanced Settings | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 188 | AAC, H264, HEVC, MP4, dvd, mpg2 |
| `ADBEAudioAmbiDoExport` | Audio Is Ambisonics | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 164 | H264, HEVC, MooV |
| `ADBEAudioAmbiGroup` | Ambisonics | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 164 | H264, HEVC, MooV |
| `ADBEAudioChannelConfiguration` | Audio Channel Configuration | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 175 | H264, HEVC, MooV |
| `ADBEAudioChannelConfigurationGroup` | Audio Channel Configuration | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 175 | H264, HEVC, MooV |
| `ADBEAudioDeemphasis` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 54 | dvd, mpg2 |
| `ADBEAudioEnableCRC` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 54 | dvd, mpg2 |
| `ADBEAudioFormat` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEAudioInterleave` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 19 | AVIV |
| `ADBEAudioLayer` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEAudioMode` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEAudioNumChannels` | -- | signed integer | -51 | 32 | -- | -- | -- | -- | 2 | no | 1475 | AAC, AIFF, AVIV, DCP_, DMXF, H264, H26B, HEVC |
| `ADBEAudioPsychMode` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 54 | dvd, mpg2 |
| `ADBEAudioRatePerSecond` | -- | signed integer | 8000 | 192000 | -- | -- | -- | -- | 2 | no | 1503 | AAC, AIFF, AVIV, DCP_, DMXF, H264, H26B, HEVC |
| `ADBEAudioSampleType` | Sample Size | signed integer | 0 | 4 | -- | -- | -- | -- | 2 | no | 1421 | AAC, AIFF, AVIV, DMXF, H264, H26B, HEVC, JMXF |
| `ADBEAudioSetCopyrightBit` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 54 | dvd, mpg2 |
| `ADBEAudioSetOriginalBit` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 54 | dvd, mpg2 |
| `ADBEAudioSetPrivateBit` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 54 | dvd, mpg2 |
| `ADBEAudioStreamMonoDiscrete` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 62 | MooV |
| `ADBEAudioTabGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 1504 | AAC, AIFF, AVIV, DCP_, DMXF, H264, H26B, HEVC |
| `ADBEAudioTrackLayout` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 99 | MooV |
| `ADBEAudioTrackLayoutGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 99 | MooV |
| `ADBEBasicAudioGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 1425 | AAC, AIFF, AVIV, DCP_, DMXF, H264, H26B, HEVC |
| `ADBEChannelLayout` | Channel Layout | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 90 | MooV |
| `ADBEChannelLayoutGroup` | Channel Layout | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 90 | MooV |
| `ADBEFLVEncodeAlphaChannel` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 12 | flv |
| `ADBEForceMonoAudioTracks` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 144 | JMXF, MX10, MX11, MXFX, PMXF |
| `ADBEMuxAudioBufferSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVMC_AudioLayer` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `AudioSampleSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 4 | DCP_ |
| `WMAudioBufferSize` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |
| `WMAudioMode` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |
| `WMAudioPeakBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |

**facet `captions`** (4 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBECaptionsExportOption` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 584 | DMXF, H264, MooV, dvd, mbd, mpg2 |
| `ADBECaptionsFormat` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 584 | DMXF, H264, MooV, dvd, mbd, mpg2 |
| `ADBECaptionsFrameRate` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 584 | DMXF, H264, MooV, dvd, mbd, mpg2 |
| `ADBECaptionsTabGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 584 | DMXF, H264, MooV, dvd, mbd, mpg2 |

**facet `colour`** (22 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEBMPEGVideoContentLightLevelsGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEExportColorSpace` | Export Color Space | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 203 | AVIV, DIBB, DPX, GIFf, H264, HEVC, JMXF, JPEG |
| `ADBEGamma` | -- | floating point | 0.001 | 10 | -- | -- | -- | -- | -- | yes | 8 | DPX |
| `ADBEH26xColorPrimariesParam` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | MooV |
| `ADBEH26xHDRParam` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | MooV |
| `ADBEMPEGGroupMasteringDisplayGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoHDR10MasterDisplayLuminanceMax` | Luminance Max(cd/m^2) | signed integer | 100 | 4000 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoHDR10MasterDisplayLuminanceMin` | Luminance Min(cd/m^2) | floating point | 0.0005 | 0.05 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoHDR10MasterDisplayPrimaries` | Color Primaries | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoHDR10MaxContentLightLevel` | Maximum(cd/m^2) | signed integer | 100 | 4000 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoHDR10MaxFrameAvgLightLevel` | Average(cd/m^2) | signed integer | 20 | 4000 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBERenderDeepColor` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 1493 | AVIV, DIBB, DMXF, DPX, GIFf, H264, H26B, HEVC |
| `ADBEVideoBitDepth` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 1041 | AVIV, DMXF, DPX, JMXF, MX10, MX11, MXFX, MooV |
| `ADBEVideoColorPrimaries` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoJ2KColorPrimaries` | -- | signed integer | -- | 2 | -- | -- | -- | -- | -- | no | 22 | JMXF, PMXF |
| `ADBEVideoMatrixCoefficients` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBE_H264_ColorPrimaries` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 88 | H264 |
| `ADBE_H264_HDR` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 88 | H264 |
| `ADBE_HEVC_ColorPrimaries` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 20 | HEVC |
| `ADBE_HEVC_HDR` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 20 | HEVC |
| `ADBE_VIDEO_HDR10` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `VideoHybridLogGamma` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 58 | MX10, MX11, MXFX |

**facet `gop_structure`** (13 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEKeyframe` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 25 | AVIV, MooV |
| `ADBEKeyframeEvery` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 127 | AVIV, MooV |
| `ADBEMPEGKeyframeRate` | Key Frame Distance: | signed integer | 1 | 300 | -- | -- | -- | -- | 2 | no | 159 | H264, H26B, HEVC, MP4, flv |
| `ADBEVMC_AutoGOPPlacement` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `ADBEVideoAutoGOPPlacement` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoClosedGOPInterval` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoGOPSettingsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVideoMFrames` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `AllowOpenGOP` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 4 | MX11 |
| `ForceClosedGOP` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 142 | MX10, MX11, MXFX |
| `ForceFixedLengthGOP` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 142 | MX10, MX11, MXFX |
| `VariableLengthGOP` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 4 | MX11 |
| `WMVideoKeyFrameInterval` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |

**facet `layout`** (16 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEAdvancedVideoGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 127 | AVIV, MooV |
| `ADBEAlternatesBasicGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesCompressHeader` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesSettingsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesTabGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesTargetDetailsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAudienceTabGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `ADBEBasicVideoGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 1427 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |
| `ADBECCGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEHeaderType` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 8 | DPX |
| `ADBEStockGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEStockLoginSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEVideoAdvancedSettingsGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 367 | H264, H26B, HEVC, MP4, MX10, MX11, MXFX, WMV |
| `ADBEVideoHinterGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEVideoTabGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 1506 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |
| `EXRSettingsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 8 | oEXR |

**facet `metadata`** (2 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEResynchronizationMarker` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 4 | MP4 |
| `UseZeroTimecode` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 894 | DMXF, JMXF, MX10, MX11, MXFX, PMXF |

**facet `multiplexing`** (10 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEAlternatesStreamingBool` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEBasicMuxGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEMuxDetailsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 90 | H26B, dvd, mbd, mpg2 |
| `ADBEMuxMuxRate` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEMuxPacketSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEMuxPacketsPerPack` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEMuxTabGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEMuxVideoBufferSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVMCMux_Type` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 151 | dvd, hbd, mbd, mpg2 |
| `Container` | -- | multi-instance group (repeating rows) | -- | -- | -- | -- | -- | -- | -- | no | 4 | DCP_ |

**facet `other`** (178 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `0` | -- | opaque / arbitrary data | 0 | 0 | -- | -- | -- | -- | 2 | no | 1541 | AAC, AIFF, AVIV, DCP_, DIBB, DMXF, DPX, GIFf |
| `13` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `201` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `203` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `204` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `206` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `207` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `208` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `209` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `214` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `251` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `253` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `254` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `260` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `261` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `263` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `3001` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 4 | hbd |
| `315` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 55 | hbd, mbd, mpg2 |
| `316` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 55 | hbd, mbd, mpg2 |
| `317` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `318` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `319` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `320` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `324` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `325` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `35` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `36` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `37` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `38` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 33 | hbd, mpg2 |
| `4` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 4 | hbd |
| `407` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `408` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `41` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `412` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `415` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `417` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `434` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `438` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `439` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `448` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `449` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `466` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `468` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `469` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `47` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `470` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `477` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `479` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 4 | hbd |
| `480` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 26 | hbd, mbd |
| `481` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 4 | hbd |
| `50` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 1 | mpg2 |
| `51` | 5.1 | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 29 | mpg2 |
| `52` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `53` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `54` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 75 | dvd, mbd, mpg2 |
| `780` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 26 | hbd, mbd |
| `92` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 74 | dvd, mbd, mpg2 |
| `ADBE10BitBlackPoint` | -- | signed integer | -- | 1023 | -- | -- | -- | -- | -- | yes | 8 | DPX |
| `ADBE10BitWhitePoint` | -- | signed integer | -- | 1023 | -- | -- | -- | -- | -- | yes | 8 | DPX |
| `ADBEAACParametricStereo` | Parametric Stereo | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 134 | AAC, H264, HEVC, MP4 |
| `ADBEAS11ShimIndex` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 4 | MX11 |
| `ADBEAlternatesAlternateBool` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesAutoplay` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesComputerPower` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | yes | 107 | MooV |
| `ADBEAlternatesConnection` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | yes | 107 | MooV |
| `ADBEAlternatesHintedMovieType` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesLanguage` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | yes | 107 | MooV |
| `ADBEAlternatesLoop` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEAlternatesPlatform` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | yes | 107 | MooV |
| `ADBEAlternatesQTVersion` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | yes | 107 | MooV |
| `ADBEAlternatesServerPath` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 107 | MooV |
| `ADBEBlackPoint` | -- | floating point | -- | 1 | -- | -- | -- | -- | -- | yes | 8 | DPX |
| `ADBECCFileSyncFolder` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBECCFileSyncSubFolder` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEDolby_1` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_10` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_11` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_12` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_13` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_14` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_15` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_16` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_17` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_18` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_19` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_2` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_20` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_21` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_22` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | yes | 73 | H264, HEVC |
| `ADBEDolby_23` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_4` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | yes | 73 | H264, HEVC |
| `ADBEDolby_5` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_6` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_7` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_8` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_9` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_Group_0` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_Group_1` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_Group_2` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_Group_3` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_Group_4` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEDolby_Group_5` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 73 | H264, HEVC |
| `ADBEExpandStills` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 127 | AVIV, MooV |
| `ADBEFLVUndershootTarget` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | yes | 12 | flv |
| `ADBEForceTCTimebase` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 53 | MooV |
| `ADBEFrameFieldCoding` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 18 | H26B |
| `ADBEHiddenTicksPerFrame` | -- | boolean | -- | -- | -- | -- | -- | -- | -- | no | 2 | MooV |
| `ADBEHiddenTimeDisplay` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 2 | MooV |
| `ADBEHighlight` | -- | signed integer | -- | 1023 | -- | -- | -- | -- | -- | yes | 8 | DPX |
| `ADBELog` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 8 | DPX |
| `ADBEReorderFrames` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEShimType` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 2 | MX10 |
| `ADBESonyDeviceCompatibility` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 94 | MX10, MX11, MXFX |
| `ADBEStillSequence` | Export As Sequence | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 33 | DIBB, DPX, GIFf, JPEG, PNG, TIFF, TPIC, oEXR |
| `ADBEStockLogin` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEStockRefreshToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEStockStatusText` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBETVStandard` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBETransparencyType` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 8 | GIFf |
| `ADBEUseAlpha` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 6 | PNG, TIFF |
| `ADBEUseIRTStructure` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 122 | MX10, MX11, MXFX |
| `ADBEVMC_Video_M` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 75 | dvd, mbd, mpg2 |
| `ADBEVMC_Video_N` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 75 | dvd, mbd, mpg2 |
| `ADBEVideoAlpha` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 8 | oEXR |
| `ADBEVideoAvgBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 79 | dvd, hbd, mbd, mpg2 |
| `ADBEVideoBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 79 | dvd, hbd, mbd, mpg2 |
| `ADBEVideoBounds` | Video Dimensions | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | H26B, mbd |
| `ADBEVideoDisplayAspect` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVideoEmbedSVCDUserBlks` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoForceVBVDelay` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoGraphicsWhiteLuminance` | -- | signed integer | 0 | 300 | -- | -- | -- | -- | 2 | no | 125 | AVIV, DIBB, DPX, GIFf, H264, HEVC, JMXF, JPEG |
| `ADBEVideoHinterBool` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEVideoHinterInterval` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEVideoHinterPacketDurationLimit` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEVideoHinterPacketSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEVideoHorizontal` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoIgnoreFrameInterval` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoIntraDCPrecision` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoJ2KAlphaEnable` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 22 | JMXF, PMXF |
| `ADBEVideoJ2KDepthAndColorspace` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 22 | JMXF, PMXF |
| `ADBEVideoJ2KLossless` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 22 | JMXF, PMXF |
| `ADBEVideoJ2KParameterVersion` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 22 | JMXF, PMXF |
| `ADBEVideoMacroblockQuantization` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoMatchSource` | -- | button / action (no stored value) | false | false | -- | -- | -- | -- | 2 | no | 984 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, HEVC |
| `ADBEVideoMaxBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 79 | dvd, hbd, mbd, mpg2 |
| `ADBEVideoMinBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 75 | dvd, mbd, mpg2 |
| `ADBEVideoMinFramePercentage` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoNFrames` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVideoNoiseControl` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoPadFramePercentage` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoPayloadEncoding` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 108 | MooV |
| `ADBEVideoPulldown` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVideoResolution` | Resolution | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 639 | DMXF, JMXF, MooV, PMXF |
| `ADBEVideoTransferChars` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoVBVBufferSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoVertical` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoVideoFormat` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoWriteSDE` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEVideoWriteSeqEndCode` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 36 | mpg2 |
| `ADBEWhitePoint` | -- | floating point | -- | 1 | -- | -- | -- | -- | -- | yes | 8 | DPX |
| `AdvancedMXFSettings` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 151 | MX10, MX11, MXFX |
| `BitRate` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 8 | MP3 |
| `CustomEditUnits` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 151 | MX10, MX11, MXFX |
| `EXRBypassLinear` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 8 | oEXR |
| `EXRCompression` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 8 | oEXR |
| `EXRfloat` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 8 | oEXR |
| `EXRlumichrom` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 8 | oEXR |
| `EditUnitsPerPartition` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 151 | MX10, MX11, MXFX |
| `Faked_PAR` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | no | 4 | DCP_ |
| `J2KTile` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 586 | DMXF, JMXF, PMXF |
| `PlayerButton` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | -- | no | 4 | DCP_ |
| `SeparateMonoFiles` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 1 | WAVE |
| `Transcode_VBI_ANC` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 58 | MX10, MX11, MXFX |
| `WMVideoAvgBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |
| `WMVideoBufferSize` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |
| `WMVideoDecoderComplexity` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |
| `WMVideoPeakBitRate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |
| `WMVideoPeakBufferSize` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |

**facet `performance`** (1 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `SmartRender` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 737 | DMXF, JMXF, MX10, MX11, MXFX, PMXF |

**facet `publishing_destination`** (86 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEBehanceDeleteLocalFile` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceDescription` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceLogin` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceLoginSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceRefreshToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceStatusText` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEBehanceTags` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEFTPDeleteLocalFileAfterTransfer` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPPassword` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPPort` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPRemotePath` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPRetries` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPServerAddress` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPTest` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPUserID` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |
| `ADBEFTPVerifyUpload` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEFacebookAccountToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEFacebookDeleteLocalFile` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 21 | H264, MooV, PCM |
| `ADBEFacebookDescription` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | -- | no | 8 | H264, MooV |
| `ADBEFacebookGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 21 | H264, MooV, PCM |
| `ADBEFacebookLogin` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 21 | H264, MooV, PCM |
| `ADBEFacebookLoginSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 21 | H264, MooV, PCM |
| `ADBEFacebookPages` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEFacebookPagesToken` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEFacebookPost` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEFacebookPrivacy` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 10 | H264, MooV |
| `ADBEFacebookRefreshToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 21 | H264, MooV, PCM |
| `ADBEFacebookStatusText` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 21 | H264, MooV, PCM |
| `ADBEFacebookTitle` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEFacebookType` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 2 | MooV |
| `ADBETwitterDeleteLocalFile` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBETwitterDescription` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBETwitterGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBETwitterLogin` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBETwitterLoginSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBETwitterRefreshToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBETwitterStatusText` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 19 | H264, MooV, PCM |
| `ADBEVimeoAuthorizationToken` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | -- | no | 22 | H264, MooV |
| `ADBEVimeoChannel` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEVimeoChannelToken` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEVimeoDeleteLocalFile` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoDescription` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoLogin` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoLoginSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoPassword` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoPasswordToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEVimeoPrivacy` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoRefreshToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEVimeoStatusText` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoTags` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEVimeoTitle` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeAuthCode` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | -- | no | 7 | H264, MooV |
| `ADBEYouTubeCategory` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 22 | H264, MooV |
| `ADBEYouTubeChannel` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeChannelAdd` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeChannelPlaylistGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEYouTubeChannelRemove` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEYouTubeChannelToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeDeleteLocalFile` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeDescription` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeLogin` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeLoginSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubePlaylist` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubePlaylistToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubePrivacy` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeRefreshToken` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeStatusText` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeTags` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 28 | H264, MooV, PCM |
| `ADBEYouTubeTerms` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 2 | MooV |
| `ADBEYouTubeTermsOfServiceGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 2 | MooV |
| `ADBEYouTubeTermsPermissions` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 2 | MooV |
| `ADBEYouTubeTermsPrivacyPolicy` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 2 | MooV |
| `ADBEYouTubeThumbnailFile` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailFileToken` | -- | file or folder path string | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailPoster` | -- | colour | -- | -- | -- | -- | -- | -- | 2 | no | 11 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailSubGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailTime` | -- | boolean | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailTimeBtn` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailTimeToken` | -- | string | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeThumbnailUse` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `ADBEYouTubeTitle` | -- | enumerated integer chosen from a constrained list | -- | -- | -- | -- | -- | -- | 2 | no | 13 | H264, MooV, PCM |
| `PostEncodeHostMultiGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | 2 | no | 55 | DMXF, H264, MXFX, MooV, PCM |

**facet `rate_control`** (34 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEAACAudioQuality` | Audio Quality | signed integer | 0 | 2 | -- | -- | -- | -- | 2 | no | 134 | AAC, H264, HEVC, MP4 |
| `ADBEAlternatesQuality` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | yes | 107 | MooV |
| `ADBEAudioBitrate` | -- | signed integer | 16 | 640 | -- | -- | -- | -- | 2 | no | 386 | AAC, H264, H26B, HEVC, MP4, MooV, WMV, dvd |
| `ADBEAudioBitrateGroup` | Bitrate Settings | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 273 | AAC, H264, H26B, HEVC, MP4, MooV, WMV |
| `ADBEAudioBitrateSettingsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 84 | dvd, flv, mbd, mpg2 |
| `ADBEFLVCodecQuality` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 12 | flv |
| `ADBEFLVVideoBitrateVariabilityTarget` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | yes | 12 | flv |
| `ADBEFLVVideoMaxBitrateTarget` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | yes | 12 | flv |
| `ADBEFLVVideoMinBitrateTarget` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | yes | 12 | flv |
| `ADBEMXFBitrateSettingsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 22 | JMXF, PMXF |
| `ADBEMuxBitrateType` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVideoBitrate` | -- | floating point | -- | -- | -- | -- | -- | -- | 2 | yes | 180 | MooV, dvd, mbd, mpg2 |
| `ADBEVideoBitrateBool` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 6 | MooV |
| `ADBEVideoBitrateEncoding` | Bitrate Encoding | signed integer | 0 | 3 | -- | -- | -- | -- | 2 | no | 274 | H264, H26B, HEVC, MP4, MooV, WMV, dvd, flv |
| `ADBEVideoBitrateGroup` | Bitrate Settings | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 297 | H264, H26B, HEVC, MP4, MooV, WMV, flv |
| `ADBEVideoBitrateLevel` | Bitrate Level | signed integer | 0 | 4 | -- | -- | -- | -- | 2 | no | 274 | H264, H26B, HEVC, MP4, MooV, WMV, dvd, flv |
| `ADBEVideoBitrateSettingsGroup` | -- | tab or section group | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBEVideoDNxHDBitrate` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 270 | DMXF, JMXF |
| `ADBEVideoDNxHRBitrate` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 300 | DMXF |
| `ADBEVideoJ2KMaxBitrate` | -- | floating point | 0.01 | 100000 | -- | -- | -- | -- | -- | yes | 22 | JMXF, PMXF |
| `ADBEVideoMaxBitrate` | Higher value can improve maximum quality, but increase decoder requirements. | floating point | 0.19 | 240 | -- | -- | -- | -- | 2 | yes | 274 | H264, H26B, HEVC, MP4, MooV, WMV, dvd, flv |
| `ADBEVideoMinBitrate` | Higher values set a higher minimum quality, but reduce quality of more difficult scenes. | floating point | 0.19 | 20 | -- | -- | -- | -- | 2 | yes | 274 | H264, H26B, HEVC, MP4, MooV, WMV, dvd, flv |
| `ADBEVideoQuality` | -- | signed integer | 0 | 100 | -- | -- | -- | -- | 2 | yes | 1490 | AVIV, DIBB, DMXF, DPX, GIFf, H264, H26B, HEVC |
| `ADBEVideoTargetBitrate` | The target data rate allowed by the encoder. | floating point | 0.19 | 240 | -- | -- | -- | -- | 2 | yes | 278 | DCP_, H264, H26B, HEVC, MP4, MooV, WMV, dvd |
| `ADBEVideoVBR` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `ADBE_HEVCEncodingSpeedVsQuality` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 20 | HEVC |
| `CodecQuality` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 8 | MP3 |
| `VideoVariableBitrateType` | -- | signed integer | -- | 2 | -- | -- | -- | -- | -- | no | 142 | MX10, MX11, MXFX |
| `WMAudioBitrateMode` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |
| `WMAudioEncodingPasses` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |
| `WMAudioVBRQuality` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 30 | WMV |
| `WMVideoImageQuality` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |
| `WMVideoMaxBitrate` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |
| `WMVideoVBRQuality` | -- | floating point | -- | -- | -- | -- | -- | -- | -- | yes | 30 | WMV |

**facet `video_codec`** (41 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEAudioCodec` | Audio Codec | signed integer | 0 | -1 | -- | -- | -- | -- | 2 | no | 1701 | AAC, AIFF, AVIV, DMXF, H264, H26B, HEVC, JMXF |
| `ADBEAudioCodecGroup` | Audio Codec | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 1421 | AAC, AIFF, AVIV, DMXF, H264, H26B, HEVC, JMXF |
| `ADBEAudioCodecPrefsButton` | -- | button / action (no stored value) | false | false | -- | -- | -- | -- | 2 | no | 1198 | AVIV, DMXF, H264, HEVC, JMXF, MP3, MP4, MX10 |
| `ADBECodecPrefs` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | -- | no | 19 | AVIV, MooV |
| `ADBEDNxHDAlphaType` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 53 | MooV |
| `ADBEFLVSimpleProfile` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 12 | flv |
| `ADBEMPEGAudioDeemphasis` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioEnableCRC` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioEnableCopyrightBit` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioEnableOriginalBit` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioEnablePrivateBit` | -- | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioFormat` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 146 | H264, H26B, HEVC, MP4 |
| `ADBEMPEGAudioFormatGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 146 | H264, H26B, HEVC, MP4 |
| `ADBEMPEGAudioLayer` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioMode` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGAudioPsychModel` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 124 | H264, HEVC |
| `ADBEMPEGCodecBroadcastStandard` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 79 | dvd, hbd, mbd, mpg2 |
| `ADBEMPEGMultiplexer` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 159 | H264, H26B, HEVC, MP4, MooV |
| `ADBEMPEGMuxBasicSettingsGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 146 | H264, H26B, HEVC, MP4 |
| `ADBEMPEGMuxRate` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 18 | H26B |
| `ADBEMPEGMuxStreamCompatibility` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 141 | H264, HEVC, MP4, MooV |
| `ADBEMPEGMuxTabGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 146 | H264, H26B, HEVC, MP4 |
| `ADBEMPEGShortHeader` | -- | container / group header (no value of its own) | -- | -- | -- | -- | -- | -- | -- | no | 4 | MP4 |
| `ADBEMPEGTVStandard` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 139 | H264, H26B, MP4, MooV |
| `ADBEMPEGVideoEncoderNameParam` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 47 | H264, HEVC, MooV |
| `ADBEMPEGVideoEncodingPerformanceParam` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoEncodingPerformanceParamUserSelected` | -- | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEMPEGVideoEncodingSettingsGroup` | -- | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 59 | H264, HEVC, MooV |
| `ADBEVideoCodec` | Video Codec | signed integer | 0 | -1 | -- | -- | -- | -- | 2 | no | 1469 | AVIV, DMXF, GIFf, H264, H26B, HEVC, JMXF, MP4 |
| `ADBEVideoCodecGroup` | Video Codec | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 1394 | AVIV, DCP_, DMXF, GIFf, H264, H26B, HEVC, JMXF |
| `ADBEVideoCodecPrefsButton` | -- | button / action (no stored value) | -- | -- | -- | -- | -- | -- | 2 | no | 190 | AVIV, MooV, dvd, hbd, mbd, mpg2 |
| `ADBEVideoCodec_Unabridged` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 58 | MX10, MX11, MXFX |
| `ADBEVideoMPEGProfile` | Profile | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 160 | H264, H26B, HEVC, MP4, MooV |
| `ADBEVideoMPEGProfileLevel` | Level | signed integer | 0 | 62 | -- | -- | -- | -- | 2 | no | 232 | H264, H26B, HEVC, MP4, MooV, dvd, mbd, mpg2 |
| `ADBEVideoProfile` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 72 | dvd, mbd, mpg2 |
| `ADBE_H26x_Tier` | -- | signed integer | -- | -- | -- | -- | -- | -- | 2 | no | 13 | MooV |
| `ADBE_HEVC_Tier` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 20 | HEVC |
| `AudioCodec` | Audio Codec | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 4 | DCP_ |
| `EXRCompressionLevel` | -- | floating point | -- | 1000 | -- | -- | -- | -- | -- | yes | 8 | oEXR |
| `J2KBroadcastProfileLevel` | -- | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 586 | DMXF, JMXF, PMXF |
| `VideoCodec` | Video Codec | signed integer | -- | -- | -- | -- | -- | -- | -- | no | 4 | DCP_ |

**facet `video_frame`** (5 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEVideoAspect` | Aspect | multi-instance group (repeating rows) | 0,0 | 0,0 | -- | -- | -- | -- | 2 | no | 1506 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |
| `ADBEVideoFPS` | -- | boolean | 1 | 254016000000 | -- | -- | -- | -- | 2 | no | 1506 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |
| `ADBEVideoFieldType` | -- | signed integer | 0 | 2 | -- | -- | -- | -- | 2 | no | 1506 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |
| `ADBEVideoHeight` | -- | signed integer | 4 | 16384 | -- | -- | -- | -- | 2 | no | 1506 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |
| `ADBEVideoWidth` | -- | signed integer | 4 | 30000 | -- | -- | -- | -- | 2 | no | 1506 | AVIV, DCP_, DIBB, DMXF, DPX, GIFf, H264, H26B |

**facet `vr_immersive`** (5 identifiers)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Identifier | Label | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | slider | used by presets | formats |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ADBEVideoVRDoExport` | Video Is VR | container / group header (no value of its own) | false | false | -- | -- | -- | -- | 2 | no | 113 | H264, HEVC, MooV |
| `ADBEVideoVRGroup` | VR Video | tab or section group | 0 | 0 | -- | -- | -- | -- | 2 | no | 113 | H264, HEVC, MooV |
| `ADBEVideoVRHFOV` | Horizontal Field of View | signed integer | 60 | 360 | -- | -- | -- | -- | 2 | no | 97 | H264, HEVC, MooV |
| `ADBEVideoVRStereoscopic` | Frame Layout | signed integer | 0 | 0 | -- | -- | -- | -- | 2 | no | 113 | H264, HEVC, MooV |
| `ADBEVideoVRVFOV` | Vertical Field of View | signed integer | 60 | 180 | -- | -- | -- | -- | 2 | no | 97 | H264, HEVC, MooV |

**[STU-VID-062] Container formats.** 40 distinct container/exporter pairings carry shipped recipes.
The table below is the coverage target, with the shipped recipe count as the weight of each. Studio's
own shipped container set is a dependency decision under [STU-VID-051]; the contract is that each
supported container declares which codecs, colour metadata, caption streams and timecode tracks it
can carry, and that an unsupported combination is refused with a determinate result before encoding
starts rather than failing partway.

*Derivation: catalogue table, splits per row; yields 34 microtasks, one per container.*

| Container fourcc | Exporter class | Shipped presets | Present in |
|---|---|---|---|
| `DMXF` | `MXF` | 152 | premiere, media_encoder |
| `MXFX` | `XDCA` | 117 | premiere, media_encoder |
| `H264` | `NICK` | 43 | premiere, media_encoder |
| `MooV` | `????` | 35 | premiere, media_encoder |
| `MXF` | `P2MX` | 23 | premiere, media_encoder |
| `mpg2` | `LORI` | 18 | premiere, media_encoder |
| `WMV` | `????` | 15 | premiere, media_encoder |
| `mpg2` | `AME` | 14 | premiere, media_encoder |
| `dvd` | `AME` | 12 | premiere, media_encoder |
| `mbd` | `AME` | 11 | premiere, media_encoder |
| `HEVC` | `JEFF` | 10 | premiere, media_encoder |
| `dvd` | `LORI` | 9 | premiere, media_encoder |
| `mbd` | `LORI` | 9 | premiere, media_encoder |
| `H26B` | `NICK` | 9 | premiere, media_encoder |
| `PMXF` | `MXF` | 8 | premiere, media_encoder |
| `AVIV` | `????` | 6 | premiere, media_encoder |
| `flv` | `VLAD` | 6 | premiere, media_encoder |
| `DPX` | `????` | 4 | premiere, media_encoder |
| `oEXR` | `eEXR` | 4 | premiere, media_encoder |
| `MP3` | `????` | 3 | premiere, media_encoder |
| `JMXF` | `MXF` | 3 | premiere, media_encoder |
| `PNG` | `????` | 2 | premiere, media_encoder |
| `hbd` | `AME` | 2 | premiere, media_encoder |
| `MX11` | `AS11` | 2 | premiere, media_encoder |
| `DCP_` | `DTEK` | 2 | premiere, media_encoder |
| `GIFf` | `GIFS` | 2 | premiere, media_encoder |
| `GIFf` | `GIFf` | 2 | premiere, media_encoder |
| `AAC` | `NICK` | 2 | premiere, media_encoder |
| `MP4` | `NICK` | 2 | premiere, media_encoder |
| `AIFF` | `????` | 1 | premiere, media_encoder |
| `DIBB` | `????` | 1 | premiere, media_encoder |
| `JPEG` | `????` | 1 | premiere, media_encoder |
| `TIFF` | `????` | 1 | premiere, media_encoder |
| `TPIC` | `????` | 1 | premiere, media_encoder |
| `WAVE` | `????` | 1 | premiere, media_encoder |
| `MX10` | `AS10` | 1 | premiere, media_encoder |
| `PCM` | `RAWP` | 1 | premiere, media_encoder |
| `AVIV` | `Ucmp` | 1 | premiere, media_encoder |
| `_unresolved_` | `????` | 1 | premiere, media_encoder |

**[STU-VID-063] Export recipes are portable data with stable identity.** 1,541 shipped recipes
resolve to 704 distinct recipe identities -- the same recipe ships in more than one install, so
recipe identity is a stable id, not a name or a file path. A recipe declares `does_video`,
`does_audio`, a metadata-export option, a standard-filter list, and its full parameter set. A user
recipe is the same record as a shipped one.

**[STU-VID-064] Export is queued, headless and observable.** An export is submitted to a queue, runs
without a foreground window, reports determinate progress, is cancellable, and produces a receipt
naming the recipe, the source span, the output artifact and the outcome. A model may submit,
monitor and cancel exports through the same typed surface as an operator ([STU-CON-007], [STU-FX-038]).
Batch export of several sequences or several recipes is one queue with per-item
receipts, not a loop the caller drives.

---

## 14.25.5 Motion-graphics templates as a timeline capability

**[STU-VID-070] A motion-graphics template is a parameterised composition placed as a clip.**
`StudioMotionTemplate` (schema id `hsk.studio.motion_template@1`) packages a `StudioComposition`
([STU-MOT-001]) together with an ordered list of EXPOSED CONTROLS -- a curated subset of the
composition's property tree promoted to a flat inspector. Placing the template on a track creates
an ordinary clip whose inspector shows only the exposed controls; the composition inside is intact
and re-enterable. 77 shipped templates across 10 categories with 461 exposed controls are recovered
as the coverage and shape reference.

**[STU-VID-070a] The exposed-control type system is normative.** Five control types appear across
the 461 recovered controls:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Control type | Count | Contract |
|---|---|---|
| `source_text` | 144 | A localized string bound to a text layer's source text, carrying explicit per-control font-editing permissions: whether the font family, the faux bold/italic style and the font size may be changed by the person using the template, plus the locked font, size, all-caps, small-caps, bold and italic state. A template author can therefore expose the words while locking the typography, and Studio MUST honour that as a hard permission, not a UI hint. |
| `group` | 144 | A section header whose value is the kind of the layer it heads. Structural, no value ([STU-FX-117]). |
| `colour` | 93 | Linear RGBA as four floats in 0..1, with a `StudioColorProfile` reference ([STU-FX-118]). |
| `checkbox` | 51 | Boolean. |
| `numeric_slider` | 29 | Numeric, carrying `min`, `max` and `default`. These are SOFT bounds under [STU-FX-105] -- they are the control's presented range -- and the exposed control declares no hard bound, so `bound_state` is `declared_soft_only` and the underlying property's own hard bounds still govern acceptance. |

**[STU-VID-070b]** Every exposed control carries `can_animate`, which decides whether the control may
be keyframed on the clip. In the recovered set the numeric sliders declare `can_animate = true` and
the source-text controls declare `can_animate = false`; the flag is per-control data, not a rule
per type.

**[STU-VID-070c]** A template declares its required fonts, the effects it uses and the renderer it
expects; opening a template whose requirements are unmet produces a determinate
`TEMPLATE_REQUIREMENT_UNMET` result naming what is missing -- a missing font is never silently
substituted, because silent substitution destroys a layout that was the point of the template.

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Template | Category | Exposed controls | Fonts | Effects used |
|---|---|---|---|---|
| Basic Lower Third | (root) | 2 | 6 | -- |
| Basic Title | (root) | 1 | 4 | -- |
| Bold Broadcast Caption | Captions and Subtitles | 1 | 5 | -- |
| Bold Web Caption | Captions and Subtitles | 1 | 6 | -- |
| Classic Broadcast Caption | Captions and Subtitles | 4 | 4 | -- |
| Classic Web Caption | Captions and Subtitles | 2 | 5 | -- |
| Film Broadcast Caption | Captions and Subtitles | 1 | 4 | -- |
| Film Web Caption | Captions and Subtitles | 1 | 4 | -- |
| Modern Broadcast Caption | Captions and Subtitles | 4 | 5 | -- |
| Modern Web Caption | Captions and Subtitles | 2 | 5 | -- |
| Simple Broadcast Caption | Captions and Subtitles | 4 | 4 | -- |
| Simple Web Caption | Captions and Subtitles | 2 | 4 | -- |
| Angled Credits | Credits | 7 | 4 | -- |
| Bold Credits | Credits | 7 | 5 | -- |
| Classic Credits | Credits | 7 | 4 | -- |
| Film Credits | Credits | 7 | 4 | -- |
| Modern Credits | Credits | 7 | 5 | -- |
| Angled Coming Up Next | Graphic Overlays | 1 | 6 | -- |
| Angled Live Overlay | Graphic Overlays | 1 | 4 | -- |
| Bold Coming Up | Graphic Overlays | 3 | 5 | -- |
| Bold Live Overlay | Graphic Overlays | 3 | 5 | -- |
| Bold Tap to Hear | Graphic Overlays | 3 | 5 | -- |
| Classic Coming Up Next | Graphic Overlays | 2 | 5 | -- |
| Classic Live Overlay | Graphic Overlays | 2 | 4 | -- |
| Film Coming Up | Graphic Overlays | 1 | 4 | -- |
| Film Live Overlay | Graphic Overlays | 1 | 4 | -- |
| Modern Coming Up Next | Graphic Overlays | 3 | 5 | -- |
| Modern Live Overlay | Graphic Overlays | 3 | 5 | -- |
| Angled Image Caption | Lower Thirds | 1 | 5 | -- |
| Angled Lower Third | Lower Thirds | 1 | 5 | -- |
| Bold Image Caption | Lower Thirds | 3 | 5 | -- |
| Bold Lower Third Left | Lower Thirds | 3 | 5 | -- |
| Bold Lower Third Right | Lower Thirds | 3 | 5 | -- |
| Classic Image Caption | Lower Thirds | 2 | 4 | -- |
| Classic Lower Third One Line | Lower Thirds | 2 | 4 | -- |
| Classic Lower Third Two Lines | Lower Thirds | 3 | 5 | -- |
| Film Caption | Lower Thirds | 1 | 4 | -- |
| Film Lower Third Left | Lower Thirds | 1 | 4 | -- |
| Film Lower Third Left Two Line | Lower Thirds | 2 | 4 | -- |
| Film Lower Third Right | Lower Thirds | 1 | 4 | -- |
| Film Lower Third Right Two Line | Lower Thirds | 2 | 4 | -- |
| Modern Image Caption | Lower Thirds | 3 | 5 | -- |
| Angled Slate | Slates | 4 | 6 | -- |
| Bold Slate | Slates | 4 | 4 | -- |
| Classic Slate | Slates | 4 | 6 | -- |
| Film Slate | Slates | 4 | 4 | -- |
| Modern Slate | Slates | 4 | 5 | -- |
| Like | Social Media | 2 | 5 | -- |
| Share | Social Media | 2 | 4 | -- |
| Subscribe | Social Media | 2 | 5 | -- |
| Angled Presents | Titles | 1 | 4 | -- |
| Angled Title | Titles | 1 | 6 | -- |
| Bold Presents | Titles | 3 | 5 | -- |
| Bold Title | Titles | 3 | 5 | -- |
| Classic Logo Presents | Titles | 2 | 4 | -- |
| Classic Title | Titles | 11 | 6 | -- |
| Film Presents | Titles | 1 | 4 | -- |
| Film Title | Titles | 2 | 4 | -- |
| Modern Presents | Titles | 3 | 5 | -- |
| Modern Title | Titles | 5 | 6 | -- |
| Sports Graphic Overlay | [AE] Sports Package | 9 | 5 | ADBE Checkbox Control, ADBE Color Control, ADBE Drop Shadow, ADBE Slider Control, ADBE Tritone |
| Sports Intro | [AE] Sports Package | 17 | 6 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Gaussian Blur 2 |
| Sports Logo Loop | [AE] Sports Package | 6 | 0 | ADBE Camera Lens Blur, ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow |
| Sports Looping Background | [AE] Sports Package | 20 | 6 | ADBE Camera Lens Blur, ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow |
| Sports Lower Third Center | [AE] Sports Package | 25 | 7 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE MESH WARP |
| Sports Lower Third Side | [AE] Sports Package | 12 | 6 | ADBE Checkbox Control, ADBE Color Control, ADBE Drop Shadow, ADBE Slider Control, ADBE Tint |
| Sports Scoreboard | [AE] Sports Package | 37 | 7 | ADBE Camera Lens Blur, ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow |
| Sports Text Background | [AE] Sports Package | 20 | 6 | ADBE Camera Lens Blur, ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow |
| Sports Transition | [AE] Sports Package | 6 | 0 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Gaussian Blur 2 |
| Gaming Background Loop | [AE] Video Gaming Package | 13 | 1 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Gaussian Blur 2 |
| Gaming Bullet List | [AE] Video Gaming Package | 35 | 6 | ADBE Box Blur2, ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow |
| Gaming Graphic Overlay | [AE] Video Gaming Package | 9 | 5 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Glo2, ADBE Ramp |
| Gaming Intro | [AE] Video Gaming Package | 22 | 6 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Gaussian Blur 2 |
| Gaming Logo Loop | [AE] Video Gaming Package | 28 | 6 | ADBE Box Blur2, ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow |
| Gaming Lower Third Left | [AE] Video Gaming Package | 14 | 6 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Gaussian Blur 2 |
| Gaming Lower Third Right | [AE] Video Gaming Package | 14 | 6 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Gaussian Blur 2 |
| Gaming Transition | [AE] Video Gaming Package | 5 | 0 | ADBE Checkbox Control, ADBE Color Control, ADBE CurvesCustom, ADBE Drop Shadow, ADBE Geometry2 |
---

## 14.25.6 Declared gaps

**[STU-VID-080] GAP -- editing-mode constraint lists.** Per [STU-VID-017a], all 53 modes ship with
`constraints_known = false`. Authoring the allow-lists is a deliberate per-mode task. Studio
enforces nothing until they exist and MUST NOT infer them from preset usage.

**[STU-VID-081] GAP -- time-display code 113.** Five shipped configurations declare it; the
enumeration has no member for it. Not guessed.

**[STU-VID-082] GAP -- immersive projection-type and stereoscopic-type enumerations.** The fields
are specified and typed; their member sets were not recovered and are authored deliberately.

[STU-VID-083] **GAP -- 178 export parameter identifiers with no determined facet, and 409 with no
shipped label.** Each needs classification and an authored label before it can be exposed.

**[STU-VID-084] GAP -- codec adapter selection.** [STU-VID-051] specifies the descriptor, the
enumeration and the failure contract but NOT which codecs Studio ships. That is a licensing and
dependency decision requiring a named operator decision.

**[STU-VID-085] GAP -- caption format support.** Caption tracks are specified as a track kind
([STU-VID-037]); which caption interchange formats Studio reads and writes, and the per-format
styling model, are not specified here.

[STU-VID-086] **GAP -- the relationship to `engine.director` (Spec 11.8) beyond the ownership
statement of [STU-VID-001a].** The interface by which `engine.director` consumes a `StudioSequence`
is not specified in this sub-section and requires a cross-section decision.

---

## 14.25.7 Model steerability, GUI, diagnostics and manual obligation

**[STU-VID-090]** Every panel, control, monitor, track header, clip, edit point and visible state in
this sub-section MUST be model-visible and typed-steerable through the Studio command surface
(14.16); MUST be headlessly inspectable, steerable and screenshot-capturable through Argus with no
foreground focus steal (14.20); and MUST ship dual-audience UserManual entries -- an operator layer
and a model layer carrying command ids, typed I/O, receipts, undo semantics, Argus targets and
failure/recovery -- kept same-change current (14.22). Three obligations are specific to the clip
timeline:

1. **Every trim operation MUST have a headless form that takes an explicit delta.** A trim that
   exists only as a drag is not model-steerable. The typed form takes the edit point, the mode
   ([STU-VID-026b]) and a signed tick or frame delta, and returns the achieved delta, which may be
   smaller than requested when a limit was reached.
2. **The Argus diagnostic for a sequence MUST expose the edit decision list** -- every clip's
   source reference, source window, timeline position, speed and track -- as structured data, so a
   visual regression in a rendered frame can be traced to an edit decision rather than guessed at.
3. **A determinate result is required for every refusal.** Insufficient handle, sync-lock conflict,
   source exhaustion, missing codec, missing font, offline media and unmet template requirements
   each have a named result naming the specific obstruction. "The operation failed" is not an
   acceptable outcome anywhere in this sub-section.

---

## 14.25.8 Microtask Derivation

**[STU-VID-100] Microtask derivation index.** Applying the shared derivation convention to this
sub-section yields exactly 163 microtasks. The correspondence is NORMATIVE and CLOSED: a microtask
corresponds to a yielding clause or to a table unit as marked, and to nothing else.

Rule 0 -- derivation markers are authoritative. Every table in this sub-section carries an italic
`*Derivation: ...*` marker sentence directly above it stating how many microtasks that table yields.
The marker is normative. A tool that classifies a table differently from its marker has diverged
from this sub-section and MUST be corrected to the marker, not the reverse. The five marker forms
are: parameter table taken whole (1); enumeration table taken whole (1); preset or command table
taken whole (1); catalogue table splitting per row (N); contract table carried into the clause's own
microtask (0). A sixth form, reading aid inside a non-yielding clause, also yields 0.

Rule A -- one microtask per yielding clause. Every numbered clause yields exactly one microtask
EXCEPT the members of the no-yield set of [STU-VID-100a]. A sub-lettered anchor
([STU-VID-020a], [STU-VID-031a], [STU-VID-061b]) is a clause for this purpose and yields on its own account.

Rule B -- table units, counted from the markers of rule 0. A parameter table is a unit in its own
right even though it sits inside a clause that is also a unit, because its rows are bound-sets that
have to be individually proven. An enumeration table is a unit for the same reason, its members being
the criteria. A catalogue table splits because each row names a separately implementable subject --
one trim operation, one export facet, one container. A contract table does not split and is not its
own unit: it describes the fields of the single contract its clause already defines.

Three counts in this sub-section are traps for a tool that reads structurally rather than reading
the markers:

1. **The 15 command tables of [STU-VID-041] hold 676 rows and yield 15, not 676.** One table is one
   command family; a row is a command id, a label and a default binding, and it is an acceptance
   criterion of its family's microtask. Splitting per row would produce a microtask per keystroke.
2. **The 392 sequence presets of [STU-VID-016a], the 53 editing-mode rows of [STU-VID-017a], the 39
   importer modules of [STU-VID-050], the 58 codec modules of [STU-VID-051] and the 77 templates
   of [STU-VID-070c] are shipped DATA over a contract that a clause already states.** Each table yields
   1: ship the library and prove it round-trips. Their rows are inventory, not subjects.
3. **The export surface yields on two axes and both are real.** [STU-VID-061]'s facet table splits
   per facet (14), because a facet is the organising axis of the export inspector and each one is a
   separately buildable surface; the 14 per-facet parameter tables then yield 1 each, because
   proving 453 identifier bound-sets is a different obligation from building the inspector. 14 plus
   14, not 14 and not 453.

**[STU-VID-100a] The no-yield set: 14 clauses.** Nothing else may be excluded, and a clause not on
this list yields under rule A whether or not it is convenient.
In this list a MEMBER of the set is written in backticks, as `STU-AREA-nnn`, and an anchor written
in brackets, as [STU-AREA-nnn], is a REFERENCE and is not excluded from anything. The two forms
are visually distinct so that a reader and a tool can both count the members without parsing the
surrounding English.

The members:

1. **Supersession.** `STU-VID-001`, `STU-VID-001a` (whose what-it-said / what-now-holds table is a
   supersession record) and `STU-VID-001b` (the domain-list amendment).
2. **Ownership and authority.** `STU-VID-002` and `STU-VID-003`.
3. **Restatement of an obligation every microtask inherits.** `STU-VID-040`: every operation in this
   sub-section is one reversible `StudioHistory` step. That attaches to all 168 microtasks by
   reference.
4. **Declared-gap register rows whose gap is already stated by a yielding clause.** `STU-VID-080`
   points at [STU-VID-017a] and `STU-VID-081` at [STU-VID-012b].
5. **This derivation section.** `STU-VID-100`, `STU-VID-100a`, `STU-VID-101`, `STU-VID-102`,
   `STU-VID-103` and `STU-VID-104`.

Clause [STU-VID-090] is NOT in the no-yield set: its lead paragraph restates the steerability law, but it
carries three obligations specific to the clip timeline -- a headless trim form taking an explicit
delta, the Argus edit-decision-list diagnostic, and a determinate result for every refusal -- and
those are real, provable work. Tables inside a non-yielding clause yield nothing.

Timeline AUDIO ROUTING is specified but yields in 14.9, not here. Track volume, pan, mute, solo,
submixes, sends, master channel configuration, channel layouts and sample rate are stated
by [STU-FX-137] and reached from [STU-VID-021]'s track record. They are neither restated nor missing,
and a derivation that expects them in this ledger is looking in the wrong sub-section.

**[STU-VID-101] Microtask content obligation.** A microtask derived under [STU-VID-100] MUST carry
into its own body: the clause anchor, or the catalogue row and its table; for an editing operation,
its typed command id, its history semantics and its named illegal-result behaviour
([STU-VID-026], [STU-VID-090]) -- an operation microtask without its refusal contract is not implementable; the
complete member list of every enumeration it touches with the shipped codes preserved
([STU-VID-012a]); the full field list of every record it touches; and for any numeric parameter, all
seven fields of [STU-FX-105] separately with every undeclared side left `--` per [STU-VID-061b]. No
microtask may cite the green-room corpus as its source of truth ([STU-SECTION-002]).

**[STU-VID-102] Ledger.**

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Ledger line | Basis | Yields |
|---|---|---|
| Clauses in 14.25 | anchors 001 through 104, sub-lettered anchors included | 82 |
| less the no-yield set | the 14 clauses of [STU-VID-100a] | -14 |
| **Rule A subtotal** | one microtask per yielding clause | **68** |
| Parameter tables | 14 tables: the per-facet export parameter tables of 061b, each taken whole | 14 |
| Enumeration tables | 4 tables: the tick-rate table of 013, the time-display codes of 012a, the frame-interpolation methods of 031, the exposed-control types of 070a | 4 |
| Preset, command and inventory tables | 20 tables: 15 command families of 041, sequence presets of 016a, editing modes of 017a, importer modules of 050, codec modules of 051, shipped templates of 070c | 20 |
| Catalogue: trim operations of 026 | one per operation | 9 |
| Catalogue: export facets of 061 | one per facet | 14 |
| Catalogue: containers of 062 | one per container; the 39 rows are container/exporter pairings and five containers are written by two exporter classes, which is one container to support and two provenance rows | 34 |
| Contract tables | 4 tables carried into the owning clause's microtask: sequence settings of 011, the clip record of 020, the two-surface comparison of 022, the rename provenance of 041a | 0 |
| Reading aids in non-yielding clauses | 2 tables: the supersession table of 001a and this ledger | 0 |
| **Rule B subtotal** | table units | **95** |
| **Total microtasks yielded by 14.25** | rule A plus rule B | **163** |

**[STU-VID-103] An open item or a blocked dependency does NOT remove a microtask.** A clause that
declares a gap, an unrecovered enumeration, an undetermined facet or an unresolved adapter choice
still yields its rule-A microtask, and that microtask's FIRST acceptance row MUST read "the named gap
is raised to the operator as a capture request and is NOT closed by an invented value". The clauses
carrying a declared gap or open decision are [STU-VID-012b] (time-display code 113), [STU-VID-017a]
(the editing-mode constraint lists), [STU-VID-061a] (labels for 409 unlabelled
identifiers), [STU-VID-061b] (the absent soft ranges, defaults and units on all 453 export
identifiers), [STU-VID-082] (the immersive projection-type and stereoscopic-type members), [STU-VID-083] (178
facet-less identifiers), [STU-VID-084] (codec adapter selection), [STU-VID-085] (caption formats)
and [STU-VID-086] (the relationship to `engine.director`). Nothing may disappear from the work
because it is not yet answerable.

**[STU-VID-104] Anchor binding.** A microtask derived from this sub-section cites the clause anchor
directly, and a catalogue microtask additionally cites its row and the table it came from. A
microtask staged before this sub-section landed carries `spec_anchor_status = "PROVISIONAL"`;
binding it to an anchor here clears that status. A microtask that cannot cite an anchor in this
sub-section is out of scope for the timeline domain and MUST be re-derived or retired, not activated.
