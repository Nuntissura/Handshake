---
file_id: diagnostics-press-proof-soundness
file_kind: diagnostics
status: current
role: remediation builder SMB-PRESS (Opus), R7 pre-review B findings N1-N7
lane: SMB-PRESS (MT-35557)
inputs: validation/R7-prereview-b.json (head 0e48167), validation/round-R5.json, validation/round-R6.json, builders/lanes/hsk-studio-press.json
updated_at: 2026-10-11T11:01:59Z
---

<topic id="summary" status="current" summary="why the proof keeps missing bytes, in 8 lines" updated_at="2026-10-11T09:58:00Z">

- R5, R6 and R7-pre all found the same shape: `is_proven()` true while the removed string is still in the saved file. Each fix closed one site; the mechanism that creates new sites stayed.
- The mechanism: the proof is an ENUMERATION of what its own decoders and interpreter produce, followed by a SUBTRACTION. Raw-file hits inside the span of any stream that decoded were discarded (`covered` = whole object span, `sanitized_save.rs:480-482, 554-557`). Every place where the interpreter's model is narrower than the file therefore became a silent exemption.
- Narrow-model sites found in R7-pre: bytes past a filter's end-of-data marker and dictionary comments (N1), inline-image data never decoded (N2), streams not parsed at all (N3), font state without q/Q or `gs` (N4), parse failure = no text reading (N5), forms without own resources read with no fonts (N6), exemption keyed on dictionary listing instead of drawing (N7).
- Two exemption keys were proxies: "the stream decoded" (for raw bytes) and "the stream is listed under page resources" (for kept text). Neither says the bytes are the text a viewer draws.
- New rule set (sound by construction): (1) the raw backstop scans every byte of the output and is never subtracted except where an exact provenance check shows the same needle was judged in decoded context of the stream whose CONSUMED input holds it; (2) every decode reports what it consumed, and non-whitespace input after an end-of-data marker is a typed NotProven; (3) text exemptions need positive evidence: the stream is drawn from a page through the `Do` graph, the show operator's own string values carry the needle, and fonts come from a graphics-state stack (q/Q, `gs` /Font, `Tf`, inherited across `Do` and across a page's content streams); (4) anything that cannot be read where text could be drawn (parse failure, gstate overflow, form cycle or depth, unresolved font in drawn page content) is a typed NotProven, never a skip.
- Needle spellings searched everywhere bytes are scanned: given bytes, PDFDocEncoding, UTF-8, UTF-16BE, UTF-16LE, upper/lower hex of the byte and UTF-16BE spellings; escaped literal forms are matched by decoded value wherever PDF syntax defines a string (objects, dictionary keys, content operands).
- Out of scope, stated in the DESCRIPTOR: application-level encodings inside decoded payloads (zip or base64 inside an embedded file, zlib at an arbitrary offset of a non-content stream), pixels of raster images (P1 is a lead decision).

</topic>

<topic id="evidence" status="current" summary="per-finding root cause at 0e48167" updated_at="2026-10-11T09:58:00Z">

| id | site at 0e48167 | root cause | class |
|---|---|---|---|
| N1 | `sanitized_save.rs:480-482, 554-557`; decoders `filters.rs:139, 294, 370, 407, 452` | decoded stream => whole object span `covered`; decoders return at EOD and discard the rest; dict text and comments sit inside the span | subtraction by proxy |
| N2 | `raster.rs:80-82`, `sanitized_save.rs:487-494` | inline image with implemented filters classed `Searchable` and never decoded | decoder not applied |
| N3 | `sanitized_save.rs:484` | `ops` only for kept/AP streams since c520095, so text/opaque/inline checks off for stray forms | parse gated by role guess |
| N4 | `show_text.rs:106-114` | current font only from `Tf`; no q/Q stack, no `gs` /Font | interpreter narrower than viewer |
| N5 | `sanitized_save.rs:484` (`.ok()`), `show_text.rs:157-160` | parse error => no text reading, reads clean | failure mapped to clean |
| N6 | `show_text.rs:175-185`, `:104` | forms without own /Resources unmapped => fonts None => Simple | interpreter narrower than viewer |
| N7 | `show_text.rs:210-216` | kept = listed in resources, not drawn by `Do` | exemption by proxy |

Shared class regex for the V4-06 sweep: `covered|\.ok\(\)|Err\(_\) =>|unwrap_or_default\(\)|Searchable|kept_text|content_resources|looks_like_content|b"Tf"|b"Do"|return Ok\(ops\)|b'>' =>|b'~' =>|128 =>|257 =>` (sites listed in the lane file `notes.remediation_R7pre.class_sites`).

</topic>

<topic id="design" status="current" summary="backstop and fixes" updated_at="2026-10-11T09:58:00Z">

## Backstop (raw, every byte)
- `scan(output)` over the whole file in every spelling; a raw hit is dropped only when it lies entirely inside the bytes the FIRST filter of stream S consumed AND the decoded analysis of S found the same needle (as a hit or a kept collision). Information argument: such a copy adds nothing beyond what the decoded verdict of S already reports. Hits in dictionaries, comments, keywords, after EOD, between objects: always hits.
- Reader records the absolute start of each stream's data (`Entry::data_at`).

## Decoders (consumed input)
- `filters::decode_chain` returns `(bytes, consumed_by_first_stage, trailing: Option<filter>)`. Flate uses `miniz_oxide::inflate::core::decompress` with the consumed count; LZW stops at 257; ASCIIHex at `>`; ASCII85 at `~>`; RunLength at 128. Non-whitespace input after the marker at any stage => `trailing_data_streams` => `NotProven::TrailingData` (also for inline images and the zlib inside a raster codec name).

## Inline images (N2)
- `content::Op::inline_data` records the data range; every inline image with an implemented chain is decoded and scanned (`PayloadLocation::InlineImage`); raster tail => prefix + hidden zlib; decode failure => `streams_not_decodable` `inline:decode_error`.

## Content walk (N3-N7)
- One interpreter (`show_text::Walker`) used by the proof and by the text-field appearance scan: gstate stack for the font (q/Q, `gs` /Font, `Tf`), forms entered by `Do` with the caller's font and the caller's resources when the form has none, memoised per (stream, resources, font, role), depth 32, cycle => unreadable.
- Roots: pages (all content streams of a page share one state, in order) = role Kept; annotation /AP streams = role Appearance (font fallback: AcroForm /DR); every other content-role stream (Subtype Form, tiling pattern, Type3 glyph proc, anything with /Resources) not reached from a root = role Stray (own resources).
- Exemptions only for streams whose every context is Kept, and only for byte hits inside a show operator whose own string values (or ToUnicode text) carry that needle.
- Unreadable (parse error, dangling operands between page streams, gstate depth > 256, cycle, depth, undecodable form) in any role => `NotProven::UnreadableContent` when a needle search ran. Unresolved font name: Kept => opaque; Appearance => /DR then viewer default font (single-byte, byte-scanned); Stray => single-byte (no drawing context).

## Lead decisions folded in
- PLR-039 (residual pass re-runs the redaction selector): superseded by the backstop; the proof shares no code with the selector (own parse, own walker, every byte scanned). `residual_in_region` stays the separate same-selector re-pass.
- PLR-040 (fail closed): `raster::codec_signature`: JPEG marker chain SOI..EOI with nothing after; JP2 box chain (ftyp + jp2c) or bare SOC SIZ..EOC codestream; JBIG2 embedded segment-header chain (T.88 7.2 types and lengths, no file header per ISO 32000-2 7.4.7); CCITT Columns = Width, Rows = Height, K <= 0, rows decode to the image size (T.4/T.6 tables cross-checked against the MIT `fax` 0.2.7 crate tables in the local registry; own decoder, no dependency added). Otherwise not raster: `streams_not_decodable` `<codec>:<reason>`.

</topic>

<topic id="results" status="current" summary="commits, proofs, residuals" updated_at="2026-10-11T11:02:00Z">

## Commits (branch codex/handshake-studio-assets, all pushed)
- 187e6cf engine + 5 regression tests (N1-N7); a4d2234 N1 test raw-hit assertion only where the token boundary is clean; 1c3fd8c PLR-040 + DESCRIPTOR proof keys.

## Proof
- GREEN: `cargo nextest run -p hsk-studio-press --no-fail-fast` 38/38 at 1c3fd8c; clippy `-p hsk-studio-press --tests --examples` clean (only hsk-studio-observe warnings, not press).
- RED (V4-11 scratch `Handshake_Artifacts/WP-KERNEL-STUDIO/press-soundness-redproof`, git archive 0e48167, own STUDIO_OWNER): the 6 new tests fail on 0e48167 (copy without the 8 lines marked `R7-new-api`); a red-only probe (not committed) shows 40/40 sub-cases read proven on 0e48167: N1 x7, N2 x3, N3 x3, N4 x3, N5 x2, N6 x4, N7 x2, PLR-040 x16.
- V4-06: class regex and 49 sites with status in lane file `notes.remediation_R7pre.class_sites`.

## Residuals for the lead (not fixed, outside N1-N7)
- Simple fonts with `/Encoding /Differences` can draw letters with arbitrary codes; proof and selector read simple-font codes as single bytes or through `/ToUnicode` only.
- Whole-token rule (lane design) lets a needle glued to letters/digits on both sides pass the raw scan; for bytes after an end-of-data marker `TrailingData` blocks anyway.
- Compressed data inside valid codec metadata (JPEG APPn/COM, JP2 boxes other than jp2c) and application encodings inside decoded payloads are scanned, not decompressed (DESCRIPTOR `proof.not_covered`).
- CCITT K > 0 (mixed 2D) is not raster: such images block until a K > 0 row decoder exists.

</topic>
