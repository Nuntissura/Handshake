---
file_id: LANE-SMB-LAYOUT-COMPOSER-DEPTH-1
file_kind: lane_brief
updated_at: 2026-11-05T14:24:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-COMPOSER-DEPTH-1
lane_file: builders/lanes/hsk-studio-layout-composer-depth-1.json
crates: [hsk-studio-layout (Adobe paragraph composer)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Document data never aborts the paragraph composer; a large paragraph answers in bounded time or refuses typed">

## Target

STUDIO-R56 finding F-R56-1 (HIGH): read it in `S/validation/round-R56.json`, evidence PROBE-R56-BREAKS-1. `crates/hsk-studio-layout/src/adobe_composer.rs` `Para::best` recurses once per line, and `paragraph()` sets the window to the whole paragraph on ragged text (`PARA_MODEL.ragged_depth` 0, so depth is items + 1). On a 2 MiB test thread, debug build, `H LS*2000 I`, `(a LS)*2000` and plain wrapped text of 3000 words all end in STATUS_STACK_OVERFLOW. `Limits.max_lines` does not guard it, because the recursion runs before any line is counted. The cost is steep too: 600 plain words took 31 s and 1800 words exceeded the 120 s test cap (debug; release not measured). It is reachable from document data: a paragraph style with `composition_scope` `every_line` selects this composer (`typeset.rs:258-259`, `:317`).

1. **Red first.** Tests for those three shapes on a stated bounded thread, proven red by running. An abort takes its whole test binary down, so give these their own test file. A stack overflow counts as that test failing.
2. **No call-stack depth from document data.** Compose without recursing per line: an explicit stack or a bottom-up pass over the same states. The output must not change. `best` keeps the first candidate whose total beats the current one by more than `1e-12`, in `lines_from` order, so the new form keeps that order and that epsilon. Proof: every existing composer fixture and oracle row passes with no blessed value changed, plus a differential table over generated short paragraphs (justified window 7 and ragged whole paragraph, with hyphen runs and forced breaks) against bytes captured from the base sha or a test-only copy of the recursive form. Say which.
3. **Budget.** Read what the `work` counter bounds and why it did not stop 1800 words inside 120 s. Declare a bound so that a very large paragraph answers a typed refusal in bounded time, using an existing `LayoutError` code where one fits; cancellation stays honoured inside the pass. Removing a redundant memo key dimension is allowed if output is identical (read whether `line` and `d` are both needed; do not assume). Do NOT change `PARA_MODEL` values: they are fitted, and a finite ragged window changes output. If you conclude the window must be bounded, stop and send me the numbers. Measure debug and release at 600, 1800, 5000 and 20000 words and record them, with the time to reach the refusal.
4. **Failure-class sweep (V4-06).** Every self-recursive function over document-sized data in layout, and read-only in type: list each site with its bound, fixed or not applicable with a reason. One bounded-count test per composer in layout (first-fit, Knuth-Plass, single-line, every-line, CJK) at a count past its budget: a typed answer, no abort.
5. **Host reach, read only.** R56 reads the native fixture host at a 4096-byte Type budget and the Viewer at 8192, and marks other hosts NOT_INSPECTED. State which hosts can reach this composer with a larger budget.
6. **MTs.** MT-30904 anchors STU-TYP-135. This lane is a correction: claim a row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): the point of this lane; no new unbounded allocation; memo size bounded by the declared budget. Vendor parity UNVERIFIED (D5); do not re-bless any parity row.

Out of scope: `toc::line_of` (another lane); changing composer output or fitted values; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: layout returns typed refusals and counters, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composition commits nothing", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-layout-composer-depth-1`, branch `lane/smb-layout-composer-depth-1`, base 091c4d1b)

- layout: `src/adobe_composer.rs`, any other composer the sweep proves wrong, `lib.rs` DESCRIPTOR, `tests/`.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-layout-composer-depth-1 --disk d`): step 0; clippy over layout and its direct dependents; nextest WHOLE suites of layout and interop-layout; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-layout-composer-depth-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
