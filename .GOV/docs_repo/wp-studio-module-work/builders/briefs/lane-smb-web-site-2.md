---
file_id: LANE-SMB-WEB-SITE-2
file_kind: lane_brief
updated_at: 2026-11-13T05:13:41Z
wp: WP-KERNEL-STUDIO
lane: SMB-WEB-SITE-2
lane_file: builders/lanes/hsk-studio-web-site-2.json
crates: harness-core, session, kernel-bindings
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, ONE HOLD PER PHASE, CARGO DISCIPLINE (Operator 2026-11-13))
---

<topic id="target" status="active" summary="Templates, library items and snippets on the site, change-link-site-wide, and session Action::Read for read-only verbs">

## Target

Site depth after SMB-WEB-SITE-1: templates with editable, locked and repeating regions and update propagation, library items, snippets, change-link-site-wide, and a session Action::Read for read-only verbs

Why this lane: the spec-coverage audit of 2026-11-11 (`S/research/spec-coverage-20261111-rollup.json` and the group file named below) judged every requirement against origin 87f331f9. Rows named here as MISSING or PARTIAL are your acceptance: a row moves when a model can run it through a typed command (JSONL loop of `harness/src/cli/drive.rs`) and the result reaches save and export.

Facts are READ by the wave planner at origin b57c82d1 with git grep, read-only, not run, unless marked 'planner READ at 0a733db8' (research/width-plan-20261108-*.json, older pin: re-check). Treat them as claims and check them.

Group file: research/spec-coverage-20261111-g5-web-whiteboard.json. Rows: 14.28.6 STU-WEB-091..093, 096..099 (7 MISSING), site record and synchronisation STU-WEB-113, 119 (PARTIAL). Ledger candidates: 2026-11-12T15:08:00Z session Action::Read for read-only verbs (web.site_report and similar) with the kernel-bindings mapping; 2026-11-12T15:08:00Z and 2026-11-11T11:09:18Z change-link-site-wide for case-mismatched and percent-encoded spellings; 2026-11-11T10:39:51Z diag_sink abort flake (act only if it repeats in R60 attempt 3 or in your bundled run).

Facts:
- `session/src/ports.rs:12-17` `enum Action { Edit, Save, Export }` with `as_str`; `kernel-bindings/src/wire.rs:128-130` maps each variant; `harness-core/src/web_site.rs:124` `REPORT_ACTION: Action = Action::Export` (lead Q1 reading at SMB-WEB-SITE-1). Action::Export or Save is named in 15 files (31 hits): list the exhaustive matches a new variant touches before coding.
- No template, library item or snippet record: grep repeating_table|library_item|locked_region: 0; snippet_trigger_duplicate: 0 (audit).
- SMB-WEB-CODE-1 runs in parallel in web-core and new harness-core files; this lane owns web_site.rs and commands/web.rs.
- diag_sink test: `harness-core/tests/diag_sink.rs:568` an_abort_leaves_the_lines_written_and_no_end_of_run; module `harness-core/src/diag_sink.rs`.

How to work:
1. Read the code first. Use the spec's names and cite clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Typed commands with typed parameters and typed refusals; each mutating command is one named undo step, carries the caller's attribution and survives save and reopen.
3. Commands register in the existing catalog: your domain file under `harness-core/src/action/commands/` (the per-domain `ROWS` tables that `commands.rs` concatenates into `RUN_COMMANDS`), or a `viewer.<method>` row.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 6 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files.

Out of scope: panels and drawing (except where this lane is the GUI lane), RF-01, render-gpu, any Cargo.lock change without asking the lead.

R60 attempt 3 takes the D: lock first. Until it releases, read the code, write red tests and commit locally; your first hold queues behind the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16).

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-session-1`, branch `lane/smb-web-site-2`, base b57c82d1)

hsk-studio-harness-core/src/web_site.rs; hsk-studio-harness-core/src/action/commands/web.rs; hsk-studio-session/src/ports.rs; hsk-studio-kernel-bindings/src/wire.rs; hsk-studio-harness/tests (new web site targets)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-sequence-session-1 --disk d`): step 0; session and kernel-bindings suites; harness-core suite; harness [viewer] web targets; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-sequence-session-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
