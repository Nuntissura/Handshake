---
file_id: LANE-SMB-R60-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-11-08T09:49:21Z
wp: WP-KERNEL-STUDIO
lane: SMB-R60-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r60-followups-1.json
crates: hsk-studio-harness-view, hsk-studio-harness-core, hsk-studio-reel, kumihimo-ui
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Fix the eight R60 attempt-2 probe failures in READY lanes before attempt 3: JobPort admission for four exports, page.add mint loop, Windows name normalisation for the .vtt guard, layout entry after a parented page delete, and two vacuous kumihimo-ui checks">

## Target

## Lead brief 2026-11-08 (binding)

R60 attempt 2 did not open: it stopped on the C: cap. Its validator probed the READY claims on a scratch tree at 35c50d3b and confirmed product failures by running them. Every fact below is in S/validation/round-R60.json, key `attempt_2_probes`, with the probe test, file:line, evidence, repair direction and probe log sha256. Read that section first and re-read each file:line at your base (origin 88b5500c or later). The probe test files were never committed, so write your own red tests from the probe descriptions.

Fixed elsewhere, NOT in this lane:
- SMB-WEB-HOST-1's web.export admission goes to SMB-WEB-SITE-1;
- SMB-COMMAND-SURFACE-1's photo.info.* mutating flag goes to SMB-DEVELOP-COMMANDS-1;
- SMB-MOTION-COMMANDS-1's add_at_current goes to SMB-MOTION-COMMANDS-2.

## Items (red first, each proven red by running it)

1. HIGH, class export-job-not-admitted-jobport. Admit export_document (harness-view view/doc_export.rs:844-883) through the Viewer's bound JobPort before the first byte is written, using the as_export_job pattern at view/layout_export.rs:543. Settle completed, failed and canceled. A saturated port gives a typed `overloaded` refusal with no file written, and the port is asked exactly once. Red test: a saturated port refuses; export_layout_pdf is the control.
2. HIGH, same class. Admit export.run (view/export_set.rs:468-480) the same way, before the first file, with typed overload, cancel and stale outcomes. Red test as in item 1.
3. HIGH, same class. Admit the mixdown export (view/audio_session.rs:370) and the stem export (:429) through the Viewer's JobPort before rendering. SMB-SCORE-EFFECTS-6 (C:) edits audio_session.rs: write items 3a and 3b LAST, and only after the lead tells you it has landed, then rebase. Red tests under harness --features audio-batch,audio-device, as the probe did.
4. HIGH, class page-add-mint-never-returns. page.add must return when a story's text names page-18446744073709551615 (harness-core harness/containers.rs:115-128, document_new.rs:245-254): the id minter saturates and loops. Use checked arithmetic and a typed refusal on exhaustion; the mint must never loop. Red test with a watchdog.
5. HIGH. SMB-PR-EXPORT-RECIPE-1's F1 check (reel export.rs:926): on Windows an output named "x.vtt " (trailing space, and dot or space variants that Windows strips) slips past the .vtt refusal, and with overwrite the sidecar replaces the media while the receipt says Complete. Normalise the extension the way Windows resolves names (trailing dots and spaces stripped, case-insensitive) before the comparison. Red test.
6. HIGH. SMB-LAYOUT-PERSIST-1 (harness-core harness.rs:2055): delete a page that has a parent page, save and reopen, and the layout can never be edited again. Fix the stale reference so the entry stays editable after reopen. Red test: the full sequence. Also, layout.open now adds an undo step while its row says non-mutating: make the row's mutating flag true, or make open add no step. Choose from the spec and say why.
7. MEDIUM, class acceptance-check-vacuous-ac-geom-03. kumihimo-ui checks.rs:88-99 edge_gaps can never report a gap. Compare every adjacent pair's snapped x1p with the next x0p (AC-GEOM-03), at scales 1, 1.25 and 2. Red test from the probe.
8. MEDIUM, class hug-cross-clamp-no-op. kumihimo-ui layout.rs:137: `Size::Hug => cm.min(avail_cross.max(cm))` is always cm. Clamp a Hug child's cross size to the available cross size, so AC-TEXT-01 (clipped_labels) sees the overflow. Red test from the probe. Then rerun the kumihimo-ui gallery goldens and the native-host gui suites: SMB-GUI-FRAME-1's frame must keep matching its design, so report any frame change with before and after captures.

## Not this lane

New features, other probe-free cleanups, and anything the items do not need. Report other defects you find; do not fix them.


</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: admissions settle through the JobPort with typed outcomes; new refusal codes reach the existing observe frames; no new sink", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: each command keeps its caller's CommandContext; admissions add no actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-r60-followups-1`, base 88b5500c)

crates/hsk-studio-harness-view/src/view/doc_export.rs, view/export_set.rs, view/audio_session.rs (items 3a and 3b, after SMB-SCORE-EFFECTS-6 lands); crates/hsk-studio-harness-core/src/harness/containers.rs, src/document_new.rs, src/harness.rs (layout entry and the layout.open row) and its action/commands row file for layout.open; crates/hsk-studio-reel/src/export.rs; crates/kumihimo-ui/src/checks.rs, src/layout.rs; new red-test files in each crate's tests/ and in crates/hsk-studio-harness/tests/. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`): step 0; harness-view [viewer] suite; harness-core suite; reel default suite; kumihimo-ui suite plus its gallery goldens; native-host --features document gui suites (gui_frame, gui_chrome, gui_layout, gui_compose, gui_window, gui_vulkan); harness [audio-batch,audio-device] for item 3; harness_drive_typed and the harness process tests that drive document export, export.run, page.add or layout.open, with a freshly built example; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
