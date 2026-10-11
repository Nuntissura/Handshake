---
file_id: BUILDER-BRIEF-LANE-REEL-ENG-1
file_kind: builder_brief
updated_at: 2026-10-11T12:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-reel
lane: SMB-REEL-ENG-1
depends_on: [reel @ e8b464b (reel sources identical to f6e5a49), accord Timescale/Rounding]
sequencing: starts only after SMB-REEL-AV1-ROUTE has a verdict, or the lead lists these commits as part of that claim (V4-14); one builder per crate, so SMB-REEL-ENG-2 follows this lane in the same crate
author_role: researcher (no product code written; no cargo run)
---

<topic id="finding" status="active" summary="Lane scope is ~90% already implemented; this brief covers only the verified gap">

## Finding: most of the lane scope exists. Verified against the product tree (reel, HEAD e8b464b)

Lane scope text (ledger): "own ISOBMFF/MOV index reader (edit lists, ctts, tmcd) + index-first frame-accurate seek returning requested vs actual PTS; exact-or-error in export mode".

| Scope item | Status | Evidence (crates/hsk-studio-reel) |
|---|---|---|
| own ISOBMFF/MOV index reader | DONE | `src/isobmff.rs:1-31` module doc, `build_index` `:1444`; fMP4 `moof/traf/trun/sidx` `src/isobmff/fragments.rs`; `#![deny(clippy::arithmetic_side_effects)]` `:15` |
| edit lists (`elst` v0/v1, empty edits, media_time, unaligned edit start) | DONE | `build_edits` `:1200-1274`, `build_presentation` `:1276-1348` (`trim_start` for an edit that starts inside a sample); tests `tests/foundation_contract.rs:331-346` (leading empty edit), `tests/v4_hardening.rs:843-875` (extremes) |
| `ctts` (v0 read as signed, v1 signed, negative composition offsets) | DONE | `:1022-1035`, per-sample `pts = dts + offset` `:905-927`; test `tests/foundation_contract.rs:102-132,135-256` (open-GOP B-frame table), `tests/v4_hardening.rs:877-` (i32 extremes) |
| `tmcd` timecode track (raw fields + start sample; label rendering deliberately left to pulse) | DONE | `src/isobmff/timecode.rs:26-41` (flags 0x1/0x2/0x4/0x8), `:58-99` `read_timecode_start`; test `tests/slice2_contract.rs:132` |
| index-first seek returning requested vs actual PTS | DONE | `TrackIndex::seek` `:368-415` -> `SeekResult{requested,target,decode_from,samples_to_decode,trim_start,disposition}` (`:134-147`); `Exact` vs `Containing`; time outside the presented range is `OutOfRange`, never clamped (`:365-367`); open-GOP conservative decode start `sync_for` `:345-363`; packet delivery receipt `requested` vs `actual` `Span` + `Disposition::{Exact,Approximate}` `deliver_packets` `:1776-1911` |
| index built once per source revision and reused | DONE (in memory) | `src/cache.rs` `IndexCache` keyed (content id, revision, length, limits); test `tests/slice2_contract.rs:445` |
| hostile-input bounds | DONE | `tests/v4_hardening.rs` `isobmff_dims_u32_max` `:759`, `isobmff_aggregate_over_budget` `:934`, fragments `:1102,:1201`; `tests/v4_alloc_budget.rs` |
| **exact-or-error in export mode** | **MISSING** | `Purpose::Export` is read in exactly one place: `src/registry.rs:242` (refuses a PROXY preference). No provider refuses an unaligned start under `Export`: `deliver_packets` returns `Ok` with `Disposition::Approximate` (`isobmff.rs:1893-1897`); same in `y4m.rs:300,392`, `wav.rs:297,392`, `mxf/provider.rs:94,250`, `isobmff/pcm.rs:436,550`; Matroska reuses `deliver_packets` (`matroska.rs:1588`). A compressed-video decode that drops pictures returns `Ok` with `Disposition::Partial` + `complete=false` (`video.rs:511-514`) even for `Export` |

Additional verified gaps (smaller, same crate, same files):
- G2: an `elst` entry with `segment_duration == 0` is not special-cased. `build_presentation` stops at `pts >= presentation_start + duration` (`isobmff.rs:1325-1329`), so a zero-duration non-empty edit presents NOTHING and every seek is `OutOfRange`. ISO/IEC 14496-12 allows `segment_duration` 0 for the initial edit of a fragmented file (source: W3C bug 26066 quoting the 2012 edition, see anchors). No test pins this (`grep edits tests/` shows no zero-duration case).
- G3: sample-entry children are read for `colr`/`pasp`/`av1C` only (`parse_visual_children` `:687-732`). `fiel` (field order), `clap` (clean aperture) are not indexed; ProRes/interlaced consumers (lane SMB-REEL-ENG-2) need them as container-level fallbacks.
- Not gaps, kept as typed `Unsupported` by decision: edit media rate other than 1.0 (`:1246-1251`; Apple documents a relative rate that "cannot be 0 or negative", so valid but rare), `stz2`, multiple sample descriptions per track.

Everything else in the lane scope is closed. Do not re-implement or refactor it.

</topic>

<topic id="purpose" status="active" summary="What this lane delivers">

## Purpose (gap only)
1. Export is exact or it fails with a typed error, before the first byte reaches the sink (G1).
2. A zero-duration edit in a fragmented file keeps its presentation (G2).
3. `fiel` and `clap` are captured as raw, uninterpreted fields (G3), no behaviour change for existing callers.
4. One independent-writer oracle (ffmpeg-made MOV) pins edit/ctts/tmcd semantics, because every existing container fixture is built by this repo's own synthetic builder (`tests/common/mov.rs`), which cannot catch a symmetric misreading.

</topic>

<topic id="spec-anchors" status="active" summary="Sources actually opened 2026-10-11 and what each fixed">

## Spec anchors (opened this session unless marked)
- Apple QuickTime File Format, edit list atom: https://developer.apple.com/documentation/quicktime-file-format/edit_list_atom.md and https://developer.apple.com/documentation/quicktime-file-format/edit_list_atom/edit_list_table.md . Facts used: track duration is in the MOVIE timescale; media time -1 is an empty edit; "the last edit in a track should never be an empty edit"; media rate is a relative rate that cannot be 0 or negative.
- Composition offset atom: https://developer.apple.com/documentation/quicktime-file-format/composition_offset_atom.md . `CT(n) = DT(n) + CTTS(n)`, delta "can be positive or negative".
- Composition shift least greatest atom (`cslg`): https://developer.apple.com/documentation/quicktime-file-format/composition_shift_least_greatest_atom.md . Optional summary atom; NOT needed (the index is built from stts/ctts directly).
- Timecode sample description: https://developer.apple.com/documentation/quicktime-file-format/timecode_sample_description.md and `/flags.md` (drop frame 0x0001, 24-hour max 0x0002, negative times OK 0x0004, counter 0x0008).
- Colour parameter atom (`colr`): https://developer.apple.com/documentation/quicktime-file-format/color_parameter_atom.md (video-range mapping 10-bit Y' 64-940, Cb/Cr 65-960).
- Clean aperture (`clap`): https://developer.apple.com/documentation/quicktime-file-format/clean_aperture.md : eight signed 32-bit fields `apertureWidth_N/_D, apertureHeight_N/_D, horizOff_N/_D, vertOff_N/_D`.
- Field/frame information (`fiel`): Apple TN2162 https://developer.apple.com/library/archive/technotes/tn2162/_index.html : two u8 fields, `fields` (1 progressive, 2 interlaced) and `detail` (1, 6, 9, 14 for interlaced). The older QTFF text for 9/14 was reported reversed by an Apple engineer (ffmpeg-devel thread https://lists.ffmpeg.org/pipermail/ffmpeg-devel/2017-August/214848.html : search snippet only, thread not opened). Therefore this lane stores the two bytes RAW and ships no interpretation. OBSERVED (ffmpeg-written ProRes MOVs from this research, `engine-video-oracle/cmd-check/final/`): progressive `fiel` = (1, 0), top-field-first = (2, 9), bottom-field-first = (2, 14), matching TN2162's table and the ProRes frame header's own `interlace_mode` (1 / 2). The writer is ffmpeg, so this is not Apple-verified; lane SMB-REEL-ENG-2 asserts these values on its interlaced fixtures.
- Zero `segment_duration`: W3C bug https://www.w3.org/Bugs/Public/show_bug.cgi?id=26066 (opened) quoting ISO/IEC 14496-12: in an empty initial movie of a fragmented file the `segment_duration` of a non-empty edit may be zero. Secondary source; the ISO text itself was not opened.
- ISO/IEC 14496-12 itself: NOT opened. The ISO "publicly available standards" page https://standards.iso.org/ittf/PubliclyAvailableStandards/index.html (opened) says the ITTF site is closed and free deliverables moved to the ISO/IEC webstores; it does not list 14496-12. Free availability is UNVERIFIED.
- Master spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`): STU-VID-051 (l.24559), STU-VID-038 (l.23653), STU-ARC-015 (l.225), STU-ARC-021 (l.251: exact requested vs actual counts, no silent clamp/truncation). Prep MT-35542 (requested vs actual PTS, explicit disposition). Research `research/engine-video.md` topic improvements item 2 ("exact or error in export mode").

</topic>

<topic id="slices" status="active" summary="Minimal first slice and later slices">

## Slices
**Slice 1 (the whole lane, small).**
1. Typed errors (additive; wire strings stable): `ReelError::NotExact { requested: Ticks, nearest: Ticks }` -> `"MEDIA_NOT_EXACT"`; `ReelError::Incomplete { requested: u64, delivered: u64 }` -> `"MEDIA_INCOMPLETE"`. Add both to `error.rs` `code()`, `Display`, `report.rs` outcome mapping (Failure(Loss)), and the `DESCRIPTOR` error list in `lib.rs`. `reel-native` matches `ReelError` with a wildcard (`reel-native/src/wire.rs:415-418`), so adding variants does not break it.
2. One shared guard in `model.rs`: `pub(crate) fn require_exact_start(req: &SegmentRequest, exact: bool, nearest: Ticks) -> Result<(), ReelError>` that returns `NotExact` when `req.purpose == Purpose::Export && !exact`. Call it BEFORE the first read/`sink.accept` at: `isobmff.rs:1786` (after `track.seek`, `exact = first.disposition == Exact && first.trim_start == 0`; covers Matroska via `deliver_packets`), `isobmff/pcm.rs:436`, `y4m.rs:300`, `wav.rs:297`, `mxf/provider.rs:94`. `Purpose::Preview` behaviour is unchanged (`Approximate` stays `Ok`).
3. `video.rs:511-514`: when `delivered < req.count` and `req.purpose == Export`, return `Incomplete { requested, delivered }` instead of `Ok(Partial)`. Chunks already handed to the sink stay delivered; the error is the statement that the export must not publish them (STU-ARC-021).
4. Tick rounding is NOT an export refusal: `ticks_exact == false` (e.g. 100 ns timescale) stays an `Ok` receipt flag, as today (`tests/foundation_contract.rs:348-399`). Decision recorded here so no builder "fixes" it.
5. Zero-duration edits in `build_edits` (needs the `fragmented` flag threaded from `build_index` `:1471` into `finish_track` `:1184`): empty edit (`media_time == -1`) with duration 0 = no-op, skipped; non-empty edit with duration 0 that is the LAST entry of a FRAGMENTED file = open-ended (`duration = i64::MAX`, same representation as the no-`elst` case `:1216-1220`); any other zero-duration non-empty edit = `Unsupported { what: "elst-zero-duration" }` (never guessed).
6. Raw metadata: `pub struct FieldInfo { pub fields: u8, pub detail: u8 }`, `pub struct CleanAperture { pub width: (i32, i32), pub height: (i32, i32), pub horiz_off: (i32, i32), pub vert_off: (i32, i32) }` (numerator, denominator), `TrackIndex.field_info: Option<FieldInfo>`, `TrackIndex.clean_aperture: Option<CleanAperture>`, parsed in `parse_visual_children`. `TrackIndex` has private fields, so it cannot be built outside the crate and the new public fields are additive. A malformed child ends the scan (existing rule `:687-688`), never fails the track. A zero denominator is stored as read; no division happens in reel.

**Later slices (not this lane; new lane ids per V4-15).**
- Edit media rate other than 1.0 (scale presented duration; `EditSegment.rate`).
- `mdcv`/`clli` HDR static metadata: layout not opened, so excluded until a primary source is opened.
- Persisting the index to disk: needs a serialization owner (session/package), not reel.
- Multi-sample `tmcd` tracks (timecode changes per edit): `read_timecode_start` reads only the first sample.

</topic>

<topic id="api" status="active" summary="Public API delta">

## API delta (everything else unchanged)
```rust
// error.rs
pub enum ReelError { /* existing */ NotExact { requested: Ticks, nearest: Ticks }, Incomplete { requested: u64, delivered: u64 } }
// isobmff.rs (re-exported from lib.rs)
pub struct FieldInfo { pub fields: u8, pub detail: u8 }
pub struct CleanAperture { pub width: (i32, i32), pub height: (i32, i32), pub horiz_off: (i32, i32), pub vert_off: (i32, i32) }
impl TrackIndex { /* pub fields */ field_info: Option<FieldInfo>, clean_aperture: Option<CleanAperture> }
```
Behaviour contract: `Purpose::Export` + (start not frame/sample aligned | edit-trimmed first sample | fewer units delivered than requested) -> typed error, sink untouched for the first two.

</topic>

<topic id="hostile" status="active" summary="V4-01/02 bounds for the changed code">

## Hostile-input bounds
- All new parsing stays inside `isobmff.rs`, which carries `#![deny(clippy::arithmetic_side_effects)]` (V4-01). `fiel` needs 2 bytes, `clap` 32 bytes: check `payload.len()` first; no allocation depends on a field.
- `elst` zero-duration handling adds no loop and no allocation. Existing edit-count bound (4096, `:1269`) and `Limits::max_edit_entries` stay the aggregate bound (V4-02 `aggregate_over_budget`-style test already exists for edits: `tests/v4_hardening.rs:1003`); extend that test with one zero-duration entry rather than adding a new one.
- Guard errors carry only `Ticks`/counts (no file names, no parser text).

</topic>

<topic id="tests" status="active" summary="At most 5 focused tests, fixtures and exact commands">

## Tests (at most 5; `bounded_case`; synthetic builders from `tests/common/mov.rs`, which already accept `edits` for MOV and fMP4)
1. `export_is_exact_or_error_for_every_provider`: for each decode-capable provider id in `Registry::standard()` plus `MxfProvider`, one unaligned-start request: `Preview` -> `Ok` + `Approximate`; `Export` -> `NotExact { nearest }` with `NeverReached` sink (existing helper). The test ENUMERATES `descriptors()` and fails when a decode-capable provider has no case (a new provider cannot silently skip the rule). Aligned `Export` request -> `Ok` + `Exact`.
2. `export_refuses_partial_video_decode`: reuse the stub `ExternalVideoDecoder` `Fake` in `tests/video_route.rs:76-129` and add a `Plan::DropPicture(tag)` variant to its `Plan` enum (`:60-74`) so one picture is never released: `Preview` -> `Ok` + `Partial` + `complete == false`; `Export` -> `Incomplete { requested: 3, delivered: 2 }`.
3. `fragmented_zero_duration_edit`: `FragSpec` with `edits: Some(vec![(0, 2002)])` presents the same PTS list as the same spec with a full-length edit; progressive MOV with `edits: Some(vec![(0, 2002)])` -> `Unsupported { what: "elst-zero-duration" }`; `(0, -1)` followed by a normal edit = same presentation as without the empty entry.
4. `fiel_and_clap_are_raw_and_advisory`: extend `Mp4Spec` with `fiel: Option<(u8, u8)>` and `clap: Option<[i32; 8]>`; values round-trip raw (`fields=2, detail=14`; negative offsets); a truncated `clap` (24 bytes) leaves `clean_aperture == None` and the track indexable.
5. `ffmpeg_made_mov_matches_independent_oracle` (fixture below, committed, no test runs ffmpeg): `presented()` PTS list in ticks equals `n * 8_475_667_200` for n in 0..24; decode-order PTS (units of 1/30000) are `0,3003,1001,2002,6006,4004,5005,9009,7007,8008,11011,10010,12012,15015,13013,14014,18018,16016,17017,21021,19019,20020,23023,22022` (ffprobe values; ffmpeg's own dts shift by -1001 is NOT used); sync samples `[0, 12]`; a `ctts` offset of -1001 exists (v1); `read_timecode_start` -> `raw_sample == 107891`, `drop_frame`, `timescale 30000`, `frame_duration 1001`, `frames_per_second 30`. 107891 was derived independently: 00:59:59;29 drop-frame at 29.97 = 1800*59 + 30*59 + 29 - 2*(59 - 59/10) = 107891, and read from the file with `ffprobe -select_streams 1 -show_packets -show_data` (`0001a573`).

Fixture (authored once under the artifact root, then committed per the existing fixture convention in `tests/fixtures/README.md`: synthetic, < 200 KB, generator + oracle recorded, ffmpeg never a test-time dependency):
- Folder: `tests/fixtures/mov/` (new), file `mov-h264-bf2-negcts-tc-df-64x36.mov`, 4197 bytes, sha256 `4480db23d496dedb5dc7608ab6b9b21627dd94900221f018e03d7e6bf1bce418`.
- Artifact root for authoring: `D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/engine-video-oracle/` (this is the existing artifact root; the path `.../Handshake/Handshake_Artifacts/...` named in the dispatch does not exist). A reference copy of the final fixtures already sits in `engine-video-oracle/cmd-check/final/`.
- Generator (confirmed to run and to be byte-reproducible with ffmpeg 8.1.1 gyan.dev full build; regenerating gave the same sha256 for the sibling H.264 fixture):
  `ffmpeg -hide_banner -y -f lavfi -i "testsrc2=size=64x36:rate=30000/1001" -frames:v 24 -c:v libx264 -preset veryfast -profile:v high -pix_fmt yuv420p -bf 2 -g 12 -x264-params "threads=1:open-gop=0:scenecut=0:no-info=1" -fflags +bitexact -flags:v +bitexact -movflags +negative_cts_offsets -timecode "00:59:59;29" mov-h264-bf2-negcts-tc-df-64x36.mov`
- Oracles: `ffprobe -v error -select_streams v:0 -show_entries packet=pts,dts,flags -of csv=p=0 FILE` and `ffprobe -v error -select_streams 1 -show_packets -show_data FILE`. Record both outputs in `tests/fixtures/mov/provenance.json` with the ffmpeg version string and the sha256 above.

Cargo form (WORKSPACE_RULES): `cd "<wt-studio-assets>/handshake-studio-assets" && source "<kernel>/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-reel --tests`, then clippy, then focused `cargo test -p hsk-studio-reel --no-fail-fast` (49 default tests today; also re-run with `--features av1` because `video.rs` is touched).

</topic>

<topic id="determinism" status="active" summary="Determinism contract">

## Determinism contract
Container indexing is integer arithmetic plus accord `Timescale` conversion with `Rounding::NearestHalfUp` (`src/time.rs:8-12`); `ticks_exact` records any rounding. No floating point is added. Same bytes + same limits -> same `MediaIndex` and same receipts, on every host. The new errors depend only on `Ticks`/counts. No tolerance rule applies in this lane.

</topic>

<topic id="deps" status="active" summary="No new dependencies">

## Allowed dependencies
None new. Reel keeps `hsk-studio-accord`, `hsk-studio-observe`, `sha2 =0.10.9` (force-soft; licence `MIT OR Apache-2.0`), `miniz_oxide =0.9.1` (`MIT OR Zlib OR Apache-2.0`), optional `rav1d =1.1.0` (`BSD-2-Clause`) and `libc =0.2.190` (`MIT OR Apache-2.0`) behind `av1` (licences read from the local cargo registry copies, read-only). No `Cargo.toml` change. LICENCE RULE: ffmpeg/ffprobe (LGPL/GPL; the installed build is `--enable-gpl --enable-version3`; https://ffmpeg.org/legal.html opened) are used only as external black-box generators/oracles. Do not read, copy or translate ffmpeg source (libavformat `mov.c`/`movenc.c` included); implement from the Apple pages above and the existing code.

</topic>

<topic id="redteam" status="active" summary="Risks, failure scenarios, minimum controls">

## Red team (controls are enforced by the acceptance criteria)
| Risk / plausible failure | Minimum control |
|---|---|
| R1 the guard runs after the sink already saw data (side effects on a rejected export) | guard call precedes the read loop in every provider; test 1 uses the `NeverReached` sink, so any early delivery panics the test |
| R2 only isobmff is fixed, another provider keeps returning `Approximate` under `Export` | test 1 enumerates `descriptors()` and fails on a decode-capable provider without a case |
| R3 `Incomplete` leaks into `Preview` and breaks scrubbing | test 2 asserts the `Preview` result is unchanged (`Ok` + `Partial`) |
| R4 treating zero-duration as open-ended on progressive files hides a truncated edit list | rule restricted to `fragmented && last entry`; test 3 pins the progressive `Unsupported` |
| R5 raw `fiel`/`clap` get interpreted downstream with the historically reversed 9/14 meaning | raw fields only, doc comment cites TN2162 and the dispute; any field-order claim is `NOT_PROVEN` (V4-07: no "always/never" claim without a test) |
| R6 hostile `fiel`/`clap` boxes break otherwise indexable tracks | test 4 truncated `clap`; advisory-metadata rule kept |
| R7 oracle drift when ffmpeg is upgraded | fixture committed with sha256 + provenance; no test runs ffmpeg (V4-03) |
| R8 export refuses legitimate 100 ns sources | decision 4: rounding is a receipt flag, not an error; existing test stays green |
| R9 concurrent edits of `isobmff.rs`/`video.rs` by SMB-REEL-AV1-ROUTE or ENG-2 | sequencing line in the frontmatter; pathspec commits only (V4 git rules) |

</topic>

<topic id="acceptance" status="active" summary="Acceptance criteria">

## Acceptance
1. Tests 1-5 green; `cargo test -p hsk-studio-reel` default and `--features av1` green; no existing test edited except to add fields to builders.
2. Each of the five delivery sites calls `require_exact_start` before any read (reviewable by grep: `require_exact_start` appears 5 times in `src/`, plus the `video.rs` Incomplete branch).
3. `report()` and `DESCRIPTOR` list the two new codes; `not_implemented` in `DESCRIPTOR` no longer claims anything this lane implemented; no "never/always/guarantees" wording without a test name (V4-07).
4. `#![deny(clippy::arithmetic_side_effects)]` still on every touched module; no `unsafe`; no new dependency.
5. Fixture + `provenance.json` + generator lines committed under `tests/fixtures/mov/`; READY_FOR_VALIDATION only; the validator issues the verdict.

</topic>

<topic id="unverified" status="active" summary="UNVERIFIED and Operator items">

## UNVERIFIED
- ISO/IEC 14496-12 text (zero `segment_duration` rule is a W3C-quoted secondary source for the 2012 edition); free availability of the standard.
- `fiel` detail 9/14 direction in any real-world file (TN2162 read, Apple's older text disputed); that is why the fields are raw.
- `mdcv`/`clli` box layouts (not opened); `cslg` semantics beyond the Apple page (not needed).
- How Adobe/Apple tools treat a zero-duration edit (no black-box probe was run).
- No caller in the workspace sets `Purpose::Export` yet except tests (future export queue STU-VID-060..064), so the guard is proven only by the tests in this lane.

## Operator items
None new for this lane. (Related, already open: D5 oracle session; STU-VID-051 clause 1 wording is addressed in lane-reel-eng-2.)

</topic>
