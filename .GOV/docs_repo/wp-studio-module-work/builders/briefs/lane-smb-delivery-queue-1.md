---
file_id: LANE-SMB-DELIVERY-QUEUE-1
file_kind: lane_brief
updated_at: 2026-11-08T08:53:46Z
wp: WP-KERNEL-STUDIO
lane: SMB-DELIVERY-QUEUE-1
lane_file: builders/lanes/hsk-studio-delivery-queue-1.json
crates: hsk-studio-harness-view (one admitted delivery queue over reel's ExportQueue for sequence recipes and composition render queue items; user StudioExportRecipe records; PNG image sequence with alpha) wi
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Sequences and compositions deliver through one headless queue of typed items with per-item receipts and cancel, user recipes are stored and reused, and a composition writes a PNG sequence with alpha">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/width-plan-20261108-video-motion-audio.json. Every open question below takes the planner's recommended default as a lead ruling. Its code facts were checked at {'commit': '0a733db8', 'subject': 'harness-view, PLACE-ASSET-1: F4 float sources decode unclipped (lead ruling 2026-11-08)', 'baseline_of_scenarios': 'c4d06841', 'commits_since_baseline': 132, 'read_from': 'wt-studio-lanes/smb-pr-edit-footprint-1/handshake-studio-assets via git grep and git show 0a733db8:./crates/... (working tree at 08b372af, not checked out)', 'spec': 'v02.229, .GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md', 'label': 'READ'}; this lane's base is eb17fad6. Re-read every file:line at your base before you edit it.

- STU-IO-166 stores recipes in SurrealDB tables; the asset has no store. Where do user recipes live? -> A document entry export_recipes in the package, recorded as an SD item, because a recipe must travel with the project in the asset-local build and STU-IO-110 makes recipes portable artifacts; a host store can import them later.
- Is the delivery queue saved with the document (like the AE render queue records) or per session? -> Per Viewer, not saved: composition items already persist as StudioRenderQueueItem; sequence items are cheap to re-add; saving needs a new record.
- What is a render-settings or output-module template (STU-MOT-125) in records? -> An output-module template is a user StudioExportRecipe with does_video and the module's fields mapped; render-settings templates are next slice. Refuse queue_apply_template for render settings with a typed code.
- PNG sequence frame-number padding width is not stated by STU-MOT-122. -> Seven digits, because starting_number's hard_max is 9999999 (:28282), so every number fits and names sort.
- Does export_sequence keep its preset_id parameter? -> Yes, as a recipe id alias resolved through ExportPreset::studio_recipe, so the CLI and existing tests keep working.

- Process: red first per item, then focused runs on D:. Stop at REVIEW READY for the read-only code-check and red-team review, then run ONE bundled run, then stop at BUNDLED GREEN. Lanes land one at a time on the lead's RELEASED (HANDOFF topic workflow, items 10, 11 and 16).
- Any lane that changes a Viewer command's refusal order also runs every harness process test that drives that command. This is the lesson from the 2026-11-08 trunk break at 35c50d3b.

## Target

Delivery has two engine halves and no product path joining them. Sequence export takes one of six reel preset ids (`harness-view/src/view/timeline_export.rs:616` `preset_recipe`, `timeline/export_media.rs:505` `export_sequence`), while `StudioExportRecipe` (`reel/src/export_recipe.rs:273`, `resolve` `:547`, `job_from_recipe` `:739`, shipped presets as the same record `reel/src/presets.rs:110`) has no caller in harness-view, harness-core or harness (git grep, READ). The composition render queue builds reel jobs (`harness-view/src/view/timeline_export/render_queue.rs:361` `session_render_queue_jobs`) that only harness tests run (`harness/tests/motion_commands.rs:700`, `motion_records_session.rs:193`; `harness-view/src/view/job_admission.rs:38-40` says so). Output formats are MOV VC-3 and MOV/MP4 AV1 only (`folio/src/motion_records.rs:542`); `rgb_alpha` is refused (`render_queue.rs:203`) and post-render actions too (`:206`). This lane gives both one typed, headless, admitted queue (STU-VID-064 :25560, STU-MOT-124 :28302-28308, STU-MOT-125 :28310), user recipes, and a PNG image sequence with alpha. All file:line at 0a733db8, paths under `handshake-studio-assets/crates/hsk-studio-`.

Clauses: STU-IO-101 :21135, STU-IO-110 :21191, STU-IO-142 :21337, STU-IO-163 :21420, STU-IO-164 :21422, STU-IO-166 :21426, STU-VID-060 :24889, STU-VID-061 :24894, STU-VID-063 :25554, STU-VID-064 :25560, STU-MOT-120 :28245, STU-MOT-121 :28251, STU-MOT-122 :28279 (channels RGB, alpha, RGB+alpha; color premultiplied or straight; starting_number and use_comp_frame_number), STU-MOT-122a :28292, STU-MOT-124 :28302, STU-MOT-125 :28310.

1. **Red first, compiling.** New `harness/tests/delivery_queue.rs` (feature `media`) through the drive rows and `Viewer` methods, with records as JSON, every row refusing until built. Prove red by running.

2. **One queue (harness-view, new egui-free `src/view/delivery.rs`).** Items are either `sequence` (sequence id, recipe id, span, output path) or `composition` (render queue item id from the motion records, `StudioMotionRecords.render_queue`). One `Viewer::delivery_run` builds every item's jobs, admits them through `AdmittedExportQueue` (`job_admission.rs:226`, built by `admitted_export_queue` `timeline_export.rs:608`), runs them on one worker like `export_sequence_inner` (`export_media.rs:514-537`) and returns one receipt per item and module: recipe identity, source span, output artifact or artifacts, outcome, partial output named when canceled (STU-MOT-125 :28313-28314). Cancel one item without cancelling the rest (STU-VID-064 'one queue with per-item receipts'); test it. Settlement keeps `completed_stale` (`job_admission.rs:17-20`).
   - Sequence items resolve a `StudioExportRecipe` (shipped preset through `ExportPreset::studio_recipe`, or a user recipe) through `job_from_recipe`; the six-identifier writer binding (`export_recipe.rs:24-28`, `:83-100`) stays: any other parameter refuses `export_parameter_not_consumed` before the first frame. `export_sequence` (`export_media.rs:505`) keeps working with `preset_id` and becomes a thin call into the same path (one implementation, a pinned byte test against today's output).
   - Composition items need a product `ScenePixels` over the open document's tiles. Only test impls and the monitor's media `Frames` exist (`harness-view/src/view/monitor.rs:259`); wrap the static renderer's tile source (`harness-view/src/view/render.rs:622` `TileInfoSource for TileTable`, `:648` tile planes by `SourceKey`) keyed by the composition adapter's tile id `"{layer_id}/{object_key}"` (`composite/src/temporal/composition.rs:984`). Glyphs from `view::text::animated_glyphs` (`harness-view/src/view/text.rs:487`), audio layers from the bound sound port (`export_media.rs:447-468`). NOT_INSPECTED: whether TileTable can be borrowed for the worker's lifetime; if not, copy the planes the items name, charged.

3. **User recipes (STU-VID-063 'a user recipe is the same record as a shipped one').** A document entry `export_recipes` holding `StudioExportRecipe::to_json` texts (`export_recipe.rs:594`) keyed by `recipe_id`; reel has no JSON parser by design (`export_recipe.rs:36`), so the host parses into `set_parameter` calls (`:468`), which check every value (unknown ids, credentials, bounds). Add the token to `session/src/source.rs:176` `DOCUMENT_ENTRY_TOKENS`, validated by its producer on set and read like `raster_records`. Rows: `export.recipe_save` (from a preset or recipe id plus parameter edits, new id), `export.recipe_delete`, `export.recipes` (read: shipped and user, with does_video, does_audio). STU-IO-166 puts recipes in SurrealDB tables; this asset has none (open question 1).

4. **PNG image sequence with alpha (STU-MOT-122, motion step 84).** `StudioOutputFormat` (`folio/src/motion_records.rs:542`) gains `PngSequence` (a `settings_enum!` member; documents that do not use it stay byte-identical, an older reader refuses one that does). In `module_output` (`render_queue.rs:194-252`) a PNG module is not a reel job: frames from the shared render (`SharedFrames`, `timeline_export/composition.rs:345`) through `OutputPost::apply_rgba` (`:521`, written for exactly this) and `hsk_studio_interop_raster::png::encode_png` (`interop-raster/src/png.rs:240`; harness-view already depends on interop-raster under `viewer`, `harness-view/Cargo.toml:14`, `:15`, `:45`). `channels` rgb, alpha or rgb_alpha, `color` straight or premultiplied, depth 8 or 16, file names from `starting_number` and `use_comp_frame_number` (`motion_records.rs:598-599`, bound checked at `:1054`) with a declared zero-padded width. Each file written no-clobber like the export publish path (`reel/src/export.rs`, SMB-PR-EXPORT-RECIPE-1 hard-link publish) and named in the receipt; a cancel names the frames written. `rgb_alpha` on MOV stays refused `output_alpha_codec`: no reel encoder carries alpha (ProRes is decode-only, `reel/src/prores/SOURCE_PROVENANCE.json`).

5. **Render queue commands (STU-MOT-125).** The queue records are typed (`harness-core/src/action/commands/motion.rs:45-50`: queue_add, queue_remove, queue_set_settings, queue_add_module, queue_set_module, queue_remove_module). Add `composition.queue_reorder` and `composition.queue_duplicate` beside them in `harness-core/src/motion_commands.rs` (render_queue edits `:945-978`), and `composition.queue_apply_template` taking a stored user recipe or a named render-settings and output-module template (open question 3). Post-render actions stay refused (`render_queue.rs:206`).

6. **Typed surface and process.** RUN_COMMANDS or typed Viewer rows in `harness-core/src/action/commands/export.rs` (today `export.set_settings`, `export.run` for documents, `:16-26`): `delivery.add`, `delivery.remove`, `delivery.list`, `delivery.run`, `delivery.cancel`, `delivery.receipts`, plus the recipe rows. A CLI verb `deliver PACKAGE --item ...` beside `export-sequence` (`harness/src/cli.rs:577-612`, `execute_export_sequence` `:1282`), headless (STU-IO-164). The queue itself is per-Viewer state, not saved (open question 2).

Hostile input: a recipe JSON over the bounds of `export_recipe.rs:56-64` refused before parse completes; 4096 queue items; a PNG sequence of 4,000,000 frames refused by `ExportLimits.max_frames` (`reel/src/export.rs:162-178`) before the first frame; an output path with `..` or absolute refused (`render_queue.rs` relative-path rule, module docs `:28`); disk full mid-sequence names the last frame written; two items naming the same output path refused before any frame.

Out of scope: AAC, MP3 or any compressed audio encoder (licence question, Operator call); frame-rate conversion, scaling and reframe in sequence recipes (REEL_WRITER_BINDING equality rules); MOV alpha codec; post-render actions; Collect Files (STU-MOT-126); any audio_session.rs edit (audio mixdown and stems stay Viewer rows and may be queued later); any panel, egui or Vulkan line; any Cargo.toml change; Master Spec and MT edits.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: each item settles through the existing job admission (completed, completed_stale, canceled, failed) with one Observe terminal frame per outcome (job_admission.rs:31-33); receipts name recipe, span, artifacts, outcome and partial output; refusals are typed codes before any frame", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "the queue's jobs carry the Run's actor and a correlation minted per job by JobAdmission::next_correlation (job_admission.rs:21-27); recipe save and delete are entry-only steps with the caller's CommandContext; no default_actor() or literal actor in new code", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r53-followups-1`, branch `lane/smb-delivery-queue-1`, base eb17fad6)

- harness-view: new `src/view/delivery.rs` (queue, items, receipts, document ScenePixels), `src/view/timeline_export/render_queue.rs` (`module_output` PNG branch), `src/view/timeline_export/composition.rs` only for a PNG sink over SharedFrames, `src/view/timeline/export_media.rs` (export_sequence routed through the queue), `src/view/timeline_export.rs` (recipe resolution beside `preset_recipe`), `src/view/mod.rs` (one `mod` line and the Viewer field).
- folio: `src/motion_records.rs:542` (PngSequence) and its validation.
- session: `src/source.rs:176` token, new entry module shaped like `project_bin.rs`.
- harness-core: `src/action/commands/export.rs` (rows), `src/motion_commands.rs` and `src/action/commands/motion.rs` (queue_reorder, queue_duplicate, queue_apply_template).
- harness: `src/cli.rs` (verb), `src/cli/drive_viewer.rs` (dispatch arms), new `tests/delivery_queue.rs`.
- reel: none expected; `src/export_recipe.rs` only if a test proves a defect.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-r53-followups-1 --disk d`): step 0; clippy over harness-view, harness-core, harness, folio, session and their dependents; nextest WHOLE suites of hsk-studio-folio, hsk-studio-session, hsk-studio-reel, hsk-studio-package (rule 4b, new document entry), hsk-studio-harness-core; hsk-studio-harness tests delivery_queue, export_sequence*, render_queue*, composition_export, motion_records_session, harness_drive with features media (and os-encode only where the host has it; H.264 rows refuse typed without it); rule 4e: confirm units in .config/round-features.toml; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r53-followups-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
