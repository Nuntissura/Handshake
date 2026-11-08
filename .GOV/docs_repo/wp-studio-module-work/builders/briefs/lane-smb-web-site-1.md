---
file_id: LANE-SMB-WEB-SITE-1
file_kind: lane_brief
updated_at: 2026-11-08T08:50:53Z
wp: WP-KERNEL-STUDIO
lane: SMB-WEB-SITE-1
lane_file: builders/lanes/hsk-studio-web-site-1.json
crates: hsk-studio-harness-core (web site document set, link graph, site reports, update links on move, export of the set, site lock, job admission), hsk-studio-harness (tests)
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A local site of several pages is one link graph: typed site reports, update-links-on-move as one receipted transaction, export of the whole document set, a site lock and job admission, all as web.* commands">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/width-plan-20261108-print-web.json. Every open question below takes the planner's recommended default as a lead ruling. Its code facts were checked at 0a733db8; origin is now eb17fad6, whose new commits touch only hsk-studio-native-host and kumihimo-ui. Re-read every file:line at your base before you edit it.

- Should a missing page link become a warning in the default open-document export too, or only with documents: site? -> Both: STU-WEB-163 gives severity warning for any link in the site graph, and the ledger already records the refusal as wrong; list every test that changes.
- Is web.move_file a web.undo step? -> No: it changes files, not the open document's history; refuse while unsaved, reopen afterwards, and the receipt is the record; a reverse move restores (tested).
- Lock file name and stale-lock handling. -> .studio-site.lock in local_root, excluded from reports and export; no automatic breaking; the refusal names the holder and the manual step (delete the file after checking the holder is gone) goes in the manual.
- Which document is never an orphan: the recipe's file, index.html, or every document? -> Documents are never orphans in this slice (only non-document files are); orphan pages need an entry-point rule the spec does not state.
- Command ids web.site_report and web.move_file: no spec id is verified for them. -> Use them, following the web.* row naming of commands/web.rs, and record them for the command contract.

- Process: red first per item, then focused runs on D:. Stop at REVIEW READY for the read-only code-check and red-team review, then run ONE bundled run, then stop at BUNDLED GREEN. Lanes land one at a time on the lead's RELEASED (HANDOFF topic workflow, items 10, 11 and 16).
- Any lane that changes a Viewer command's refusal order also runs every harness process test that drives that command. This is the lesson from the 2026-11-08 trunk break at 35c50d3b.

## Target

Evidence: board-slides-web scenario gap rank 14 (site quality checks; steps 105, 108-111), step 113 (document set part of the export recipe), ledger `direction.next_lane_candidates` 2026-11-07T17:20:57Z: "one document per export ... no JobPort ... a link to a missing page fails the export closed instead of warning link_broken". All file:line below are READ at 0a733db8 unless marked; paths are under `handshake-studio-assets/crates/hsk-studio-`.

Today:
- The host holds one document: `WebHost` `harness-core/src/web_commands.rs:188` (doc, path, site, recipe, undo, redo); declared limits `:42-48`; dispatch `:299-350`; `Run` field `harness-core/src/harness.rs:459`, route `:1608`.
- Export `:549-625`: the open document plus every granted local reference. A link to another page (`<a href="thanks.html">`) is granted and copied as a raw `asset` (`:564-566`), so it is not exported as a document (not minified, not in a document set); a missing reference of any load kind, page links included, refuses `asset_missing` (`:567`). STU-WEB-163 says a link that resolves to no file is a `warning`.
- `Port::resolve` `:797-821` sees each reference's `Loads`; folder links are granted as `link:` (`:791`, `:799-801`); names checked portable (`:845`), joins never leave the root (`:810-819`).
- Engine: `parse` `web-core/src/dom.rs:384`, `collect_assets` `web-assets/src/assets.rs:1233` returning `AssetRef {node, source, reference, span, outcome, data_depth}` (`:156-167`; the span is the whole value when decoding changed offsets), `SourcePatch` `web-core/src/source_dom_patch.rs:24`, `apply` `:71`, `set_attribute` `:218`; the URL carrier table names `href` rows with `Load::Document` for navigation (`web-assets/src/carriers.rs` Load enum, `Document` documented as "navigated to (a, area, form, refresh)").
- No JobPort in harness-core's Run (git grep JobPort over harness-core src/harness.rs: 0 hits); the session JobPort is used by `batch.rs:29`, and harness-view's `job_admission.rs:47-62` holds an `AdmissionJobPort`.
- Rows: `harness-core/src/action/commands/web.rs:18-60`; tests `harness/tests/web_host.rs` (`:239` export, publish and verify; `:486` a linked folder does not reach outside the root).

Clauses (module 14, v02.229): STU-WEB-007 :30437, STU-WEB-113 :31101, STU-WEB-118 :31111, STU-WEB-120 :31115, STU-WEB-121 :31117, STU-WEB-127 :31140, STU-WEB-131 :31152 with :31153-31157 (parallel-safe, site-level advisory lock reporting the holder, deterministic), STU-WEB-139 :31179, STU-WEB-163 :31273, STU-WEB-164 :31275, STU-ASSET-009 :31352, STU-AUT-147 :21905, STU-CON-007 :22896.

### Items, red first

1. **Red, compiling.** New `harness/tests/web_site.rs`, commands by id string (the method of `web_host.rs:62` `cmd`). Fixture written by the test in a temp folder: `index.html` (links `thanks.html`, `styles/site.css`, `img/hero.png`, `missing.html`, `https://example.org`; one `img` without `alt`), `thanks.html` (no `title`), `orphan.png`, the site defined with `web.site`. Expect: `web.site_report` lists broken `missing.html` from `index.html` with its line, orphan `orphan.png`, external `https://example.org`, missing alt in `index.html`, untitled `thanks.html`; `web.export` with `documents: site` writes both pages and the manifest lists both as documents; `web.move_file` of `thanks.html` to `pages/thanks.html` rewrites the `href` in `index.html`. Prove red by running (`unknown_command` today).

2. **Document set and link graph.** New module `harness-core/src/web_site.rs`, declared from `web_commands.rs` (`mod web_site;`) so `harness.rs` is untouched. Walk `local_root` in sorted order (never enumeration order, STU-WEB-131), bounded by file count (reuse `MAX_EXPORT_FILES` `:75`), depth (`MAX_SEGMENTS` `:81`) and the existing portable-name and no-escape rules; every `.html`/`.htm` is parsed under `Limits`; the open document contributes its live source, not its file. Per document, `collect_assets` through a recording port that keeps each reference's `Loads`. Graph row: `{from, reference, span, line, load, target_id, outcome}`; line = 1 + newlines before `span.start`. A fragment `page.html#id` to a site document resolves against that document's `id` attributes (the `#id` key form exists, `web_commands.rs:984`).

3. **`web.site_report`** (new row, `web_command`; a read, no history step, one op record with counts). Rows sorted by (document, line, reference): `broken_links` severity `warning` with referring document, target and line (STU-WEB-163); `orphaned_files` severity `info` (STU-WEB-164; the site sidecar, manifest, receipt and lock file excluded; the recipe's entry document is never an orphan); `external_links` (STU-WEB-118); `missing_alt` (`img` without `alt`) and `untitled_documents` (no `title` or an empty one) from STU-WEB-121. Declared not done: redundant nested tags, removable empty tags, lint threshold, checked-out files. Bounded by `MAX_ROWS` (`:71`) with a `truncated` count.

4. **Export of the document set** (STU-WEB-120 "which document set"). `web.export_recipe` gains `documents`: `open` (default; today's output byte-identical except item 4's page-link rule) or `site`. With `site`: every document of the set is exported from its source (minified per recipe); a link to a document of the set is that document, not an asset; assets are the union; the manifest lists every document with role `document`. In both modes a page link (`Load::Document`) with no target becomes a `link_broken` warning row in the manifest and the result instead of a refusal (STU-WEB-163); any other missing local reference still refuses `asset_missing` before anything is written (STU-ASSET-009 fail closed). `web.publish` (`:627`) publishes what the manifest lists, so it follows unchanged (read `:627-690` to confirm).

5. **`web.move_file`** `{from, to}` (STU-WEB-118 update-links-on-move). Under the site lock (item 6): refuse while the open document has unsaved edits (`document_unsaved`); compute every referring document's new source by span edits of the references (attribute values through `escape_attr`; CSS `url()` spans as text edits), honouring `site_relative_link_base` (`document` relative or `site_root` absolute), and re-base the moved document's own relative references; re-parse every result before the first write; then write each through the existing atomic writer (`:909`) and rename last; on a failed write, restore already-written files from the in-memory originals and say so in the receipt. Receipt: the move plus `{document, before, after}` byte ranges per changed reference. Not a `web.undo` step: the open document, if rewritten, is reopened from disk with a fresh history (open question). Refusals: `to` outside the root, an existing `to` (case-folded, as export's collision check `:576-591`), a sidecar, manifest, receipt or lock file.

6. **Site lock** (STU-WEB-131). `web.site_report`, export with `documents: site`, `web.move_file` and `web.publish` create `<local_root>/.studio-site.lock` exclusively with `{actor, client_session, command_id}`; contention refuses `site_locked` naming the holder, never blocks; released on every exit path (a guard). No stale-lock breaking in this slice.

7. **Job admission.** `web.export`, `web.publish`, `web.site_report` and `web.move_file` are admitted through a session `JobPort` the `WebHost` holds (an `AdmissionJobPort`, as `harness-view/src/view/job_admission.rs:62` builds it): `overloaded` and `canceled` refuse before work; cancellation is checked between documents. The declared gap at `web_commands.rs:47-48` goes.

8. **Docs.** Module docs `web_commands.rs:1-48` (limits updated), new rows in `commands/web.rs` with parameters, `web_site.rs` manual section. Read MT-34270, MT-34271, MT-34274, MT-34275, MT-34346, MT-34349, MT-34348 (NOT_INSPECTED in this plan) before claiming; do not edit MT files.

Hostile input (V4-01/02), typed and changing nothing: 4097 files; a junction or symlink inside the root pointing outside; a document over `Limits`; a document with a very large number of references (bounded by collect_assets limits, NOT_INSPECTED: read them and state the number); a move onto an existing file, a case-only rename on a case-folding file system, a move out of the root; references with entities (span is the whole value); a held lock; a canceled token mid-walk and mid-move.

Determinism: report and export outputs are pure functions of the files and the recipe (STU-WEB-131): same tree, same bytes, whatever the enumeration order; no time in any output except the publish receipt's elapsed time, as today.

Out of scope: rendering and `web.capture_view`; validator and linters (STU-WEB-052..058); templates, library items, behaviours; change-link-site-wide (next slice); synchronise (STU-WEB-119); transports other than `local_or_network`; version control; source maps, preprocessors, asset rewriting other than `keep`; site records in SurrealDB tables (STU-WEB-007; the sidecar stays); any panel, egui line, `Cargo.toml` change, Master Spec edit or MT edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: one op record per command with a typed error and bounded target (web_commands.rs:1289-1320 run and target); report rows and receipts in the record's detail; the terminal Observe frame stays the run's own", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: every command runs under Run::begin_at with the caller's command context (web_commands.rs:1299); the lock file records that context's actor and client session; no default_actor() or literal actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-sentinels-native-1`, branch `lane/smb-web-site-1`, base eb17fad6)

- harness-core: new `src/web_site.rs` (declared from web_commands.rs); `src/web_commands.rs` (module docs :1-48, WebHost :188, execute arms near :299-350, export :549-625, recipe_from :1202-1235, a recording Port beside :783-821, the job port field); `src/action/commands/web.rs` (rows web.site_report, web.move_file; documents parameter on web.export_recipe).
- harness: new `tests/web_site.rs`; `tests/web_host.rs` only if the page-link rule flips an existing assertion, reported in the notes.
- web, web-core, web-assets: none expected; if a carrier gap is proven by a test, ask first.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-sentinels-native-1 --disk d`): step 0; clippy over hsk-studio-harness-core and hsk-studio-harness; nextest WHOLE suite of hsk-studio-harness-core; hsk-studio-harness tests web_site, web_host, harness_drive and the catalog key tests (rule 4e: confirm the units in .config/round-features.toml); web crate suites only if edited; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-sentinels-native-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
