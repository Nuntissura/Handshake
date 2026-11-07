---
file_id: LANE-SMB-PR-EXPORT-RECIPE-1
file_kind: lane_brief
updated_at: 2026-11-07T17:22:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-EXPORT-RECIPE-1
lane_file: builders/lanes/hsk-studio-pr-export-recipe-1.json.json
crates: c4d06841
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="StudioExportRecipe with the 453-row export parameter dictionary and declared container capabilities, consumed by the real export path, plus the export publish-safety fixes; no new writer.">

## Target

## Target

Spec: resolve `.GOV/spec/SPEC_CURRENT.md`, v02.229, module `14-studio-creative-suite.md`; `SM:` below is a line in that module. Code: `crates/hsk-studio-reel/` at c4d06841 unless named. Nothing was run; every code fact is read.

**The gap.** Spec law says every output is a `StudioExportRecipe`, a typed parameter set (STU-VID-060 SM:24889, STU-IO-101 SM:21135). No such record exists: the name appears in no code at c4d06841, and `ExportRecipe` is four fields (`src/export.rs:106`). 0 of the 453 parameter identifiers of STU-VID-061 are in code (no `ADBE` token in `crates/`). STU-VID-062 (SM:25503) wants each container to declare what it carries; today that knowledge exists only as refusals inside `check_combination` (`src/mux.rs:224`). So each later writer would invent its own settings, and a preset is six hand-built Rust values (`src/presets.rs:47`). Separately, the export publish step can destroy other work (item 1).

This lane does not add a writer. It builds the contract every writer plugs into and makes the real export path consume it. The next lane adds writers (`next_slices`). Say so in your report: a table nothing reads is not reach, which is why items 5 and 6 are required.

### 1. Publish safety, red first (reported by the lead from SMB-SESSION-WRITE-SAFETY-1; that lane's owned paths exclude reel)

New file `tests/export_publish_safety.rs`, with `dir` and `entries` shaped like `tests/export_queue.rs:107`, `:114`. Each red proven by running.

a. **Fixed sidecar temp.** `src/export.rs:864-867` names it `<output stem>.vtt.partial`, and `:1146` `std::fs::write` overwrites whatever is there. Red: a sentinel file at that exact name survives a captioned export byte for byte. Fix: unique name, `create_new`, same directory.

b. **Another queue's temp is deleted.** `temp_path` (`:755-761`) uses pid plus item id, and every `ExportQueue` restarts ids at 1 (`:372`, `next_id` default 0). `run_item` computes the temp before `plan` (`:798`) and on ANY error removes it (`:836`), though the comment says "Our own temp file only". So in one process, queue B on the same output, refused in `plan` or failing `create_new` (`:1076`), removes queue A's live temp. Red: queue A held mid-run by a barrier in its video source, queue B runs on the same output and refuses, then A is released: A must end Complete with its file published. STU-IO-162 (SM:21418) says never by racing. Fix: a process-wide counter in the temp name, and remove only a temp this item created.

c. **Sidecar temp leak.** `run_item` removes `temp` only (`:836`). Any failure after `:1146` (`:1153`, `:1155`) leaves the sidecar temp. Red: output path is a directory with `overwrite` true, captions with sidecar on; after the Failed receipt `entries` shows no temp.

d. **Failed receipt with media published.** `:1155` renames the media, then `:1158` `remove_file(side)` or `:1160` can fail. Red: `<output>.vtt` is a directory with `overwrite` true. Today the receipt says Failed while the new media is on disk, against the module doc at `:19-20` and `:25-26`. Fix: refuse in `CaptionPlan::new` (`:858`) when the sidecar path is a directory. For a rename failure that cannot be pre-checked, the receipt must state what was published. Choose and state the order.

e. **Remove before rename.** `:1152-1154` and `:1157-1159` delete the old file first, so a failed rename loses it, against `:19-20`. Rust `std::fs::rename` replaces an existing file on Unix and Windows and fails when the target is a directory (source in `research_basis`), so remove the deletes. A failing rename is hard to force, so prove what you can and state the rest.

V4-06 sweep for this class in reel and reel-native. The read found only `src/export.rs:760`, `:836`, `:866`, `:1076`, `:1146`, `:1148`, `:1153`, `:1155`, `:1158`, `:1160`; reel-native has no hit for `fs::write`, `fs::rename`, `remove_file`, `create_new` or `File::create` under `src/`. List each site, fixed or not applicable with a reason. Do not touch harness-view writers: SMB-SESSION-WRITE-SAFETY-1 owns them.

### 2. Parameter dictionary (MT-33767..33797, STU-VID-061 SM:24894, 061b SM:24924)

New `src/export_parameters.rs` and a data file beside it, all 453 rows of the facet tables at SM:24952..25501, one const row each.

Row fields, named as STU-IO-102 SM:21137: `identifier`, `label` (Option), `type_code`, `facet`, `hard_min`, `hard_max`, `soft_min`, `soft_max`, `default`, `unit`, `precision`, `is_slider`, plus provenance `used_by_preset_count`, `used_by_formats`, and `bound_state` with the five tokens of STU-FX-106 SM:12816.

Closed enums: 12 type codes (STU-IO-104 SM:21162), 14 facets (STU-IO-105 SM:21181).

Type column to code, with measured counts: `signed integer` 226 signed_integer; `tab or section group` 72 tab_group; `container / group header (no value of its own)` 71 group_header; `enumerated integer chosen from a constrained list` 24 enumerated_integer; `floating point` 23 floating_point; `button / action (no stored value)` 18 action; `file or folder path string` 11 path; `boolean` 3 boolean; `multi-instance group (repeating rows)` 2 repeating_group; `opaque / arbitrary data` 1 opaque; `colour` 1 colour; `string` 1 string.

Facet counts (the spec table and my parse agree): other 178, publishing_destination 86, video_codec 41, audio 36, rate_control 34, colour 22, layout 16, gop_structure 13, multiplexing 10, video_frame 5, vr_immersive 5, captions 4, metadata 2, performance 1.

Measured bound shapes, all 453: none declared 365; `hard_max` only 8 (`ADBEVideoJ2KColorPrimaries`, `ADBE10BitBlackPoint`, `ADBE10BitWhitePoint`, `ADBEBlackPoint`, `ADBEHighlight`, `ADBEWhitePoint`, `VideoVariableBitrateType`, `EXRCompressionLevel`); `false`/`false` 14; equal numeric pair 35 (signed 15, tab group 19, opaque 1); min above max 2; `0,0` text 1; true range 28. Soft, default and unit are `--` on all 453; precision is 2 on 209 and `--` on 244; slider yes on 37; labels on 44.

Traps. Do not "clean" any of these; store them as the table says:
- `--` is absent, never 0, never unbounded by decision, never copied from its twin (061b, STU-IO-103 SM:21160). STU-FX-106 tells `declared_hard_only` to set soft equal to hard; STU-IO-103 and 061b forbid that for export. Follow the export clauses and name it in notes.
- `ADBEVideoCodec` (SM:25465) and `ADBEAudioCodec` (SM:25437) declare 0 and -1. `hard_min <= hard_max` is false, and an honest range check refuses every value.
- 15 more signed-integer rows declare 0/0, for example `ADBEExportColorSpace` SM:25013. 061b explains 0/0 only for group headers.
- `ADBEVideoFPS` (SM:25486) is typed boolean with bounds 1..254016000000: it is ticks per frame (STU-IO-142 SM:21337).
- `ADBEVideoAspect` (SM:25485) declares the text `0,0`.
- `ADBEAudioNumChannels` (SM:24972) declares -51..32.
- `ADBEFTPPassword` (SM:25313) is typed enumerated integer; refresh tokens are typed path.
- Identifiers `0`, `13`, `51` and other bare numbers are real rows.
- `ADBEAudioCodec` says used by 1701 presets, above the 1,541 of STU-VID-061: do not assert an upper bound.
- Three labels are sentences (SM:25416, :25417, :25419): keep them verbatim.

Proof:
- `tests/fixtures/stu-vid-061b-rows.txt` holds SM:24952..25501 verbatim. SHA-256 of those lines, LF-joined with a trailing LF, is c19e4b92f419d768904ce66fc4336dc914b2cbb3457978608cbb9292fff89ab3 (58,934 bytes), measured from v02.229. The test normalises CRLF, pins that hash, parses every row, and compares every cell to the const table.
- Tests: per-facet counts; 453 distinct; 44 labelled; every soft, default and unit field absent; STU-IO-144's worked example `ADBEAACAudioMode` hard 0..3, precision 2.
- A table typed by hand or by a throwaway generator is fine. Commit no generator (repo governance minimisation), add no `build.rs`, read nothing from the spec at build or test time.

### 3. Honest labels and units (STU-IO-106 SM:21183, STU-VID-061a SM:24919, 061b item 4)

`label_source` on each row: `recovered` for the 44, `identifier_fallback` for the 409. Author no label and no unit in this table. `unit` is typed so `undeclared` differs from `dimensionless`. A public coverage count returns (453, 44, 409).

### 4. `StudioExportRecipe` and validation (STU-IO-101, 103, 108, STU-VID-063 SM:25554)

New `src/export_recipe.rs`. Record: `recipe_id`, `name`, `target_format`, `parameters` (BTreeMap of identifier to typed value), `source_scope`, `output_targets`, `does_video`, `does_audio`, `metadata_export_option`, `standard_filters`. `target_format` carries the exporter identity, per STU-IO-141 SM:21335 and STU-IO-108 SM:21187. Lead reading, overrulable: class id = `MUXER_ID` (`src/mux.rs:38`), file type = `Container::as_str`.

Use a new error type with `code()`; do not add a `ReelError` variant. `ReelError` derives `Eq` and is matched exhaustively in `src/report.rs`, so a value-carrying variant would ripple.

Refusals, all before any frame, none clamping (spec token where one exists):

| Case | Code | Clause |
|---|---|---|
| Value outside a declared usable hard bound | `EFFECT_PARAM_OUT_OF_RANGE`, naming identifier, value, both bounds | STU-FX-104 SM:12785 |
| Value on a row with no declared bound | accepted unclamped at this stage | STU-IO-103 |
| One-sided row | known side enforced, other not | STU-FX-104 |
| Non-finite number; wrong value kind | typed refusal | V4-01 |
| Value on type codes 1, 7, 8, 11 | typed refusal; the dictionary still keeps those rows | STU-IO-104 |
| Numeric row whose `unit` is undeclared and has no authored unit | `export_parameter_missing_unit` | STU-IO-175 SM:21480 |
| Codec integer with no exporter identity | `export_codec_integer_without_exporter` | STU-IO-177 SM:21484 |
| Frame rate not given as integer ticks | `export_frame_rate_stored_as_float`; 8475667200 and 8475675675 stay distinct, as do 10594584000 and 10594594594 | STU-IO-178 SM:21486 |
| Any `publishing_destination` row, or any credential row outside that facet | `export_credential_in_recipe_record`, naming the recipe and field, never the value | STU-IO-147 SM:21365, STU-IO-185 SM:21500 |

Credential rows outside the facet, measured: `ADBEStockRefreshToken` SM:25225, `ADBEStockLogin` SM:25224, `ADBEStockLoginSubGroup` SM:25072.

Degenerate rows, default until the lead rules (open question): the 17 signed rows with min >= max keep their declared cells, are named in a const list, and are not range-enforced. A test pins that list.

### 5. Container declaration (STU-VID-062, STU-IO-141; MT-35261)

`Container::capability()` in `src/mux.rs` beside `check_combination`, declaring exporter identity, does_video, does_audio, video codecs, PCM layouts, colour metadata, caption streams and timecode tracks.

Facts read:
- VC-3 refused in MP4 (`:261`), allowed in MOV. AV1, H.264, HEVC have no container refusal.
- PCM 16/24/32 integer or 32 float; MP4 rate cap 65535 (`:282`).
- Colour is `nclx` in MP4, `nclc` in MOV (`:1032-1033`).
- Captions are `tx3g` through `write_text_track` (`:410`). No container check in the lines read to `:440`; confirm.
- `mux.rs` has no timecode track code (grep `tmcd` and `timecode` empty), so timecode is declared false.

Proof that the declaration cannot drift: an in-crate differential grid over container x video (none, av1, h264, hevc, vc3) x PCM (none, 16, 24, 32, 32 float, plus invalid 8 and 64 float) x rate (48000, 65536): the capability answer equals what `Muxer::new` (`:371`) answers for every cell. Reuse `provisional_entry` (`src/export.rs:541`) for entries, as `plan` does.

Scope limit: declare only MP4 and MOV. Do not add rows for formats with no writer and do not add `Container` variants; that is the next lane.

### 6. The real path consumes the recipe (STU-VID-063: a user recipe is the same record as a shipped one; MT-35295)

a. Each of the six presets (`src/presets.rs:47`) yields a `StudioExportRecipe`. `ExportPreset::recipe` (`:106`) resolves through it. Pin, before the change, the exact `ExportRecipe` for all 6 presets x `os_encoder` true and false, with typed refusals unchanged. `tests/export_captions.rs:161` stays green unedited. `preset_recipe` (`hsk-studio-harness-view/src/view/timeline_export.rs:616`) needs no edit; if it does, stop and tell the lead.

b. A public function, proposed name `job_from_recipe` (like `capability`, a builder proposal, not a spec name), returns an `ExportJob`, so `AdmittedExportQueue::submit` (`hsk-studio-harness-view/src/view/job_admission.rs:247`) admits it unchanged (HOST INTEGRATION 6). It reads source facts before the first frame.

c. Consumed identifiers, a closed list, each with a `studio_declared` unit recorded in the writer binding, not in the dictionary row:

| Identifier | Spec line | Declared hard | Consumed as | Unit | Mismatch |
|---|---|---|---|---|---|
| `ADBEVideoWidth` | SM:25489 | 4..30000 | must equal source `format().width` | pixels | refuse |
| `ADBEVideoHeight` | SM:25488 | 4..16384 | must equal source `format().height` | pixels | refuse |
| `ADBEVideoFPS` | SM:25486 | 1..254016000000 | must equal `ticks_per_frame()` | ticks per frame | refuse |
| `ADBEAudioRatePerSecond` | SM:24974 | 8000..192000 | must equal source `rate()` | hertz | refuse |
| `ADBEAudioNumChannels` | SM:24972 | -51..32 | must equal source `channels()`; a negative value is refused, its meaning is not established | channels | refuse |
| `ADBEVideoTargetBitrate` | SM:25419 | 0.19..240 | OS encoder `OsVideoSettings.bitrate`, STU-IO-143 SM:21357 | megabits per second | n/a |

Width 20000 is inside the dictionary and refused by the muxer (`max_side` 16384, `src/mux.rs` `MuxLimits`): test it as a writer refusal, not a dictionary one.

d. Any other valid parameter: typed refusal, proposed code `export_parameter_not_consumed` (no spec token exists; the lead may rename it), naming the identifier, before the first frame. Never silently ignored. This is the STU-VID-062 rule that an unsupported combination is refused before encoding starts.

e. An H.264 or HEVC recipe with no target bitrate is `export_preset_incomplete` naming it (STU-IO-179 SM:21488). Do not relabel the six presets as complete against the 453.

f. End to end: one VC-3 MOV export through `job_from_recipe` publishes bytes identical to the same job built by hand. Refusal tests assert the source's frame calls stayed 0, as `tests/export_queue.rs:280` does.

### 7. Hostile input (V4-01/02), red where it is a new check

Bounded before allocation or clone: parameter count at most the row count; identifier at most 128 bytes, an unknown id refused without echoing it; text and path values at most 4096 bytes; `name` at most 256 bytes; `recipe_id` at most 128 ASCII; `output_targets` at most 64, each at most 255 bytes with no separator, `..`, NUL, drive colon or empty name; `standard_filters` at most 64. Numbers: NaN, infinities, -0.0, i64 extremes, and bitrate conversion overflow are typed. Lookup is O(log n) per parameter. Your limits are lead-overrulable; list them. `deny(clippy::arithmetic_side_effects)` in the new modules, as `src/export.rs:27`.

### 8. V4-06 failure-class sweep

Besides item 1: every place in reel that assumes `min <= max` for a parameter; every place that reads `ExportRecipe` and could ignore a new setting (`plan` `:621`, `make_encoder` `:563`, harness-view `render_queue.rs:199-251`, read only); every test that would collapse hard and soft. List each with its result.

### 9. Determinism

No clock, random id or locale in any record (STU-IO-163 SM:21420). `StudioExportRecipe::to_json` is deterministic and hand-written like `ExportReceipt::to_json` (`src/export.rs:299`), because reel has no serde and `tools/workspace-probe.rs:495` does not allow it. No JSON parser in this lane. Two calls give identical bytes; BTreeMap order pinned.

### MTs to read

Read, claim a row only if you read it and this proves it, do not edit MT files.

Dictionary: MT-33766, MT-33767, MT-33768..33781 (14 facet rows), MT-33782, MT-33783, MT-33784..33797 (14 tables, in alphabetical facet order: audio, captions, colour, gop_structure, layout, metadata, multiplexing, other, performance, publishing_destination, rate_control, video_codec, video_frame, vr_immersive).

Record: MT-33368..33373, MT-33375, MT-33405, MT-33406, MT-33408, MT-33411, MT-33418, MT-35261, MT-35295. Descriptors: MT-33428, MT-33430, MT-33431, MT-33432, MT-33438.

Expect partly met on: MT-33369 (no `group_path`, step fields or `is_hidden` in the source table; do not invent them), MT-33766, MT-33782 (no authored labels), MT-35261 (two containers), MT-35295 (no portable import, not the 704). These contracts name `hsk-studio-pulse` as crate and pulse proof commands, the same class as F-R55-3: report it, do not repoint.

Vendor parity UNVERIFIED (D5). Every bound is the spec's capture; nothing here was checked against a vendor app.

### Out of scope

New writers or `Container` variants; image sequences; MXF; classifying the 178 `other` rows or authoring the 409 labels (STU-VID-083 SM:25697); `group_path`; soft bounds, defaults; importing the 704 identities; persistence (STU-IO-166 SM:21426 is the host's SurrealDB; add no other database); any new command, control id, or `RUN_COMMANDS` row; manual.rs; timeline render state, proxies, offline, peak jobs, default transitions, time remap (`next_slices`); harness-view writers; any GUI or panel. No egui, eframe, winit, wgpu, naga or WGSL line. No GPU work; this lane has no pixels. No `Cargo.toml` or `Cargo.lock` change. No Master Spec edit: propose SD items. Never touch WP-KERNEL-012 files or refund-claim-20261021.md.

PUSH HOLD: if STUDIO-R59 is not recorded when you start, push nothing and ask the lead (at this scope's read, `validation/round-R59.json` did not exist). One lane pushes at a time.


LEAD NOTE: the code facts in this brief were verified at an earlier origin (c4d06841 or the sha named). About 15 lanes landed since, including SMB-COMMAND-SURFACE-1 (typed rows), SMB-DOC-NEW-1 (document.new, artboards, pages, HistoryStep.containers), SMB-LAYOUT-PDF-HOST-1, SMB-DOC-EXPORT-1, SMB-PROJECT-BIN-1 and SMB-WEB-HOST-1. Re-verify every file:line at your base before relying on it, and say in your report which facts had moved. C: is over its cap: build only on D:. Lanes land one at a time: hold your push for the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: reel returns typed refusal codes with identifiers and bounds; emission stays with the host, and a credential value is never in any message. ExportQueue has no observe call today (no observe hit in src/export.rs) and this lane does not add one.", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: nothing commits to a document; recipes are values and export is a read of the document plus a write of new artifacts (STU-IO-162). No actor is taken or defaulted.", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r53-followups-1`, branch `lane/smb-pr-export-recipe-1`, base aa68164c)

['crates/hsk-studio-reel/src/export.rs', 'crates/hsk-studio-reel/src/presets.rs', 'crates/hsk-studio-reel/src/mux.rs (capability declaration beside check_combination only)', 'crates/hsk-studio-reel/src/lib.rs (mod lines, re-exports, DESCRIPTOR)', 'crates/hsk-studio-reel/src/export_parameters.rs and its data file (new)', 'crates/hsk-studio-reel/src/export_recipe.rs (new)', 'crates/hsk-studio-reel/tests/export_publish_safety.rs (new)', 'crates/hsk-studio-reel/tests/export_parameters.rs (new)', 'crates/hsk-studio-reel/tests/export_recipe.rs (new)', 'crates/hsk-studio-reel/tests/fixtures/stu-vid-061b-rows.txt (new)']

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-r53-followups-1`): step 0; ['lane-check step 0, then clippy over hsk-studio-reel and its direct dependents hsk-studio-harness, hsk-studio-harness-view, hsk-studio-reel-native', 'nextest WHOLE hsk-studio-reel in the default unit and in each round unit of .config/round-features.toml:22: av1, prores, av1-encode, av1 + av1-encode, studio-aac. av1-encode matters: presets resolve differently under cfg!(feature = av1-encode), src/presets.rs', 'cargo test --doc -p hsk-studio-reel (lib.rs carries a compile_fail doctest; rule 4d if you add doc examples)', 'dependents that pin the export path, focused: hsk-studio-harness tests timeline_export, composition_export, timeline_captions, render_queue_records, render_queue_settings, motion_records_session; hsk-studio-harness-view tests viewer_timeline_nest, viewer_timeline_nest_2. Feature unit for these NOT_INSPECTED: read .config/round-features.toml before the run (rule 4e), a test that compiles to zero tests proves nothing', 'not needed: hsk-studio-package suite, because no Folio field is added (rule 4b)']; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r53-followups-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
