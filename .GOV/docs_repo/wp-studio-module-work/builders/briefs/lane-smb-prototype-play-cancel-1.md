---
file_id: LANE-SMB-PROTOTYPE-PLAY-CANCEL-1
file_kind: lane_brief
updated_at: 2026-11-13T17:08:44Z
wp: WP-KERNEL-STUDIO
lane: SMB-PROTOTYPE-PLAY-CANCEL-1
lane_file: builders/lanes/hsk-studio-prototype-play-cancel-1.json
crates: harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A prototype's open and render run under the caller's cancel token, so a cancel during a capture's render is honored">

## Target

A prototype's open and render run under the caller's cancel token, so a cancel raised during a capture's render is honored (hsk-studio-harness-view view/prototype.rs)

Why this lane: ledger candidate 2026-11-13T15:08:59Z, verbatim: "prototype play renders under the viewer's cancel token (PrototypePlay::render uses its own token), so a cancel raised during a capture's render is honored (SMB-FIX-CAPTURE-ADMISSION-1 not_proven)". A cancel that is not seen during a render is a HOST INTEGRATION (6) gap: every job is admitted with typed cancellation (HBR-HOST-004).

Facts are READ by the wave-next-2 planner at origin 89f27330 (read-only, not run; `git diff a77c4ee4 89f27330` touches only harness-view src/view/menu_tree.rs). Treat them as claims and check them.
- Spec: STU-PRO-138 (module 14 :20746, prototype.capture_frame among the required commands).
- `harness-view/src/view/prototype.rs:520` PrototypePlay holds its own `cancel: CancellationToken`; `open` (`:525-532`) and `open_document` (`:539-542`) create `CancellationToken::default()` for import_fig, decode_document and to_native and store it (`:610`). frame_raster (`:758`, `:775-776`), smart_raster and smart_raster_mode (`:784-817`), directional_raster (`:825`) and render (`:845`) run export_frame_pdf, render_frame_export and to_native under `&self.cancel`.
- `view/commands/prototype.rs:74-131` capture_frame and render_and_write: the viewer's token (`v.cancel`, `:103`) is checked after the render and before the write (`:129`); the doc at `:112-113` says a cancel during the render itself is not seen. Test helper with cancel_on_admit at `:227-239`.
- `view/mod.rs:434` Viewer::open_prototype builds the play; view/mod.rs belongs to SMB-FIX-LAYER-RECORD-PRUNE-1 (read only here).
- `prototype.rs:906` ui(&egui::Context) is quarantined egui test-host code: do not touch it.

Red test (must fail first):
A test (harness [viewer] tests/prototype_play_cancel.rs, new, or harness-view [viewer]) that cancels the viewer's token after admission and before the render: the capture settles `canceled` with the refusal coming from the render stage (not the pre-write check), and nothing is written; and a canceled token makes the play's render refuse `canceled` before rendering. It fails at the base.

Fix contract:
- The play's renders take the caller's token (passed per call, or set from commands/prototype.rs before the render); open and open_document import under the caller's token where a caller has one.
- If threading the token needs view/mod.rs (open_prototype), ask the lead first, naming the line.
- No new egui line. 1 to 3 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-ai-distort-effects-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-harness --features viewer --test prototype_play_cancel --test fix_capture_admission --test viewer_prototype` (or the harness-view [viewer] test binary you add) and `cargo nextest run -p hsk-studio-harness-view --features viewer --lib` for the commands/prototype.rs unit tests. Bundled: the harness-view [viewer] suite plus the harness prototype binaries (git grep PrototypePlay and prototype.capture_frame in crates/hsk-studio-harness/tests; send the list with REVIEW READY).
MENU COVERAGE: this lane adds or renames no catalog command; say so in the reports.

Out of scope: the shared derived-resource helper (candidate 2026-11-13T15:08:59Z), prototype GUI, the .fig importers' own cancel granularity, harness-view view/mod.rs and view/commands.rs without asking, any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PROTOTYPE-PLAY-CANCEL-1, 2 SMB-PR-TIMELINE-FIXES-1, 3 SMB-MOTION-IMPLIED-VALUES-1, 4 SMB-GUI-GESTURE-GATE-1; the lead may interleave SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 1 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): STUDIO-R60 attempt 4 runs on the D: round slot and holds the D: lock (class round, FIFO priority) for many hours. Your lock-needing steps queue behind it: that is expected. Start the lock-needing step in the background and wait to be notified. Never work around the lock, never poll in a loop, and never run cargo outside a hold. Read code and edit while you wait. ONE HOLD PER PHASE (wave-next-2, 2026-11-13): the four lanes share the D: lock with each other and with the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Lanes land one at a time on the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the capture's existing job settlement (canceled at the render stage), codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the capture keeps the caller's context; no new mutating op", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-prototype-play-cancel-1`, base 89f27330)

hsk-studio-harness-view/src/view/prototype.rs (token plumbing, not ui()); hsk-studio-harness-view/src/view/commands/prototype.rs; hsk-studio-harness/tests/prototype_play_cancel.rs (new, or a harness-view [viewer] test)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-ai-distort-effects-1 --disk d`): step 0; harness-view [viewer] suite; harness [viewer] --test prototype_play_cancel and the prototype test binaries (list them with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ai-distort-effects-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
