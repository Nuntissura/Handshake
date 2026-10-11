---
file_id: BUILDER-BRIEF-HSK-STUDIO-PULSE
file_kind: builder_brief
updated_at: 2026-10-11T01:10:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-pulse
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-PULSE` / key `pulse` / crate `hsk-studio-pulse`; foundation MT `MT-35540` (read its `proof.checks`, `execution.inputs`, `closure_unit`); lane = PULSE only, write scope `handshake-studio-assets/crates/hsk-studio-pulse/**`.
- Responsibility (verbatim): "Canonical integer-tick sequence time at 254016000000 ticks/second per STU-VID-012 and normalized broadcast ticks/frame per STU-VID-013. Preserve declared field types with checked arithmetic/range; rational time is for interchange adapters, never a competing stored authority. Own edit/trim/ripple/multicam references and scheduling; media decoder behind typed port."
- Foundation capability: "Execute tick-based trim/ripple of real sequence references with checked boundary/timebase behavior."
- Allowed deps (STUDIO_MODULES depends_on == root Cargo.toml `[workspace.metadata.studio.dependencies.hsk-studio-pulse].allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-observe`. Slice 1 needs only accord + observe (folio unused, see non-goals); declared-but-unused is fine.
- Forbidden: handshake_core/handshake_native/SurrealDB (ENG-DAG-FORBID-001); egui/wgpu/codec FFI/image decoder/runtime DB in pulse or its deps; any edge not in `allowed`; no `.GOV` reads (CX-211). Media decoder only behind a typed port (no reel dependency; reel is not in `allowed`).
- Crate conventions copied from existing 8 crates (verified): `[package]` edition 2024 rust-version 1.97.1; path dep `hsk-studio-accord = { path = "../hsk-studio-accord", version = "=0.1.0", default-features = false }`; `[lib] path="src/lib.rs"`; `[[example]] name="pulse-consumer"`; `pub const DESCRIPTOR: &str` JSON (owner, operation, model.api, limits, cancel, recovery, argus) like accord/observe; tests in `tests/foundation_contract.rs` using a `bounded_case` 30 s deadline wrapper. Root Cargo.toml (members, registry) is steward-owned; dep needs go to `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (file `.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`, sub-section 14.25 lines ~22700-25500)
- STU-VID-012 (l.22788): time = integer ticks, constant 254016000000/s; float seconds never stored. STU-VID-013 (l.22797): normative ticks/frame table (== `accord::BROADCAST_TICKS_PER_FRAME`); 013a: 10594594594 and 8475675675 are legacy import shapes normalised to 10594584000 / 8475667200.
- STU-VID-012c: drop-frame is display only, never changes stored ticks/frame counts.
- STU-VID-020 (l.23405): `StudioClip` `hsk.studio.clip@1`: `clip_id` `SCLP-{uuid_v7}`, `source_ref` (content ref, never a path), `source_in/out` (SOURCE ticks), `timeline_start`, `duration` = `(source_out-source_in)/speed` stored redundantly AND validated, `speed` rational, `reversed`, `link_group`. 020a: trimming changes only source window; source never modified.
- STU-VID-021 (l.23397): `StudioTrack` `hsk.studio.track@1`: `track_id` `STRK-{uuid_v7}`, kind video|audio|caption|submix, `locked`, `sync_locked`; clips on one track MUST NOT overlap.
- STU-VID-026 (l.23476): trim vocabulary + illegal-result rule: trim in/out refuse past source 0/end or past own tail; ripple trim shifts later content on sync-locked tracks, refuses when a sync-locked track cannot absorb; roll/slip/slide refuse when neighbour/source exhausted. STU-VID-027 razor: second clip `source_in` continues first `source_out`. STU-VID-028 lift vs extract. STU-VID-029 link/group/sync-lock are three separate relations. STU-VID-031 speed rational, reverse = negative sign.
- STU-VID-040: each op one reversible history step; model mutations go sandbox -> validation -> PromotionGate (STU-ARC-005), no fast path (pulse only PROPOSES; never commits). STU-ARC-011: proposals carry read/write footprint + expected revision. STU-ARC-007/009/010/018/022: foundation rules (real consumer + failure path, no hollow stubs).
- Rest (later scope): STU-VID-010/011 sequence record + settings, 016 presets, 017 editing modes (VID-080 gap), 022-025 monitors/3-point edit, 030 nesting, 032 transitions, 033 multicam, 034-035 markers/snapping, 038 proxies, 050-053 import, 060-064 export; gaps VID-080..086. MT-35540 fixture is the acceptance oracle.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, ~600-900 LOC in `src/lib.rs` or `src/{time,model,edit}.rs`)
Reuse (verified by grep): `hsk_studio_accord::{Ticks (new/value/checked_add/checked_frames/from_seconds), FrameRate (from_import/ticks_per_frame), FrameRateNormalization{source_ticks_per_frame,canonical_ticks_per_frame,changed}, TICKS_PER_SECOND, BROADCAST_TICKS_PER_FRAME, DomainId (parse; prefixes STRK/SCLP are valid), ActorContext, CancellationToken (check()), ValidationError (code())}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, Budget, SinkPort}`. Accord has no `checked_sub`: add local helper. Folio has NO sequence/timeline type (grep: no sequence/clip) -> do not invent a Folio mapping.
1. Timebase: `pub fn timebase_from_import(ticks_per_frame: u64) -> Result<(FrameRate, FrameRateNormalization), PulseError>` (thin over accord) + `frames_to_ticks(frames: u64, FrameRate) -> Result<Ticks>` and `ticks_to_frames_exact(Ticks, FrameRate) -> Result<u64>` (remainder != 0 -> `NotFrameAligned`).
2. Immutable snapshot + validation: `pub struct Speed{num:i64,den:u64}` (num != 0, reverse = negative num), `pub struct Clip{clip_id:DomainId, source_ref:String, source_in:Ticks, source_out:Ticks, timeline_start:Ticks, duration:Ticks, speed:Speed, link_group:Option<String>}`, `pub struct Track{track_id:DomainId, kind:TrackKind, locked:bool, sync_locked:bool, clips:Vec<Clip>}`, `pub struct Sequence{revision:u64, frame_rate:FrameRate, tracks:Vec<Track>}`; `Sequence::new(revision, frame_rate, tracks, &CancellationToken) -> Result<Sequence, PulseError>` checks: STRK/SCLP prefix via `DomainId::parse`, source_in<source_out, duration == `(source_out-source_in)*den/|num|` exactly (u128 intermediate, inexact -> `DurationMismatch`), sorted non-overlap per track, checked end overflow. Fields private + getters (snapshot immutable).
3. Trim/ripple planner (core): `pub fn plan_ripple_trim_out(seq:&Sequence, expected_revision:u64, clip:&DomainId, new_duration:Ticks, cancel:&CancellationToken) -> Result<EditProposal, PulseError>`; also `plan_trim_out` (gap opens, nothing shifts) and `plan_razor(seq, expected_revision, clip, at_timeline:Ticks, new_right_id:DomainId, cancel)`. `EditProposal{base_revision:u64, target_revision:u64 (checked +1), changes:Vec<ClipChange{clip_id, before:ClipWindow, after:ClipWindow}>, read_footprint:Vec<DomainId>, write_footprint:Vec<DomainId>}` with `ClipWindow{source_in,source_out,timeline_start,duration}`. Refusals: `TrackLocked`, `SyncLockBlocked` (a later clip on a sync-locked track whose `timeline_start < delta` or whose track is locked), `PastSourceEnd`/`PastHead`, `StaleRevision`, `Overflow`, `Canceled`; linked clips (`link_group`) trim together or refuse. Never mutates `seq`.
4. Apply: `Sequence::apply(&self, &EditProposal, cancel) -> Result<Sequence, PulseError>` returns a NEW snapshot at `target_revision` (re-validates; rejects `base_revision != self.revision`). This is the local verification step only; real commit authority stays with host (STU-ARC-011).
5. Observability: `pub const DESCRIPTOR: &str` (JSON like accord's) + `pub fn report(result:&Result<EditProposal,PulseError>, cancel, obs:&mut Observe, sink:&mut impl SinkPort)` mapping Ok -> `Outcome::Success`, `Canceled` -> `Outcome::Canceled`, validation -> `FailureCode::Validation`, overflow/unsupported -> `FailureCode::Unsupported`; no private text (`private_project_text: None`). Example `pulse-consumer` (`--descriptor`, `--rate TICKS --a-frames N --b-start N --trim-to N [--cancel]`) prints proposal JSON by hand (no serde needed).
Expected oracle (MT-35540, independent arithmetic): 25 fps = 10_160_640_000 ticks/frame; clipA 0..3 frames, clipB starts at 3 frames; ripple-trim A to 2 frames => A.duration == 2*10_160_640_000 == 20_321_280_000, A.source_out reduced by the same ticks (speed 1), B.timeline_start == 20_321_280_000, ids and source windows of B unchanged; original `Sequence` still equals its pre-call clone.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- No new crates.io dependency needed (std + 3 workspace crates). Do NOT add serde/num-rational in slice 1. If JSON snapshot IO is later required, reuse the already steward-pinned `serde 1.0.229` / `serde_json 1.0.151` (registry metadata of folio/chronicle, pure-rust) via dep-request rather than a new crate.
- Build/test: `cd "<ws root>" && cargo check -p hsk-studio-pulse --tests`, `cargo clippy -p hsk-studio-pulse --tests`, `cargo test -p hsk-studio-pulse` (focused, shared target per START_LOG cargo-tooling).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/foundation_contract.rs">

## Tests (<=5, each wrapped in `bounded_case`)
1. `ripple_trim_out_25fps_fixture`: MT-35540 fixture above; assert exact tick values, B untouched source window, original snapshot unchanged, `target_revision == base+1`, apply() yields the same numbers.
2. `legacy_rates_normalise_with_receipt`: 10594594594 -> 10594584000, 8475675675 -> 8475667200 (`changed == true`), canonical 10160640000 unchanged, unknown rate (e.g. 7) -> error; `frames_to_ticks(u64::MAX,..)` -> `Overflow`.
3. `invalid_sequence_inputs`: zero speed, duration mismatch, overlapping clips, wrong id prefix (SLYR for clip), source_in >= source_out each rejected with distinct code.
4. `refusals_and_stale`: locked track, sync-lock cannot absorb, trim past source end, `expected_revision` != current -> `StaleRevision`, near-`u64::MAX` timeline_start shift -> `Overflow`.
5. `cancel_and_observe`: pre-canceled token -> `Canceled` and no proposal; `report()` into a `Vec`-backed `SinkPort` emits exactly one terminal frame with `Outcome::Canceled` / `Success` / `Failure(Validation)`.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No media decode/encode, proxies (VID-038), transitions (032), multicam (033), nesting (030), retime/optical flow (031 algorithms), markers/snapping (034/035), presets/editing modes (016/017, gap VID-080), export (060-064), drop-frame/timecode display (012a/012c), GUI, history stack, EventLedger/CRDT, Folio `StudioDocument` embedding (gap: STU-VID-010 says sequences are `StudioDocument` members but `hsk-studio-folio` has no sequence type; needs a Folio-owner decision before any mapping), Argus/Controls wiring, rational interchange adapters (belong to interop-media).
- Later MTs: video feature MTs approx MT-33685..MT-33847 (STU-VID-010..105; UNVERIFIED exact range, select narrowly from `_MT_INDEX.json`); host adoption MT-36200. Domain MT plan files `_mt_plan_*.json` contain NO STU-VID content (grep: 0 hits) - the only plan-level source is MT-35540.
- Report completion as READY_FOR_VALIDATION only; never self-PASS (CX-PROOF-002).

</topic>
