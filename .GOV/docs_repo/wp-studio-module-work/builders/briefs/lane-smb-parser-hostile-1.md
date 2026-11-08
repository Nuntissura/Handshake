---
file_id: LANE-SMB-PARSER-HOSTILE-1
file_kind: lane_brief
updated_at: 2026-11-08T06:00:13Z
wp: WP-KERNEL-STUDIO
lane: SMB-PARSER-HOSTILE-1
lane_file: builders/lanes/hsk-studio-parser-hostile-1.json
crates: [hsk-studio-press-read, hsk-studio-press-write (render.rs guard only), hsk-studio-package (bounded_xml), hsk-studio-interop-vector-svg, hsk-studio-interop-layout; tests only: hsk-studio-press, hsk-studio-interop-tiff, hsk-studio-interop-psd, hsk-studio-interop-fig, hsk-studio-develop]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close two file-data abort routes (PDF reader unguarded outside flatten, XML element recursion before any depth check) and add a deterministic fixed-seed mutation test with allocation and reach checks over PDF, SVG, IDML, .fig, PSD, TIFF and RAW; prove the export.rs:384 candidate bounded.">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/rulings-20261108-width-lanes.json, entry `SMB-PARSER-HOSTILE-1`: read its `rulings`, `stale_facts` and `overlap_controls` in full before you code. They replace this scope's open-question defaults and correct its facts, which were written at c4d06841; origin has moved 102 commits to cede5958. Where a stale fact and the scope text below disagree, the stale_facts entry wins, and you re-read the code at your base.

- Dispatch: Safety track (blocker_decisions scoped_lanes): code now on a free C: tree, bundled run after R60, and yield the push slot to any feature lane ready at the same moment. Priority is higher than when scoped: a damaged PDF on disk now reaches the unguarded render route from the Viewer.
- Tree: wt-studio-lanes/smb-layout-composer-depth-1 (C:, target C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target/lanes/smb-layout-composer-depth-1). Its old branch lane/smb-layout-pdf-host-1 at a26b4d11 is backed up on GitHub as lane/smb-layout-pdf-host-1 (pushed by the lead 2026-11-08). Confirm with `git ls-remote origin refs/heads/lane/smb-layout-pdf-host-1` before `git switch -c lane/smb-parser-hostile-1 cede5958`.
- **No cargo on C: until the lead says R60 has released C:.** A validation round holds C:, and one builder hold stalls it. Until then, write code and tests without building. Read the code with git grep, and stop at REVIEW READY with an unbuilt tip; the pre-build review needs no build. After release: cargo check and focused tests, then the bundled run.
- Landing order (HANDOFF workflow item 16): HANDOFF item 16: lead releases one push at a time. Proposed order of these four after the in-flight lanes they depend on: SMB-TYPE-INDENT-SPACING-1 (no in-flight overlap) first; SMB-PARSER-HOSTILE-1 (no in-flight overlap; safety track per blocker_decisions scoped_lanes, so it yields its slot to any feature lane ready at the same time); SMB-AI-LIVE-EFFECTS-1 after SMB-ARTBOARD-EXPORT-1; SMB-SCORE-EFFECTS-6 after SMB-DRIVE-TYPED-2.

Ruling headlines, details in the file:
- OQ1 A PDF that PdfDoc::open refuses but hayro's repair could read reaches hayro with no bound. Refuse, or keep reading and accept the abort? -> Refuse with Press's own error (default), with the stop rule: list every Illustrator oracle file before and after; if any file that read before now refuses, stop and send the list.
- OQ2 XML_MAX_NESTING 256 is chosen, not measured. -> 256 (default), reporting the deepest nesting met in the corpus files the suites read.
- OQ3 R-C02 (older binary copies of the chosen tree) needs fresh approval if the lane starts first. -> Moot.
- OQ4 Sibling scopes name smb-pr-retime-output-1 twice. -> Resolved: four distinct C: trees are assigned (see tree_plan).
- OQ5 cargo-fuzz off to the side? -> No (default) until the in-tree sweep stops finding defects; Operator decision later.
- New: the item 9 recount at cede5958. -> Classify press-write/src/export/tests.rs:58 (Pdf::new) and :75 (interpret_page) as test-only before the recount, so the count matches: product routes are flatten.rs:523/:540 (guarded at :461), vector_read.rs:621/:655, render.rs:127/:147; roxmltree 4 (svg_adapter.rs:649, idml_decode.rs:80, epub.rs:411, :420).
- New: PSD seeds. -> Add compose_psd (interop-psd/src/compose.rs:170, landed with SMB-DOC-EXPORT-1) as one more seed builder for the PSD target: layered files with groups, luni names, iOpa and PSB above 30000 px.

HANDOFF next-work item 9, parser hardening, and the check of `interop-fig-native export.rs:384`. Verified at c4d06841 (crates under `handshake-studio-assets/crates/`). Evidence: `S/research/truth-check-20261105-code-health.json` q1 `build_scope`, q2 `reachable_aborts` (read at 1940292c; every line below re-read at c4d06841). No cargo was run for this scope: every claim that something aborts is from reading, UNVERIFIED until your red run.

**The gap in plain words.** Two parsers can be killed by a tiny file, and nothing tests the parsers against damaged input in general.

1. A stack overflow is not a panic: it ends the process, so `catch_unwind` and the thread join do not help (`press-read/src/render_bounds.rs:9-13`). The project knows this and guards one route: `render_bounds::check` has one caller, `press-read/src/flatten.rs:461`. The same reader runs unguarded on the other two routes: `press-read/src/vector_read.rs:621` then `:655`, and `press-write/src/export/render.rs:127` then `:147`. `read_vector_document` even continues after Press's own reader refuses the file: `read_sources` returns None when `PdfDoc::open` fails (`vector_source.rs:546-548`), and `read` still calls `Pdf::new` (`vector_read.rs:614-621`). Product reach: `Viewer::import_ai` (`harness-view/src/view/vector/interop.rs:407`, `:205`, `:103`; panel action at `view/hosted.rs:1416`), so a damaged `.ai` file reaches it. Also `interop-layout/src/pdf_place.rs:158`, which opens with `PdfDoc::open` first (`:155`), and that does not read stream contents. `render_page`'s only product caller feeds it Studio's own export (`interop-fig-native/src/export.rs:815`). The pinned reader recurses per bracket with no limit (hayro-syntax 0.7.2 `src/object/array.rs:72`; `Cargo.lock` pins hayro-syntax 0.7.2, hayro-interpret 0.7.0). `RENDER_STACK_BYTES` is 256 MiB (`flatten.rs:126`), called margin, not the guard, by `render_bounds.rs:12-13`. Flate lets a very deep content stream fit in a small file. Existing proof stops short: `press/tests/flatten_r13.rs:288-333` runs its abort shapes only through `redact_and_save`, and `press/tests/pdf_delivery.rs:2374-2385` only through `PdfDoc::open`.

2. roxmltree 0.21.1 parses element nesting by recursion (`src/tokenizer.rs:584` `parse_element`, `:639` calls `parse_content`, `:662`), with no depth limit; `nodes_limit` is the only bound (`src/parse.rs:561-563`). Every call passes a node budget far above a safe depth: `interop-vector-svg/src/svg_adapter.rs:644-649` (20,000, `interop-vector-core/src/model.rs:30`), `interop-layout/src/idml_decode.rs:74-80` (1,000,000, `model.rs:50`), `interop-layout/src/epub.rs:409-420` (1,000,000, two parses). SVG's `max_depth` 32 (`model.rs:31`) is checked at `svg_adapter.rs:1057`, after the parse. The only depth test uses 10 levels (`interop-vector/tests/vector_edit_roundtrip.rs:251-257`). Then `idml_longdoc.rs:291-294` `walk` recurses on `ParagraphStyleRange` and `CharacterStyleRange` with no depth word (called at `:549`, `:997`, `:1026`). Reach today is the public library API: `git grep` finds no caller of `import_svg`, `import_idml`, `open_idml`, `import_longdoc` or `inspect_epub` in harness, harness-core, harness-view or session at c4d06841. It is an abort waiting for its first host.

3. General damage. Code health records 0 fuzz or property-test files. What exists: byte sweeps in `interop-tiff/tests/tiff_codec.rs:186-205` and `real_tiff.rs:150-163`, `interop-raster/tests/jpeg_codec.rs:148` and `png_codec.rs:191`, one in `develop/tests/develop_dng_write.rs:490-500` over `exif_subset` only, and an unmutated corpus sweep in `harness/tests/import_psd.rs:449-494` (`#[ignore]`). They discard the result (`let _ =`), so they catch panics only: no allocation bound, and nothing shows the damage got past the first check. A search of the test trees finds no byte-mutation sweep for PSD, PDF, `.fig`, IDML, SVG or the camera RAW decoders; targeted hostile tests exist (`interop-psd/tests/foundation_contract.rs:566`, `press/tests/pdf_delivery.rs:462`, `interop-fig/tests/fig_instances_frames.rs:440`).

**`interop-fig-native export.rs:384`: the answer, from reading.** Not the same class as F-R56-1 from file data. `children_of` (`:384`) and `layer` (`:392`, `:450`) do recurse with no guard, but the depth is bounded before they run. Both product callers export a proposal from `to_native` (`harness-view/src/view/prototype.rs:768`, `:802-809`). `to_native`'s walk stops at `depth > 64` with loss `native.depth` (`native.rs:3099-3101`). Depth rises once per level (`:3052`) and passes through instance expansion unchanged (`:3122`, `:3372`, `:2757`, `:2797`), so instances do not multiply it, and an expansion adds one group layer per level. The layer tree is about 65 levels. The truth check's 4096 (`Budget.nodes`) is not the operative bound. `NativeProposal` has public fields (`native.rs:277`), so Rust code could hand it a deeper tree; a file cannot. Frame size is UNVERIFIED: item 8 proves it by running. Change `export.rs` only if that test fails. The larger stack user is probably `walk` itself, which holds `patched` on its frame (`native.rs:3106`), UNVERIFIED.

**Recommendation on the method: a deterministic in-tree mutation test. Do not add cargo-fuzz to this lane or to rounds.** Reasons are in `research_basis`. Short form: `rust-toolchain.toml` pins stable 1.97.1, cargo-fuzz needs nightly, and a run that differs every time cannot be a round row.

## Items, red first

**1. Red: XML nesting.** New files `interop-vector-svg/tests/hostile_xml_depth.rs` and `interop-layout/tests/hostile_xml_depth.rs`, each its own binary, because an abort takes its binary down. A stack overflow counts as the test failing. Run each case on an explicit `std::thread::Builder` stack, not the default.

| Case | Input | Entry | Required after item 2 |
|---|---|---|---|
| svg_openers | `<svg xmlns=...>` plus `<g>` x 19,990, never closed | `import_svg`, `ImportPolicy::default()` | `VectorError::BudgetExceeded("depth")` |
| svg_closed | the same, closed | `import_svg` | same |
| idml_story | a Story part with `<ParagraphStyleRange>` nested x 500,000 | `decode_idml` with a built `IdmlPackage` (shape: `tests/decode_amplification.rs:97`) | `IdmlError::BudgetExceeded("xml_depth")` |
| idml_note | Footnote fragment nested x 500,000 | `import_longdoc` (`idml_longdoc.rs:995-997`) | same |
| epub_container, epub_opf | deep `container.xml`, deep OPF | `inspect_epub` (`epub.rs:397`) | `EpubError::NotEpub("container_xml")`, `NotEpub("opf_xml")` |
| within_cap | depth 200 | each entry | the same answer the base gives |

Record the smallest depth that aborts on a 2 MiB thread, debug, and one release measurement. If a shape does not abort, say so and keep the row as a bound test.

**2. No element recursion past a declared depth.** New `hsk-studio-package/src/bounded_xml.rs` (package already owns the bounded ZIP reader, and both crates already depend on it): `pub const XML_MAX_NESTING: usize = 256` (the same number as Press's `HARD_MAX_DEPTH`, `press-read/src/error.rs:49`) and `pub fn xml_nesting_within(text: &str, cap: usize) -> bool`. Iterative, no allocation, one pass. It skips `<!--...-->`, `<![CDATA[...]]>`, `<?...?>`, `<!DOCTYPE...>` and quoted attribute values; `<name` adds one, `</` removes one (saturating), `/>` nets zero. On anything it cannot read it returns true, so a malformed file keeps its existing code. Call it before all four parses in the table above. Do not change `max_depth` or any node limit. A table test in package (comments, CDATA and attributes holding `<`), and a differential in `interop-vector-svg/tests/hostile_xml_depth.rs`: on generated valid documents the depth equals what roxmltree reports. Assert 256 levels parses on a 1 MiB thread (the Windows default main-thread reserve, per the `/STACK` documentation; where the Viewer runs imports is NOT_INSPECTED). If debug frames fail that, stop and send me the numbers. `idml_longdoc.rs:291` is then bounded by the cap; say so with a test, do not add a second guard.

**3. Red: the unguarded reader.** New `press/tests/hostile_vector_read.rs` (press-read has no tests directory, its proof lives in press). Build small PDFs with the helpers' shapes from `press/tests/flatten_r13.rs:293-303`; `miniz_oxide` is already a press dev-dependency.

| Case | Input | Source of the shape |
|---|---|---|
| content_arrays | page content, FlateDecode, `[` x 4,000,000 | new; chosen so the depth passes a 256 MiB stack at any frame size of 67 bytes or more per level |
| form_arrays | form XObject with the same stream, drawn by `/Fm1 Do` | `flatten_r13.rs:305` |
| type4_procs | Type 4 tint transform, `{` x 4,000,000 | `flatten_r13.rs:307-308` |
| cmap_dicts | ToUnicode CMap, `<<` x 4,000,000 | `flatten_r13.rs:315-316` |
| indexed_self | `/Indexed` whose base is itself, and a two-object cycle | `flatten_r13.rs:311-312`; `hayro-syntax src/object/indirect.rs:29` has a parent-chain check, so this may not abort (`render_bounds.rs:30` says no guard; the run decides) |
| stitching_self | stitching function listing itself | `render_bounds.rs:29-30` |
| unopenable_deep | a file `PdfDoc::open` refuses, with `[` x 4,000,000 at the top level | `vector_source.rs:548` |
| big_image | image XObject declaring 60,000 x 60,000 with 16 bytes of data | `render_bounds.rs:14-15`; the recorder ignores images (`vector_read.rs:508-509`), UNVERIFIED that the reader does not allocate first |
| within_bound | 30 nested arrays | `flatten_r13.rs:335`; must read as before |

Each case runs on `read_vector_pages`, `read_vector_document` and `render_page` at scale 0.25. One row in `interop-layout/tests/hostile_xml_depth.rs` or a new file for `place_pdf_page`. Record which cases abort, and the smallest aborting count for arrays.

**4. Guard the reader before it sees a byte.** In `press-read`: split a nesting-only pass out of `render_bounds::check` (`render_bounds.rs:487`), keeping `construction_chains_within` (`:175`), the tokenizing part of `content_bounds` (`:454`) and `syntax_nesting_within` (`:70`), without the image budget, because reading vectors does not decode images (confirm by reading hayro; if `big_image` allocates, the image bound joins the pass). `flatten`'s behaviour must not change: `renderer_input_is_bounded_before_rendering` and `renderer_input_images_are_bounded` pass untouched. Build the object closure with `copy_closure` (`flatten.rs:406`) for each page. Run the pass before `vector_read.rs:621` and `render.rs:127`. A violation is `PressError::InputLimit("depth")` for both routes (an existing variant, `error.rs:175`; `FlattenRefused` names another operation). A file `PdfDoc::open` refuses is refused with that error, rather than handed to hayro: this changes an answer, see `open_questions`. Page-level walk failures keep `VectorNote::StructureNotRead` as now. Proof: `press/tests/vector_read.rs`, `interop-vector/tests/ai_import.rs` and `render-cpu/tests/d5_ai_oracle.rs` pass with no blessed value changed. List every Illustrator oracle file with its before and after: if any file that read before now refuses, stop and send me the list. Measure the added time on the largest of them.

**5. The mutator.** `interop-tiff/tests/support/hostile_mutator.rs`, included by `#[path]` from other crates' tests (precedent `harness-view/tests/viewer_attribution.rs:10`), so no new crate and no `Cargo.toml` line. interop-tiff is the home because its sweep is the pattern and only develop, harness-core, harness and harness-view depend on it, so `--per-sha` stays cheap. Its self-test runs once, in `interop-tiff/tests/hostile_mutator_selftest.rs`: pin an FNV digest of the first 1,000 outputs for a fixed seed and seed input.

| Part | Rule |
|---|---|
| Random source | xorshift64* from a fixed constant per target (precedent `nib/tests/owned_path.rs:317`, `layout/tests/composer_depth_diff.rs:7`). No clock, no environment |
| Offsets | half within the first 512 bytes, where length fields live; half anywhere |
| Operations, chosen uniformly | truncate; set one byte from {0, 1, 0x7F, 0x80, 0xFF}; flip one bit; overwrite 2, 4 or 8 bytes, either byte order, from {0, 1, 0x7F.., 0x80.., 0xFF.., and the seed's length}; erase a block; insert a block of one repeated byte; copy a block inside the input; replace or insert a token from the target's dictionary; for text formats, replace a decimal number with one of {0, -1, 1e308, 4294967295} |
| Mutations per input | 1 to 4 |
| Worker | an explicit 2 MiB thread, because Rust's default is subject to change and F-R56-1 used 2 MiB |
| Panic | `catch_unwind` per input; a panic fails the test and prints target, seed, index and the bytes as hex when under 4 KiB |
| Abort or hang | nextest's process per test and 120 s cap are the gate (`.config/nextest.toml`). With `HOSTILE_TRACE=1` the loop prints each index before running it, for reproducing only |
| Allocation | a counting `#[global_allocator]` in each target's binary (precedent `interop-layout/tests/decode_amplification.rs:27-55`), using a global atomic rather than that file's thread-local, because `press-read` spawns its own thread. Peak live bytes above the table's cap, for any one input, is a failure |
| Reach floor | each test prints `MUTATION target=.. seeds=.. mutants=.. ok=.. refused=.. caught_panics=.. codes={..} peak=.. slowest_ms=..` and asserts ok of at least 1, at least 3 distinct refusal codes, and at least half of the ok count you measured. A sweep that dies at the magic number must fail, not pass (V4-03) |
| Determinism | fixed count, never derived from time. Every target repeats its first 200 mutants and asserts the same outcome codes |

**6. The targets, one test file each, `Limits::default()` throughout, so production bounds are what is tested.**

| Parser | File | Entry chain | Seeds, generated in the test | Damage reaches inside by | Peak cap |
|---|---|---|---|---|---|
| SVG | `interop-vector-svg/tests/hostile_mutation_svg.rs` | `import_svg` (`svg_adapter.rs:598`) | inline documents covering gradients, clips, masks, patterns, markers, CSS, `use`; one SVGZ | dictionary of tag and attribute tokens, number swaps; raw bytes die at UTF-8 and XML | 256 MiB |
| IDML | `interop-layout/tests/hostile_mutation_idml.rs` | `decode_idml` (`idml_decode.rs:185`), then `import_longdoc` (`idml_longdoc.rs:941`); also `import_idml` (`:175`) on whole bytes | `tests/idml_longdoc.rs:35` `fixture()` shape, copied | mutate the part bytes in `IdmlPackage`, so ZIP CRCs do not stop it | 256 MiB |
| PDF | `press/tests/hostile_mutation_pdf.rs` | `PdfDoc::open` and `proposal`; `read_vector_document` (`vector_read.rs:565`); `render_page` (`render.rs:99`) at 0.25 | `press/tests/vector_read.rs:13` `fixture()` shape, plus a form, a font and an image | dictionary of PDF keys and operators; an uncompressed variant so streams are reachable | 256 MiB |
| `.fig` | `interop-fig/tests/hostile_mutation_fig.rs` | `import_fig` (`import.rs:189`), `to_native` (`native.rs:3448`), `plan_frame_exports`, `export_frame_pdf` (`export.rs:753`) | fixtures `fig-components-autolayout.fig` (45,805 bytes), `fig-basic-shapes.fig` (101,769), and `tests/fig_import.rs:251` `fig_zip` | mutate the decoded message and rewrap with `miniz_oxide` and `ruzstd` as `fig_import.rs:138-142` does; CRC and compression stop raw flips | 256 MiB |
| PSD | `interop-psd/tests/hostile_mutation_psd.rs` | `read_psd` (`document.rs:269`), `decode_merged` (`:237`), `to_proposal` (`proposal.rs:2039`) | `tests/common/mod.rs` builders (`layer_info`, `layer_section`, `rle`, `layer_block`) | dictionary `8BIM`, `8B64`, `Lr16`, `Lr32`, `Patt`, `lfx2`; plain bytes work, no outer CRC | 256 MiB |
| TIFF | `interop-tiff/tests/hostile_mutation_tiff.rs` | `decode` (`decode.rs:12`) with the container API | `encode` over None, LZW, Deflate, PackBits, as `tiff_codec.rs:190-205` | plain bytes; keep the old sweep | 256 MiB |
| RAW | `develop/tests/hostile_mutation_raw.rs` | `decode_container` (`lib.rs:324`), `exif_subset` (`dng_write.rs:750`) | copy, do not refactor, the constructors into `develop/tests/support/hostile_raw_seeds.rs`: `develop_dng.rs:211`, `develop_nef.rs:100`, `develop_nef_compressed.rs:168`, `develop_arw.rs:98`, `develop_cr2.rs:115`, `develop_raf.rs:79`, `develop_cr3.rs:51`, `develop_rw2.rs:11`, `develop_orf.rs:30` | plain bytes | 256 MiB |

Size counts by measurement: no single test over 30 s on an idle host (nextest flags at 30 s, kills at 120 s), split by seed family as `harness/tests/import_psd.rs:498-510` shards. State total added CPU; the rounds have slowed (workflow item 8), so keep it under 5 minutes across all targets and tell me if damage coverage needs more.

**7. A finding is fixed here, red first.** Each panic, abort or cap breach gets a named regression test with the minimised bytes inline, as the `tiff` crate's own `tests/fuzz_tests.rs` keeps found inputs, then the typed refusal in the parser. Never allow-list a mutant, narrow a dictionary or lower a count to get green. Stop and send me the list if one parser yields more than 3 distinct defects or the lane more than 8, or if a fix needs a changed output or a new `Limits` field. A caught panic in hayro (`vector_reader_panicked`, `vector_read.rs:677`) is not a failure but is counted and reported: a typed answer, third-party cause.

**8. Pin the fig answer.** In `interop-fig/tests/hostile_mutation_fig.rs` or a new `fig_depth.rs`, using the hand-built `FigDocument` shape of `fig_instances_frames.rs:440-470`: a chain of 70 nested frames gives loss `native.depth` and a tree at most 65 levels deep; `to_native` and `export_frame_pdf` on the deepest allowed chain, on a 2 MiB thread, finish typed. Also the same chain with instances along it, asserting instance nesting does not deepen it past that. Report the smallest of 256 KiB, 512 KiB and 1 MiB that passes for each of the two calls. If 2 MiB fails, that is a defect under item 7. Close the candidate in the lane notes by file and line.

**9. Failure-class sweep (V4-06). Class: recursion or allocation driven by file data reached before its bound. Regexes: `parse_with_options\(`, `Pdf::new\(`, `interpret_page\(`, and self-recursive functions in the parser crates. Count at c4d06841, `src` only:** roxmltree 4 (`svg_adapter.rs:649`, `idml_decode.rs:80`, `epub.rs:411`, `:420`): item 2. hayro 3 pairs (`flatten.rs:523` guarded at `:461`; `vector_read.rs:621`; `render.rs:127`): item 4. Self-recursive with no depth word in my name-based scan, which misses trait dispatch and cross-file cycles: `idml_longdoc.rs:291` (item 2); `press-read canon.rs:86`, `show_text.rs:85`, `text_fields.rs:1124`, `writer.rs:17`: these walk parsed objects, and the parser clamps depth at 256 (`syntax.rs:158`, `:228`; read this, UNVERIFIED for each and for debug frames; state the bound or fix); `interop-fig-native export.rs:310` is a false hit, a method calling the free function of the same name. Recount; a mismatch blocks READY. Read only and report, no fix here: `layout/src/toc.rs:300`, `:308`, NOT_INSPECTED how deep levels go; `harness-view/src/view/layers.rs:135`, `:190`, `design/instance.rs:570`, which are Viewer logic for the next slice. Contained by the worker process per code health, not re-read here: `ps-pdf font_embedder.rs:689`, `ps-ops file_ops.rs:1098`, `:1136`.

**10. MTs.** Read MT-33450, MT-35819, MT-35820, MT-35821, MT-445 (all STU-IO-012 recovery, all PENDING). This is a correction lane: claim a row partly met only if you read it and this proves it. Do not edit MT files.

## Clauses

STU-IO-012 (module 14 line 21649): malformed and truncated input fails closed with a deterministic result, authority unchanged. STU-IO-004(e) (line 21525, recovery at 21531). STU-IO-187 (line 231): bounded limits and cycle and overflow checks; it speaks of native snapshot containers, so this lane applies its rule by analogy (lead reading). STU-CON-026 (line 22725): budgets checked before allocation. No clause names fuzzing or mutation testing.

## Hostile input (V4-01/02)

The point of the lane. New `src` modules carry `#![deny(clippy::arithmetic_side_effects)]`. `xml_nesting_within` allocates nothing and is linear. The nesting pass stays inside `Limits::max_decoded_total` through `copy_closure`, cancellation checked as in `render_bounds.rs:498`. Known limit, not closed here: `render_bounds.rs:44-47` records that CPU time inside hayro is not cancellable, and hayro's own README says many ways to trigger timeouts and out-of-memory remain. A hang found by a target goes through item 7. Vendor parity UNVERIFIED (D5): nothing here touches it.

## Out of scope

Any GUI; no egui, eframe, winit, wgpu, naga or WGSL line; no GPU kernel. `harness-view`, `harness`, `harness-core`, `session`, `folio` source. Upgrading hayro or roxmltree, or replacing either (no `Cargo.toml` or `Cargo.lock` change). cargo-fuzz, afl or any coverage-guided tool. Corpus-seeded `#[ignore]` sweeps. Encrypted or password-protected PDF behaviour. AI legacy PostScript, EPS, DXF, Affinity, LRCAT, EDL and captions parsers. New `Limits` fields. Changing `PARA_MODEL` or any fitted value. Master Spec edits. CI and a clippy gate (HANDOFF item 9, separate).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: parsers return typed refusals, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: parsers commit nothing", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-layout-composer-depth-1`, branch `lane/smb-parser-hostile-1`, base cede5958)

- press-read: `src/vector_read.rs`, `src/render_bounds.rs`, `src/flatten.rs` (only to expose `copy_closure`), `src/lib.rs` DESCRIPTOR.
- press-write: `src/export/render.rs` (guard call only).
- press: `tests/hostile_vector_read.rs`, `tests/hostile_mutation_pdf.rs` (new).
- package: `src/bounded_xml.rs` (new), `src/lib.rs` (one mod line and re-export), a new test file.
- interop-vector-svg: `src/svg_adapter.rs` (the call before parse), `tests/hostile_xml_depth.rs`, `tests/hostile_mutation_svg.rs` (new).
- interop-layout: `src/idml_decode.rs`, `src/epub.rs` (the calls before parse), `src/error.rs` only if a code needs it, `tests/hostile_xml_depth.rs`, `tests/hostile_mutation_idml.rs` (new).
- interop-tiff: `tests/support/hostile_mutator.rs`, `tests/hostile_mutator_selftest.rs`, `tests/hostile_mutation_tiff.rs` (new).
- interop-fig: `tests/hostile_mutation_fig.rs`, optional `tests/fig_depth.rs` (new).
- interop-psd: `tests/hostile_mutation_psd.rs` (new).
- develop: `tests/hostile_mutation_raw.rs`, `tests/support/hostile_raw_seeds.rs` (new).
- Any parser `src` file a finding names, under item 7: tell me which before READY.
Commit plan, 10 at most: red XML; XML guard; red PDF; PDF guard; mutator with self-test and TIFF; SVG and IDML; PDF; `.fig` with the depth pin; PSD; RAW. Small fixes fold into the target's commit, red first.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-layout-composer-depth-1`): step 0; clippy over the changed crates and their direct dependents; nextest WHOLE suites of press, press-write, press-read (src unit tests), package, interop-vector-svg, interop-vector, interop-layout, interop-tiff, interop-psd, interop-fig, interop-fig-base, interop-fig-native, develop; render-cpu `--test d5_ai_oracle` (corpus); doctests if added. Rule 4e: no target may depend on a feature outside the default unit; all new files are default-unit. Package changes additively, so rule 4b does not add folio-field work; `--per-sha` covers its 10 direct-dependent manifests.; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-layout-composer-depth-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
