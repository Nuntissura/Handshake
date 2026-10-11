---
file_id: BUILDER-BRIEF-HSK-STUDIO-INTEROP-MEDIA
file_kind: builder_brief
updated_at: 2026-10-11T03:10:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-media
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-INTEROP_MEDIA` / key `interop_media` / crate `hsk-studio-interop-media`; foundation MT `MT-35552` (read `implementation_steps` 1-4 and `proof.checks`). Write scope `crates/hsk-studio-interop-media/**` only.
- Responsibility (verbatim): "Typed timeline/motion/audio interchange (PRPROJ/AEP/SESX included explicit missing-decoder scope), loss-aware proposals through neutral Folio contracts. No Lightroom parser or private catalog; lrcat has its own adapter."
- Foundation capability: "Execute actual timeline/develop/audio interchange mapping at exact time/resource identity with losses; PRPROJ/AEP/SESX gaps explicit."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-observe`. NOT allowed: pulse, score, motion, reel (so output is a neutral proposal type owned here).
- CONTRADICTION to resolve (lead): responsibility says proposals go "through neutral Folio contracts", but `hsk-studio-folio` has no timeline/sequence/clip/audio type (grep: only layers, artboards, page spreads, layer graph) and STU-VID-010 says sequences are `StudioDocument` members. Proposed rule for slice 1: define a small neutral `TimelineProposal` here (ticks, ids, dispositions, losses); a later decision either moves it into folio or has pulse/host adapt it. Do not invent a Folio schema.
- Forbidden: handshake_core/native/SurrealDB; Lightroom/lrcat parsing (interop-lrcat); SQLite; any Adobe SDK/decompiled-code derived logic (black-box + public specs only); `image` crate and `-sys` crates (probe policy); edges outside `allowed`.
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[[example]] name="interop-media-consumer"`, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`.
- Prep refs (reuse): `docs_repo/studio-collaboration-2026-10-09/modular-authoring/media-team/preparation.json` -> `module_proposals.interop_media` (steps: bound container/index/entry/inflation; per-entity dispositions; quarantine opaque original bytes; two-roundtrip mapping; oracle = versioned original-authored PRPROJ/AEP/SESX plus supported interchange fixtures) and `observed_source_findings[1]` (FilmCraft `Format` enum is EDL/FCP7XML/FCPXML/OTIO/AAF/OMF - NOT PRPROJ proof). Reference source (MIT OR Apache-2.0, read-only case study): `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/filmcraft-7b6c134472287db4367fca80bcb498240ba36567/crates/interchange/src/{edl,otio,fcp7,common,comp}.rs` (14k LOC crate, deps roxmltree/cfb/serde_json). `repass-2026-10-09/filmcraft-root-drill.json` FILM-ROOT-10 (interchange returns warning reports; PRPROJ import remains an OPEN_GAP).

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-IO-186 (l.211): native editable PRPROJ/AEP (and INDD/FIG) remain REQUESTED scope; PRPROJ/AEP are NOT satisfied by rendered media or alternate interchange; every format must report supported version, per-object/property editable mapping, preserved opaque data, unsupported/loss decisions, provenance and round-trip disposition; missing decoders/fixtures are blocking scope gaps, never silent exclusions; parser success, self-render or original-byte preservation is not editable-parity proof.
- STU-IO-189 (l.235): `.sesx` editable session transport needs versioned per-entity mapping/loss outcomes; retained/translated/approximate/unsupported/missing explicit; "successful parser/count is not full conversion". STU-IO-187 (l.231): container safety (bounded streaming, inflation/entry/duplicate/path/cycle checks). STU-IO-005 (l.21304): vendor-private structures preserved as opaque blob with receipt of preserved-opaque vs imported; STU-IO-012 (l.21418): import fails closed, never a silent partial; STU-IO-006 matrix rows 48-50 (l.~21360): `.prproj`, `.aep/.aepx`, `.sesx` = import ("I") with decoder NOT_PROVEN, XML/render fallback never satisfies native parity.
- STU-VID-012/013/013a (l.22788-22815): ticks 254016000000/s, broadcast table, legacy 10594594594/8475675675 normalization (use `accord::FrameRate::from_import`); STU-VID-012c: drop-frame is display/timecode-parse only, never changes stored ticks. STU-ARC-021: exact requested vs actual counts; ARC-015: typed loss for every error.
- Later ranges: STU-IO-001..014 (native format, profile-explicit import/export, fixtures), STU-IO-190 reopen/ID rewrite, STU-VID-060-064 export recipes, MOT-12x/CMP interop. MT corpus: media interop feature MTs in the STU-IO block (UNVERIFIED range; `_mt_plan_1213_raw_interop.json` has 14 STU-IO refs but raster/RAW).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only; ~800-1100 LOC)
Chosen "genuinely supported interchange mapping" for slice 1 (public, deterministic, no proprietary decoder): **CMX3600 EDL** (text). Reuse (verified): `hsk_studio_accord::{Ticks (checked_frames, from_seconds, value), FrameRate (from_import -> (FrameRate, FrameRateNormalization)), TICKS_PER_SECOND, BROADCAST_TICKS_PER_FRAME, CancellationToken, DomainId, ValidationError}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Folio is allowed but unused in slice 1 (see contradiction).
1. Time adapters (the place where rational/timecode time is converted; stored authority stays integer ticks): `pub fn timecode_to_frames(tc:&str, fps:FrameRate, drop:bool) -> Result<u64, InteropError>` (HH:MM:SS:FF / `;` drop-frame; DF only for 29.97/59.94 rates; dropped frame numbers (e.g. `00:01:00;00`/`;01` for 29.97 DF) rejected), `pub fn frames_to_ticks(frames:u64, fps:FrameRate) -> Result<Ticks>` (via `Ticks::checked_frames`), `pub fn rational_to_ticks(value:i64, rate_num:u32, rate_den:u32) -> Result<(Ticks, Exactness), InteropError>` where `Exactness::{Exact, NormalizedLegacy, Approximate(rounding)}`: exact when `TICKS_PER_SECOND*den % rate_num == 0`, legacy table values use `FrameRate::from_import` receipt, anything else is `Approximate` with the rounding rule NAMED (half-away-from-zero) and recorded as a loss note (never silent).
2. Neutral proposal types (no authority): `pub enum Disposition{Editable,Opaque,Translated,Approximate,Unsupported,Missing}` (STU-IO-186 vocabulary), `pub struct LossNote{entity:String, property:&'static str, disposition:Disposition, code:&'static str}`, `pub struct ClipProposal{event:u32, reel:String, clip_name:Option<String>, track:TrackKind, source_in:Ticks, source_out:Ticks, record_in:Ticks, record_out:Ticks, speed:Option<(i64,u64)>, disposition:Disposition}`, `pub struct TimelineProposal{source_format:SourceFormat, frame_rate:FrameRate, normalization:Option<FrameRateNormalization>, clips:Vec<ClipProposal>, losses:Vec<LossNote>, source_len:u64, source_sha256:Option<[u8;32]>}`. Output is a proposal ONLY (`apply`/commit lives in session/host).
3. EDL importer: `pub fn import_cmx3600(bytes:&[u8], req:&ImportRequest{frame_rate:FrameRate, assume_drop:bool}, limits:&Limits{max_bytes, max_events, max_line_len}, cancel:&CancellationToken) -> Result<TimelineProposal, InteropError>`: UTF-8 validate, parse `TITLE:`, `FCM: DROP FRAME|NON-DROP FRAME`, events `NNN reel track trans [dur] src_in src_out rec_in rec_out`, `* FROM CLIP NAME:` comments, `M2 reel speed_fps frame` (speed => `Translated` + exact rational), dissolves/wipes (`D`,`W`) => clip kept `Editable` but transition recorded as `Unsupported` loss (cuts-only mapping), audio tracks `A/A2/AA/B/NONE` mapped with `Approximate` channel loss. Validation (all typed, none silent): duplicate/non-monotonic event numbers, `src_out <= src_in`, record range overlap on the same track, record_in gaps allowed, timecode > limits, tick overflow, dangling reel reference (a `* FROM CLIP NAME`/reel missing from an optional caller-supplied `known_reels` list => `Missing` loss), cancel polled per event. Input bytes are never modified; failure returns no partial proposal.
4. Native-format gap stubs that are REAL container checks (STU-IO-186: explicit missing-decoder scope, not silent exclusion): `pub fn sniff_native(bytes:&[u8]) -> NativeSniff{format:Prproj|AepBinary|Aepx|Sesx|Unknown, evidence:&'static str, len:u64, sha256?:..}` using magic bytes only (gzip `1F 8B` => Premiere project candidate [UNVERIFIED which magic Adobe uses; confirm black-box by saving a project], RIFX => AEP binary candidate, `<?xml` + root element name => aepx/sesx candidate) and `pub fn import_native(..) -> Err(InteropError::NativeDecoderMissing{format, detail})` returning the opaque original bytes digest + a `Missing` loss entry for EVERY entity class, so a caller can preserve the original as an opaque blob (STU-IO-005) while editable import stays BLOCKED (never "supported").
5. Observability: `DESCRIPTOR` JSON (formats: supported = cmx3600; blocked = prproj/aep/sesx) + `report(result, cancel, &mut Observe, &mut impl SinkPort)` (Ok -> Success; `NativeDecoderMissing`/unsupported -> `Failure(Unsupported)`; lossy Ok still Success but receipt lists loss count; cancel -> Canceled); example `interop-media-consumer` (`--descriptor`, `--edl FILE --rate 25000/1000 [--drop]`, `--sniff FILE`).
Stretch (only if time remains, needs dep-request): OTIO JSON subset (`Timeline -> Stack -> Track -> Clip`, `RationalTime{value,rate}`, `TimeRange`) via the already-pinned `serde_json`; float rates like 23.976023976023978 normalize to 10594584000 ticks/frame with a `NormalizedLegacy`-style receipt.
Oracle (independent): hand-computed ticks: 25 fps `01:00:00:00` = 90000 frames = `90000*10_160_640_000`; 29.97 DF `01:00:00;00` = 107892 frames, `00:10:00;00` = 17982 frames (published drop-frame arithmetic); 24 fps `value=48` => exactly 2 s = `508_032_000_000` ticks; plus an EDL authored by hand.
Oracle (black-box vs Adobe, later; public specs + black-box observation only, NO decompilation): export CMX3600 EDL and FCP7 XML from the installed Premiere Pro for a known controlled sequence (cuts, one dissolve, one speed change, mixed audio) and compare Studio's clip/edit points (ticks and timecode display), speed and dissolve handling against Premiere's own timeline readout; for `.prproj`/`.aep`/`.sesx` derive field mappings ONLY by saving controlled one-property-different projects from the installed apps and diffing the saved files, then keep editable import BLOCKED until an independent round-trip passes.

</topic>

<topic id="deps" status="active" summary="third-party deps (crates.io API 2026-10-11)">

## Deps
- Slice 1: none (std only). Optional: `sha2 0.10.9` (`default-features=false`, `force-soft`, pure-rust, already workspace-locked) for `source_sha256`; request via dep-request; steward must add `hsk-studio-interop-media` to the probe `approved` list.
- Stretch/later (each via dep-request, pure-rust yes by dependency list): `serde 1.0.229`/`serde_json 1.0.151` (workspace-pinned); `roxmltree 0.21.1` (MIT OR Apache-2.0, dep memchr only, updated 2025-10; FCP7 `xmeml`/FCPXML/sesx/aepx read-only DOM; FilmCraft interchange uses it); inflate for `.prproj` (gzip): `miniz_oxide 0.9.1` (MIT OR Zlib OR Apache-2.0, dep adler2) or `flate2 1.1.10` with `rust_backend` only (default features pull miniz_oxide; avoid `libz-sys`/`zlib-ng` features) - needs bounded streaming inflate (STU-IO-187). `cfb` (AAF/OMF container) exists as zero-dep FilmCraft leaf (995 LOC) - adapt with `SOURCE_PROVENANCE.json` only after native scope decision.
- Rejected: `quick-xml`/`xml-rs` (no need beyond roxmltree), any Adobe-SDK-derived parsing.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-media --tests` (then clippy, focused test).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`; hand-authored EDL text inline, nothing written outside TEMP)
1. `edl_three_event_fixture_ticks`: 25 fps EDL with a cut, a dissolve (`D 025`), and an `M2` 50 fps speed event + `* FROM CLIP NAME`; assert exact `record_in/out` ticks (`frames * 10_160_640_000`), speed `Translated` as exact rational, dissolve recorded as `Unsupported` loss, input bytes unchanged.
2. `timecode_oracle_and_drop_frame`: `01:00:00;00`@29.97 DF = 107892 frames, `00:10:00;00` = 17982, `00:01:00;00` rejected (dropped number), DF on 25 fps rejected, `99:59:59:24` overflow/limit handling, non-DF 24 fps vectors.
3. `rational_time_exactness`: `(48, 24/1)` => `Exact` 508_032_000_000; `(1, 24000/1001)` => 10_594_584_000 `Exact`; legacy rate ticks 10594594594 => `NormalizedLegacy` with receipt; `rate_den=0`, huge value => error; non-divisor rate => `Approximate` with named rounding recorded as loss.
4. `edl_negative_paths`: duplicate event number, `src_out <= src_in`, overlapping record ranges on one track, invalid UTF-8, oversize/over-event limit, unknown reel vs `known_reels` => `Missing` loss, pre-canceled token => `Canceled` with no proposal.
5. `native_formats_blocked_loudly`: synthetic gzip-magic bytes / `RIFX` header / `<?xml ... <sesx` root => correct `sniff_native` format; `import_native` returns `NativeDecoderMissing` with original-bytes digest and `Missing` loss for every entity class (never `Ok` editable); truncated 3-byte input => `Unknown`.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No PRPROJ/AEP/SESX decoding (blocking scope gaps stay open, STU-IO-186), no FCP7/FCPXML/AAF/OMF/OTIO in slice 1, no media decode (reel), no audio file mapping beyond track/channel dispositions, no Lightroom (interop-lrcat), no write/export direction, no color management, no Folio schema invention, no GUI.
- Wave: 1 (accord + observe; folio unused). Pulse/host adaptation of `TimelineProposal` is a separate later step. Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
