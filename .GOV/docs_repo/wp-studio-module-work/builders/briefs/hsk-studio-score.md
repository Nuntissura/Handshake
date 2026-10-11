---
file_id: BUILDER-BRIEF-HSK-STUDIO-SCORE
file_kind: builder_brief
updated_at: 2026-10-11T01:40:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-score
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-SCORE` / key `score` / crate `hsk-studio-score`; foundation MT `MT-35541` (its `execution.implementation_steps` 1-6 are the acceptance script; read them). Write scope `handshake-studio-assets/crates/hsk-studio-score/**` only.
- Responsibility (verbatim): "Pure audio sample/channel/timing, DSP/spectral/mix/routing/latency semantics and bounded offline block processing. Device I/O and plugin hosting are optional separate providers."
- Foundation capability: "Process preallocated known audio block with channels/sample positions/latency receipt; offline/native host separate."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-observe` (NO folio, NO pulse). Hence audio is addressed by integer sample positions + `accord::Ticks`, never by pulse types.
- Forbidden: handshake_core/native/SurrealDB; cpal/ASIO/plugin hosts/decoders (belong to score-device/score-plugin/reel); egui/wgpu/codec FFI/DB in score or its deps; any dependency not in `allowed`.
- Conventions (existing crates, verified): edition 2024, rust-version 1.97.1, path dep `hsk-studio-accord = { path = "../hsk-studio-accord", version = "=0.1.0", default-features = false }`, `[lib] path = "src/lib.rs"`, `[[example]] name = "score-consumer"`, `pub const DESCRIPTOR: &str` JSON (like accord/observe), tests in `tests/foundation_contract.rs` with a `bounded_case` 30 s deadline wrapper. Root `Cargo.toml` is steward-owned; dep needs go to `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-FX-137 (l.18454) audio routing is document data: master config mono|stereo|5.1|multichannel_adaptive|sixteen_channel; channel layouts carry ORDERED label lists (Mono=1; Stereo=2 labels 100,101; 5.1=6); track `volume` default 1.0 linear gain, `pan` default 0, mute/solo, ordered sends to submix; sample rate per sequence (48000 default, 32000 also shipped).
- STU-FX-134/135/136 (l.18436-18450): audio effects are non-destructive stack entries, params are `normalised_scalar` with an affine mapping (`value = n*(max-min)+min`); STU-FX-136a (l.18496): per-processor contract tables; **Hard Limiter** (l.18696): `max_amp` -100..0 (default -50), `input_boost` -100..50 (20), `lookahead_time` 5..20 (7.1), `release_time` 40..200 (100); units are NOT declared by the sheets (136b) - author units deliberately (dB, dB, ms, ms) and record them as authored.
- STU-FX-134a (l.18795): 138 audio processors catalogued (e.g. `Amplify` 4 params, no contract table) - later scope.
- STU-VID-012 (l.22788): 254016000000 ticks/s; 48000 Hz = 5_292_000 ticks/sample, 32000 Hz = 7_938_000, 44100 Hz = 5_760_000 (exact; compute with `TICKS_PER_SECOND / rate`, reject non-divisors). STU-VID-011: `audio_sample_rate_hz` u32, `audio_time_display` audio_samples|milliseconds.
- STU-ARC-013 (l.221): audio callbacks use preallocated bounded handoff, no allocate/lock/decode/I-O; underrun has safe output. STU-ARC-021 (l.251): bounce/export receipts report EXACT requested vs actual sample counts, no silent clamp/truncation. STU-ARC-008/012: bounded admission, release on every exit. STU-ARC-007/009/010/018/022: foundation rules (real consumer + failure path; no hollow stubs). Gap STU-FX-147 (loudness standards) - do not invent.
- Later ranges: STU-FX-134a processors (138), 138 guided adjustment, STU-VID-029a sync, 037/051 audio import (reel). Feature MTs: `_mt_plan_*` files contain no audio MTs (grep); STU-FX-1xx derived MTs live in the MT-3xxxx corpus (select narrowly).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, ~700-1000 LOC; planar `f32`)
Reuse (verified by grep): `hsk_studio_accord::{Ticks (new/value/checked_add), TICKS_PER_SECOND, CancellationToken (cancel/check), ActorContext, DomainId, ValidationError}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`.
1. Types: `pub struct ChannelLayout(Vec<u16>)` (`stereo() = [100,101]`, `mono()`, `new(labels)` rejects empty/duplicate), `pub struct BlockSpec{sample_rate_hz:u32, layout:ChannelLayout, frames:usize, start_sample:u64}`, `pub fn ticks_per_sample(rate_hz:u32)->Result<u64,ScoreError>` and `pub fn sample_to_ticks(sample:u64, rate_hz:u32)->Result<Ticks,ScoreError>` (checked, exact divisors only). `ScoreError` enum with `code()` (InvalidRate, ChannelMismatch, CapacityExceeded, PositionOverflow, Busy, Canceled, StaleRevision, MissingContext, NotFinite...).
2. Admission + preallocated output (MT-35541 core): `pub struct OutputBlock{...}` created once with `OutputBlock::with_capacity(layout, capacity_frames)`; `pub struct Engine` owning one slot; `Engine::process(&mut self, ctx:&RequestContext, input:&InputBlock<'_>, proc:&mut dyn Processor, out:&mut OutputBlock, cancel:&CancellationToken) -> Result<BlockReceipt, ScoreError>`. `RequestContext{ resource:DomainId, actor:Option<ActorContext>, expected_revision:u64, current_revision:u64 }` - `None` actor => `MissingContext` BEFORE touching PCM; revision mismatch => `StaleRevision`. Validate ALL before any write: rate != 0, every channel slice length == frames, `frames <= out.capacity()` (255-cap rejects 256 frames, no writes), `start_sample.checked_add(frames)`, finite check optional. Single reservation: second concurrent `process` => `Busy` (RAII guard over `AtomicBool`, no spin). Cancel before admission publishes nothing; after begin, finish the whole block or discard (output `published_frames` stays 0 until final commit). `BlockReceipt{processed_frames:usize, latency_samples:u32, start_sample:u64, end_sample:u64, labels:Vec<u16>, revision:u64, peak:Vec<f32>}`.
3. Processors behind `pub trait Processor{ fn latency_samples(&self)->u32; fn process(&mut self, input:&InputBlock<'_>, out:&mut OutputBlock); }`: `Unity` (bit-exact copy, latency 0), `Gain{linear:f32}` + `Pan{-1.0..=1.0 equal-power}` + `ChannelMix` (stereo->mono, ordered `Send` to a submix with its own level per FX-137). No allocation inside `process`.
4. Spec-grounded DSP: `HardLimiter::new(sample_rate_hz, max_amp_db, input_boost_db, lookahead_ms, release_ms)` validating the FX-136a hard ranges (reject outside), `latency_samples = round(lookahead_ms*rate/1000)`, ring-buffered lookahead peak limiter (no output sample exceeds `10^(max_amp/20)`); plus `normalised_to_value(n, min, max) = n*(max-min)+min` helper. Receipt reports the limiter latency (non-zero), so the latency path is real, not constant 0.
5. Observability: `DESCRIPTOR` JSON + `pub fn report(result:&Result<BlockReceipt,ScoreError>, cancel, obs:&mut Observe, sink:&mut impl SinkPort)` (Ok->Success, Canceled->Canceled, validation->`FailureCode::Validation`, Busy/Capacity->`Unavailable`); example `score-consumer` (`--descriptor`, `--frames N --capacity N --rate HZ --limiter [--cancel]`) synthesizes the fixture and prints receipt + peak/RMS; no foreground windows, no device.
Oracle fixture (MT-35541): 256-frame stereo @48000 Hz, labels [100,101], start_sample 0, L repeating [0,0.25,-0.25,0.5], R = -L, unity gain, capacity 256, explicit resource/actor/revision, no random seed. Expected: output equals input bit-for-bit, `processed_frames == 256`, `latency_samples == 0`, labels preserved, start_sample preserved.

</topic>

<topic id="deps" status="active" summary="third-party deps (verified on crates.io API 2026-10-11)">

## Deps
- Slice 1: NO new dependency (std only). Optional dev-only for the example (to dump a WAV for external inspection): `hound 3.5.1` - Apache-2.0, zero deps, pure-rust yes, last release 2023-09 (stable, ~5.9M recent dl); request via dep-request as `[dev-dependencies]`, else write ~40 lines RIFF PCM16 by hand.
- Later (NOT now): `rubato 5.0.1` (resample; MIT OR Apache-2.0; updated 2026-10; deps audioadapter, num-traits, windowfunctions, visibility; pure-rust yes - no -sys deps), `rustfft 6.4.1` (MIT OR Apache-2.0; pure) + `realfft 3.5.0` (MIT) for spectral, `dasp_sample 0.11.0` (MIT/Apache; tiny, last release 2020 - stale), `fundsp 0.23.0` (MIT/Apache, pure; reference only - STU-ARC-018 forbids all-engine facades). Decoding (`symphonia 0.6.1`, **MPL-2.0**, pure Rust, updated 2026-08) belongs to reel/interop, not score. `cpal 0.18.2` (Apache-2.0) is OS-binding native: only in score-device.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-score --tests` (then clippy, `cargo test -p hsk-studio-score`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case` wrapper)
1. `unity_block_is_bit_exact`: the fixture above; assert samples, labels, start_sample, `processed_frames == 256`, `latency_samples == 0`; sample->ticks of start/end equals independent `n * 5_292_000`.
2. `admission_rejects_before_write`: capacity 255 vs 256 frames (output untouched, checked byte-for-byte), rate 0, channel slice length mismatch, `start_sample = u64::MAX - 10` + 256 frames -> overflow, missing actor -> `MissingContext`, stale revision -> `StaleRevision`.
3. `busy_and_cancel`: hold a reservation then call -> `Busy` immediately; pre-canceled token -> `Canceled` with `published_frames == 0`; mid-block cancel (cancel from the processor after N frames) leaves output unpublished.
4. `hard_limiter_spec_ranges`: input +6 dB over ceiling never exceeds `10^(max_amp/20)` (+1e-6), `latency_samples == round(7.1*48000/1000) = 341`, params outside FX-136a hard ranges rejected, `normalised_to_value(0.5,-100,0) == -50`.
5. `report_maps_outcomes`: `Vec`-backed `SinkPort`; Success/Canceled/Failure(Validation) frames emitted exactly once each.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No device I/O (score-device MT-35559), plugin hosting (score-plugin MT-35560), file decode/encode (reel/interop), resampling, FFT/spectral, EQ/dynamics beyond the one limiter, loudness (gap FX-147), automation/keyframes (motion), Folio mapping (score may not depend on folio), GUI/Controls, realtime threads, multichannel_adaptive routing graph, bounce/export receipts beyond the exact-count rule.
- Wave: 1 (accord + observe only). Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
