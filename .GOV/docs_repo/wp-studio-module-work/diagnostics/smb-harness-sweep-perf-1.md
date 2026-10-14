---
file_id: diagnostics-smb-harness-sweep-perf-1
file_kind: diagnostics
lane: SMB-HARNESS-SWEEP-PERF
crate: hsk-studio-harness
input: validation/round-R25.json (psd_tools_sweep_shard_3 TIMEOUT 120.054 s at d48bc30, 6 shards); lanes hsk-studio-convert-colour.json (3a496bb), hsk-studio-harness-ui.json (c4c6d4d 16 shards, shard_0 99 s, 240 s override)
measured_at: edit tree b316ba2 (= origin), test profile (no [profile] section in root Cargo.toml -> opt-level 0 for every crate, as in the round)
updated_at: 2026-10-14T21:27:37Z
---

<topic id="cause" status="current" summary="debug-profile cost of the full CMYK pixel path, opened for this file by 3a496bb">

Root cause: since 3a496bb the 16.8 Mpx CMYK file `cmyk-alpha-spot.psd` takes the full pixel path (260 tiles, Prism
4D-CLUT f64 conversion, ~1.2 GB of SHA-256) and the round's test profile compiles all of it at opt-level 0: 99.4 s
for that one file, 18x slower than with opt-level 3 on three crates (5.5 s).

- Before 3a496bb the harness passed no converter: CMYK without a converter is a typed unmapped layer, 0 tiles,
  0.55 s for this file (measured: `to_proposal(.., None, ..)` 300 ms, tiles 0).
- With the Prism converter: 260 tiles, 260 `convert_wide` calls, 16.83 Mpx converted.
- Old shard 3 (index % 6 == 3, 52 files, 11.47 MB) = this file (99.4 s) + the other 51 files (52.8 s serial) =
  ~152 s serial > 120 s cap -> TIMEOUT 120.054 s in R25. The other 6-shard shards held no such file (37-71 s).
- Not a per-pixel Prism call pattern (the converter is called once per 65536-pixel tile, chunked inside Prism),
  not an f64 regression, not a redundant conversion: the code path is the admitted chunked wide transform. The cost
  is unoptimized code: Prism per-pixel math, interop-psd's tile loop and SHA-256 (sha2 0.11 in prism, sha2 0.10.9
  force-soft in package) all at opt-level 0.

</topic>

<topic id="evidence" status="current" summary="stage timings, opt-level 0 vs opt-level 3 on sha2/prism/interop-psd">

Method: a temporary ignored diagnostic test (`crates/hsk-studio-harness/tests/sweep_profile_diag.rs`, NOT committed,
removed after the runs) on the 52 files of the R25 shard 3 (same file walk + sort as the sweep, index % 6 == 3).
Stages: `read_psd`, `to_proposal` with no converter and with interop-psd's `PrismConverter` wrapped in a timing
delegate (`convert_wide` time, calls, values), the harness `import` (proposal + `package_proposal`) and `to_package`.
Serial (`--no-capture`), `cargo nextest run -p hsk-studio-harness --features import-psd,import-raw --test
sweep_profile_diag --run-ignored only`. Logs: scratchpad `smb-harness-sweep-perf/diag1-baseline.log`,
`diag2.log`, `diag3-opt.log`, `proof.log`.

cmyk-alpha-spot.psd (5100x3300, 8-bit CMYK + alpha + spot, 6 channels, layerless, embedded 557168-byte `prtr` CMYK
profile), opt-level 0 (round profile today):

| stage | ms |
|---|---|
| to_proposal without converter (CMYK unmapped, 0 tiles; the pre-3a496bb behaviour) | 300 |
| to_proposal with Prism converter (260 tiles) | 80599 |
| of which convert_wide (260 calls, 16.83 Mpx, 67.32 M values) | 47735 |
| of which tile loop (decode, unit samples, f32 tiles, per-tile SHA-256) | ~32900 |
| harness import total (proposal + package_proposal: asset copies + SHA-256 per asset) | 96982 (93149 on rerun) |
| to_package | 2372 |
| whole file | 99425 |

Micro (same build): SHA-256 (sha2 0.11, prism `profile_hash`) 45.7 MB/s at opt-level 0; `convert_wide` per call
36.76 ms with 1 pixel (= three SHA-256 passes over the 557 KB profile per call: interop-psd `facts_of`, interop-psd
`convert_wide`, Prism `verify` on the plan-cache hit), 139 ms per 65536-pixel tile, 1.575 us per pixel.
SHA-256 volume per import of this file ~1.2 GB: profile 3 x 260 x 557 KB = 435 MB, Prism output digest 202 MB,
interop-psd tile digests 273 MB, harness/package asset digests 273 MB (package uses sha2 0.10.9 `force-soft`).

Rest of R25 shard 3 (51 files): 52.8 s serial (`full_ms` sum); 4 small CMYK files (200x200, same profile family)
2.2-2.8 s each, of which ~0.5 s conversion = 8 calls x ~37 ms profile hashing + pixels; gray1.psd (2.16 Mpx, 40
tiles) 9.0 s. Shard 3 total ~152 s serial against the 120 s cap: the R25 TIMEOUT.

Same file with `opt-level = 3` for `sha2`, `hsk-studio-prism`, `hsk-studio-interop-psd` only (everything else
opt-level 0, debug assertions and overflow checks unchanged):

| stage | opt-level 0 | opt-level 3 on 3 crates |
|---|---|---|
| whole file (import + to_package) | 99425 ms | 5499 ms (6524 ms on rerun) |
| to_proposal with converter | 80599 ms | 4111 ms |
| convert_wide total | 47735 ms | 2986 ms |
| SHA-256 throughput | 45.7 MB/s | 1959.6 MB/s |
| convert_wide per call (1 px) / per tile / per pixel | 36.76 ms / 139 ms / 1.575 us | 0.85 ms / 11 ms / 0.163 us |
| package sha256 | 784bcc75...61d8b1ef | 784bcc75...61d8b1ef (identical bytes, 29696840) |

distinct_from: not the shard balancing of c4c6d4d (that change moved this file into its own shard_0 at 99 s and
needed a 240 s override); not a Prism per-pixel API misuse (one chunked wide request per 65536-pixel tile).

</topic>

<topic id="fix" status="current" summary="steward: per-package opt-level 3 in the dev (=test) profile; lead: retire the shard_0 240 s override">

The cost is the unoptimized build of three hot crates, so the fix is a build-profile setting, not product code and
not a cap. It lives in the root `Cargo.toml` (workspace steward owned; outside this lane's paths), so this lane
does not apply it. Requested steward patch (append to the root `Cargo.toml`; the test profile inherits dev):

```toml
# Corpus-scale imports (SMB-HARNESS-SWEEP-PERF, diagnostics/smb-harness-sweep-perf-1.md): SHA-256, Prism's
# per-pixel ICC evaluation and interop-psd's tile loop run 18x faster optimized; results are byte-identical
# (no fast-math in Rust; debug-assertions and overflow-checks stay on).
[profile.dev.package.sha2]
opt-level = 3
[profile.dev.package.hsk-studio-prism]
opt-level = 3
[profile.dev.package.hsk-studio-interop-psd]
opt-level = 3
```

- `[profile.dev.package.sha2]` covers both sha2 0.10.9 (force-soft, package/tiff/...) and 0.11.0 (prism, develop).
- The round entrypoint builds with the root profile (no per-run flags; CX-VAL-007 satisfied: version-controlled
  config). The clap-gain fixture has its own `[workspace]`, so its byte-reproducible library is unaffected.
- Cost: one-time rebuild of prism and its dependents (measured 52 s for the harness closure in the edit tree);
  prism and interop-psd compile slower thereafter (not measured separately).
- After it lands, the lead can retire the `.config/nextest.toml` override for `psd_tools_sweep_shard_0`
  (240 s): shard_0 measures 6.0 s under the 120 s default cap.
- Rejected in-scope alternatives: re-sharding cannot split one 99 s file; dropping the converter or sampling the
  file in the sweep removes coverage; caching the profile hash in interop-psd `PrismConverter` (2 of the 3
  per-call hashes, ~6 s of 99 s at opt-level 0, ~0.2 s optimized) is too small to meet the target alone and is
  left as a note for the interop-psd owner.

</topic>

<topic id="proof" status="current" summary="digest equality, sweep shard times, prism/interop-psd tests under the override">

Edit tree b316ba2 (= origin/codex/handshake-studio-assets), studio-env.sh, the override passed as the same three
settings through `cargo nextest run --config 'profile.dev.package.<crate>.opt-level=3'` (diagnostic only; the round
must get it from the root Cargo.toml). Log: scratchpad `smb-harness-sweep-perf/proof.log` (21:22:16Z-21:27:08Z).

1. Byte identity: cmyk-alpha-spot.psd package sha256 `784bcc75f5fabf079432e7cfef2e3306ba823eedd195004cd153a4cc61d8b1ef`
   (29696840 bytes) at opt-level 0 (93.1 s import) and with the override (6.1 s import): identical.
2. Sweep, round-like (16 shards in parallel, `--run-ignored only -E 'test(psd_tools_sweep)'`, test import_psd,
   features import-psd,import-raw): 16 passed, wall 6.6 s; shard_0 (cmyk-alpha-spot.psd) 6.0 s (was 99.0 s serial),
   shards 1-15 0.6-2.4 s (were 6.5-37.5 s); 315 files: 314 ok, 1 typed refusal (psd_layer_rect), 0 panics (same
   coverage and outcome as the c4c6d4d record).
3. Semantics: `cargo nextest run -p hsk-studio-prism -p hsk-studio-interop-psd` under the override: 226 passed,
   2 skipped, incl. wide_determinism `output_is_chunk_repeat_and_cold_start_invariant_with_pinned_digests` and
   `build_has_no_simd_no_fma_contraction_and_receipts_name_the_engine` (bit-exact pinned digests unchanged).
4. Old R25 shard 3 composition: 51 files 52.8 s + this file 99.4 s at opt-level 0 (~152 s) -> with the override this
   file is 5.5-6.5 s, so even the 6-shard layout would be far under 120 s.

Not proven: a full round run with the patch in the root Cargo.toml (needs the steward edit); compile-time delta per
crate at opt-level 3 (only the one-time 52 s rebuild of the harness closure was measured).

</topic>
