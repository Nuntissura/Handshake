---
file_id: WP-STUDIO-MODULE-WORK-WORKFLOW-REVIEW-001
file_kind: workflow_review
updated_at: 2026-10-11T01:20:00Z
wp: WP-KERNEL-STUDIO
assignment: STUDIO-MODULE-BUILD
author: workflow agent (Opus, adversarial)
reviews: START_LOG.md#workflow-v1, builders/DISPATCH_TEMPLATES.md v1
proposes: workflow/workflow.json version 2 (adopted:false)
---

# Workflow review 001 — previous Studio workflow metrics, v1 failure modes, v2 proposal

Prose surface (context, not authority). Every number names its source; anything not measured is labelled UNVERIFIED.

<topic id="summary" status="active" summary="Findings in 8 lines">

## Summary

1. The previous run spent 13.3 h of governance-only preparation (35 kernel commits, ~21.9 M inserted lines, 6,223 MT files / 1.3 GB) before the first Studio product line, then 9.5 h building 8 crates serially at ~1 crate/h.
2. Validators were fast (median ~6.5 min submit -> verdict). The bottleneck was serial lanes plus growing preparation gaps between crates (10 -> 130 min).
3. Type's 3-strike fix budget burned in 13 min because builders were limited to `cargo check` (CX-EXEC-014 / KB-CAD-001) and every runtime defect cost a verdict. The test also stopped at the first failing case, so each verdict showed only one defect.
4. 46 % of all verdicts (12/26) were re-proofs of MTs that had already passed, mostly the workspace MT re-proved each time a crate landed.
5. Of the 52 lessons in the old gameplan that are not presets, 59 % came from tooling, hooks, encoding or validator harness bugs. Only 19 % were product-semantic defects.
6. Workflow v1 fixes seriality but brings in new shared-state risks: one worktree, one `Cargo.lock`, one target, three lane-state stores, and a lead session whose loss stops every agent.
7. The v2 changes below are minimal and can be adopted today. Three need an Operator decision: builder tests as an override of CX-EXEC-014, the CX-984-002 one-build-per-disk rule during rounds, and the hook matcher.

</topic>

<topic id="previous-workflow-metrics" status="active" summary="Quantified previous workflow with sources">

## Previous workflow metrics (2026-10-09 19:41 -> 2026-10-10 18:54, local +0200)

| Metric | Value | Source |
|---|---|---|
| Product commits, Studio path | 33 (the other 14 of the 47 in the repo are Handpick) | `git -C wt-studio-assets log -- handshake-studio-assets` |
| Product span | 09:18:30 -> 18:47:52 = 9 h 29 min | same |
| Product commits/hour | 3.48 | 33 / 9.49 h |
| Crate-landing commits | 9 (workspace + 8 crates); 24/33 (73 %) were pins, fixes, lint or diagnostics | commit subjects |
| Crate landing rate | ~0.95 crate/h, strictly serial | same |
| Product insertions | 85,937 (29,477 of them vendored rustybuzz in c0b4c9a) | `--shortstat`, `--numstat c0b4c9a` |
| Kernel WP commits, prep window (10-09 19:00 -> 10-10 09:00) | 35, 0 product commits in parallel; 12,830 file changes, +21.9 M / -3.4 M lines | `git log --since/--until --shortstat -- .GOV/task_packets/WP-KERNEL-STUDIO` |
| Kernel WP commits, build window (10-10 09:00 -> 19:00) | 54; +16,643 / -14,088 lines | same |
| Governance:product commit ratio, build window | 54:33 = 1.64 | above |
| Wall time that was governance-only | 13.3 h of 23.2 h = 57 % | above |
| WP folder size | 6,223 files, 1.3 GB; a single `git log --shortstat` on it ran > 120 s | `ls | wc -l`, `du -sh`, background task bmdy0b9ai |
| Verdicts | 26 across 9 MTs: 20 PASS, 6 FAIL | MT-35528..35536 `verdicts[]` |
| Re-PASS of already-passed MTs | 12/26 = 46 % (MT-35528 workspace alone: 8 verdicts) | same |
| FAIL causes | 6/6 product (Chronicle contract, Pigment allocation lifetime, workspace HELP missing Nib, Type x3); 0 infrastructure verdicts | `INDEPENDENT_FINDINGS` |
| Submit -> verdict latency | workspace 4.8, Accord 15.8, Observe 9.4, Type 8.2 / 3.4 / 3.6 min (median ~6.5) | kernel "Submit" commit time vs verdict `observed_at` |
| Gap from previous crate PASS to next crate's first product commit | Accord 10, Observe 21, Folio 29, Prism 10, Pigment 29, Nib 77, Type 130 min | kernel vs product timestamps |
| Type fix budget | FAIL_V1 16:39:44Z -> FAIL_V3 16:52:47Z = 13 min for 3 strikes; each submission was `cargo check` only (`COMPILE_STATIC_PASS_RUNTIME_NOT_RUN`) | MT-35536 `validation.proof_records` |
| Type failure discovery | Serial: sentinel bug -> RLI script resolution -> provider entry guards (`ot_shape_plan.rs:57-63`, `shape.rs:54-64`, `shape.rs:18-26`). FAIL_V2 and FAIL_V3 had the same finding | MT-35536 `implementation_state.remediation`, `operator_decision_request` |
| Validator test shape | 1 filter executed, `filtered_out:1`; second filter `font_axis_missing_cancel_stale` never run; case loop panics at `support/mod.rs:922` on the first failure | MT-35536 proof records `not_run`, `COUNTERFACTUAL_CHECKS` |
| Old WP gameplan | 66 steps: 14 preset/policy, 24 tooling/hook/shell/encoding (46 % of the 52 lessons), 7 validator oracle/harness, 11 compile/lint, 10 product-semantic (19 %) | `gameplan.py show` on the WP gameplan, causes classified by step source |
| Bespoke validator/builder scripts | 53 `.py` across MT owner dirs (MT-35536 validator alone: 15) | `find Handshake_Artifacts/WP-KERNEL-STUDIO/MT-355* -name '*.py'` |
| Isolated cold targets | 17 `cargo-target` dirs (one per MT per role) plus per-MT `CARGO_HOME` | `ls Handshake_Artifacts/WP-KERNEL-STUDIO/MT-355*/*` |
| Warm build cost | `check` 0.57-3.41 s, test build 1.68-11.03 s | `type-check-0*.stderr.log`, validator logs `Finished` lines |

## Current session so far (00:37Z -> 01:15Z)

- Product HEAD is still `55c1096`, with 0 product commits (`git log -5`, `git status` clean).
- The lead session was lost at ~00:50Z and all 10 agents stopped after ~8 min. Recovery tick at 01:05Z reads `"progress": "no direct progress"` (`journal/tick-log.jsonl`).
- Governance written before the first product line: `START_LOG.md` 12.2 -> 15.1 KB, dev-loop gameplan at generation 24 with 15/17 steps retired, `ledger.json` 9.3 KB (29 lanes, 27 empty), `team.json`, `DISPATCH_TEMPLATES.md`, `studio-env.sh`.
- `DISPATCH_TEMPLATES.md` references `builders/WORKSPACE_RULES.md`, `research/existing-api-digest.md` and `builders/lanes/`. None of them exists at 01:15Z (`find .`). A builder dispatched now would read missing files.

</topic>

<topic id="failure-modes" status="active" summary="Where v1 fails next; mitigation and cheapest verification each">

## Failure modes of workflow v1 (ranked by expected time cost)

### F1. Runtime proof only at the verdict, so the fix budget burns (observed)
- Evidence: Type, as quantified above. CX-EXEC-014 says implementers run compile and static checks only, and KB-CAD-001 says the same. START_LOG v1 says builders run focused tests, and the builder template says "focused tests (≤5)". So the repo rule and v1 conflict. The Opus Type lane reads the MT, Codex and KB protocol, and may follow CX-EXEC-014 again.
- Mitigation: the Operator's "build it, break it, fix it" supersedes CX-EXEC-014 for builders (CX-021). The lead records this once in the START_LOG decisions.
  - READY requires the brief's tests to be green on the pushed SHA.
  - Resubmitting after a FAIL requires the validator's exact failing argv to be green in the lane's own run first.
  - Builder results are implementation evidence only (CX-PROOF-002 unchanged).
- Type, now: before spending the single approved round, the lane runs both owning filters `mixed_script_bidi_font_runs` and `font_axis_missing_cancel_stale` (per `source_validation_filters`). It also runs the affected Observe/workspace tests. All must be green. There is no fifth chance.
- Cheapest verification: the lane file records argv, exit code and SHA for each READY. Each round, count FAIL verdicts on a test the builder had shown green on the same SHA. The target is 0.

### F2. Fail-fast oracles show one defect per run (observed)
- Evidence: the `support/mod.rs:922` panic stopped the case loop. FAIL_V1 -> V2 -> V3 each surfaced the next gate.
- Mitigation: table-driven tests run every case, collect the failures, and assert once at the end with every failing case listed. Add this to the brief template's test section.
- Cheapest verification: the next red run lists more than one failing case when more than one is broken. Read it from the test stdout.

### F3. Shared worktree with parallel builders
- (a) A broken manifest in one member stops every builder. Cargo loads all workspace members before applying `-p` (cargo `tests/testsuite/member_errors.rs`; amethyst#2723). The steward adding 27 members while builders run, or one builder half-writing its `Cargo.toml`, therefore stops the whole team.
  - Mitigation: no builder starts until the steward's single scaffold commit is pushed and `cargo metadata --offline` is green. Builders write their `Cargo.toml` in one Write call. When a manifest error is in another lane, report it and do not fix it.
  - Verify: the steward's report includes the metadata exit code and SHA.
- (b) A half-edited upstream crate breaks downstream checks.
  - Allowed edges computed from root `[workspace.metadata.studio.dependencies]` give three waves:
    - Wave 1 (17 crates; depend only on the 8 existing crates): composite, controls, develop, interop-lrcat, interop-media, interop-psd, interop-vector, layout, motion, package, press, pulse, reel, render-cpu, render-gpu, score, web.
    - Wave 2 (7 crates): components, interop-layout, motion-js, reel-native, score-device, score-plugin, session.
    - Wave 3 (3 crates): harness, interop-fig, kernel-bindings.
  - composite may depend on type, which is under active repair.
  - Mitigation: start wave 2 and wave 3 lanes only after their upstream lanes have a pushed compile-green commit. Upstream lanes commit each green state and never leave a broken public API at the end of a turn; this applies to the Type lane too. A downstream builder that hits an upstream compile error waits and retries; it does not fix.
  - Verify: tick check that no lane commits touch another lane's crate.
- (c) Git races.
  - `index.lock` collisions are handled by the template's 5 s retry.
  - Worse: one `git add -A` or `git commit -a` sweeps other lanes' half-edits into a commit.
  - Mitigation: stage only `crates/hsk-studio-{crate}/` (CX-GIT-002).
  - Verify per tick: `git log --since=<last tick> --name-only` shows every path inside the committing lane's crate. Only the steward commits root files.
- (d) `Cargo.lock` churn. There is one `Cargo.lock`, and every cargo run rewrites it from all current member manifests, including other lanes' uncommitted edits. A builder that commits it can push a lock that does not match the committed manifests. A builder that never commits it can push a tree that is missing new entries. Either way the round's `--locked` build fails for a non-product reason.
  - Mitigation: `Cargo.lock` and root `Cargo.toml` are steward-only, and builders never stage them. The steward makes a "lock sync" commit before each round, once `git status` shows no uncommitted `crates/*/Cargo.toml`. The validator classifies a failure at the lock stage only as infrastructure (no status change).
  - Verify: the round log shows `--locked` passing the resolve stage.
- (e) Workspace coupling. MT-35528 FAIL_V7 ("selection HELP lists Folio/Chronicle/Pigment/Prism only") and 8 workspace verdicts show the workspace descriptor/probe is edited or re-proved per crate.
  - Mitigation: the steward makes the probe's crate list data-driven, from `cargo metadata` or `declared-members`, once. Whether the probe is hand-listed today is UNVERIFIED; the steward checks `workspace-probe.rs`.
  - Verify: adding a crate needs 0 probe edits.

### F4. One shared target under CX-984-002
- Evidence: warm `check` takes 0.6-3.4 s and test builds 1.7-11 s, so lock serialization is cheap while the target is warm.
- Real risks:
  - (i) A long or hanging test. It is UNVERIFIED whether `cargo test` holds the build-dir lock during test execution. If it does, one hang stalls all builders. Per-test timeouts need cargo-nextest, which is still installing (`studio-team-dev/tool-install/install-d.log` was compiling `nextest-runner` at 01:00Z).
  - (ii) An orphaned cargo or rustc from an agent killed during lead loss keeps holding the lock. A restarted lead may not stop it without `PROCESS_STOP_APPROVED:<PIDs>` (GLOBAL-INSPECT-014).
  - (iii) The validator round builds `studio-round/target` while builders build `studio-team-dev/target`. That is two builds on D: and violates CX-984-002 "Run at most one build per physical disk at a time". C: is excluded by the Operator.
- Mitigation:
  - Builders record `Blocking waiting for file lock` occurrences and wait seconds in their lane file.
  - Builders and rounds use nextest with `slow-timeout`/`terminate-after` once installed (CX-EXEC-005).
  - Lane files record the PID of every background cargo run.
  - The lead asks the Operator to choose between waiving the one-build-per-disk limit for this session, which allows parallel rounds plus optional per-lane `STUDIO_OWNER=<crate>` targets, and queueing rounds behind builders.
  - Fine-grained build-dir locking is not confirmed stable (cargo PR #16155 status unknown). Do not plan on it.
- Cheapest verification:
  - In the first hour, one builder runs `check` while another's test binary is executing, and logs whether it blocked. This settles (i).
  - Summed lock wait per tick against builder wall time.

### F5. Sonnet on engine code
- Evidence: even the previous single strong builder (model UNVERIFIED) produced 10 product-semantic defects caught only by independent review (allocation lifetimes, cancellation, bidi) and 11 compile/lint loops. With numerics, colour and blending, wrong code can pass self-authored tests.
- Mitigation:
  - Opus for engine/numeric lanes: composite, render-cpu, render-gpu, develop, interop-psd, motion, reel (type is already Opus).
  - Sonnet for schema and plumbing lanes.
  - Every brief carries at least 1 independent numeric oracle the builder may not edit. The pulse brief already does: MT-35540, 25 fps, 20,321,280,000 ticks.
  - The validator diff-reads for stubs (template step 4).
- Cheapest verification: first-round FAIL rate per model. If Sonnet lanes fail at 2x or more the Opus rate, switch those lanes.

### F6. Briefs on the critical path
- Evidence: the pulse brief (11.4 KB) is good: verified APIs, an exact fixture, ≤5 tests, non-goals, and the foundation MT `MT-35540`. But only 1 of 27 exists after ~30 min from 2 readers. Total brief time for 27 at this rate is UNVERIFIED, but the pipeline is serial per reader.
- Mitigation:
  - Stream: dispatch each builder as soon as its brief lands, in wave-1 order.
  - Cap briefs at ~5 KB using the pulse sections (identity/edges, reuse APIs, slice, oracle, ≤5 tests, non-goals).
  - Low-risk lanes may get a short brief: identity, allowed deps, oracle and foundation MT ref, with the builder reading the spec anchors itself.
- Cheapest verification: per tick, time from brief written to the builder's first pushed commit, and the number of builders idle for lack of a brief.

### F7. Validator throughput and harness bloat
- Evidence: latency was not the bottleneck (median ~6.5 min). Harness bloat was: 53 bespoke scripts, 7 oracle/harness bugs fixed inside validator code, and 12 re-PASS verdicts.
- Mitigation:
  - Continuous union rounds: start when at least 1 lane is READY and no round is running. Freeze one SHA; later commits queue (CX-VAL-001).
  - One persistent warm `studio-round/target` refreshed in place.
  - One reusable round script written in round 1; no per-verdict scripts.
  - One validator session reused across lane rounds. It never builds judged code. Fresh context is reserved for the whole-WP integration verdict (CX-PROOF-005).
  - Re-verdict a passed lane only when CX-VAL-002 fails, that is when its input paths intersect the diff.
- Cheapest verification: per round, time READY to verdict, script count in `studio-round/`, and re-PASS count (target 0).

### F8. Three lane-state stores drift
- Evidence: `ledger.json` lanes (lead), `builders/lanes/*.json` (builders, per the template), and MT JSON `verdicts` (CX-EXEC-008). START_LOG v1 says "mark lane READY in ledger"; the template says lane file. 6,195 MTs are PENDING on SPEC207 publication, and the ledger maps only Type to an MT.
- Mitigation:
  - The lane file is the single lane-status source.
  - `ledger.json` becomes a lead index: lane -> lane file + foundation MT ref (taken from the brief, e.g. pulse -> MT-35540), mirrored at tick.
  - At PASS/FAIL only, the lead appends one verdict to that foundation MT.
  - The other MTs stay untouched and are recorded once as out of scope for this session.
- Cheapest verification: tick diff of ledger status against lane files gives 0 mismatches.

### F9. Governance overhead creeping back (already visible)
- Evidence: the previous run's 1.64 ratio, 66-step gameplan and 53 scripts. In this session, 6 governance files and 24 gameplan generations exist before any product commit.
- Mitigation: governance writes happen only on a lane status change, a verdict, or a tick line. At most 1 kernel commit per lead tick (batched). No new governance file or gameplan step without a named observed failure it prevents. Compiler/clippy errors never become gameplan steps (11 old steps did; the compiler is the check).
- Cheapest verification: per interval, kernel commits divided by product commits, with a target of 0.5 or less. Gameplan step count is flat.

### F10. Agent context drift
- Evidence: the Type lane carries a 171 KB MT JSON plus 61.8 k LOC. KB-LANES-002 keeps one context per batch.
- Mitigation: keep the context within a slice, and start a fresh agent per slice after its verdict, resumed from the lane file. Replace an agent with no pushed commit for 45 min (CX-EXEC-010 says 30; 45 accounts for the 15 min tick). Replace an agent that repeats the same error twice.
- Cheapest verification: the age of the lane's last commit at each tick.

### F11. Lead-session loss stops everything (observed 00:50Z)
- Evidence: all 10 agents were children of the lead session and stopped after ~8 min. Recovery took about 15 min with no direct progress. Crons and transcripts survived.
- Gaps:
  - No `resume` template exists in `DISPATCH_TEMPLATES.md`.
  - Lane files do not exist yet.
  - Nothing checks for a stale `index.lock` left by a killed `git commit`, which blocks all git use in the worktree. Its location is under the main repo's `.git/worktrees/<name>/`; UNVERIFIED for this repo layout.
  - Nothing checks for orphaned cargo holding the build lock (F4 ii).
  - Uncommitted half-edits from killed builders sit in the shared tree.
- Mitigation:
  - Agents write their lane or state file at every commit and before any operation over 2 min, including the PID.
  - Add `resume` and `lead-restart` topics to `DISPATCH_TEMPLATES.md` (text in workflow.json `rules`).
  - Lead restart checklist: read `team.json`; list stale `index.lock` and orphan cargo/rustc with PIDs and report them (removal or stop needs Operator approval for processes); then SendMessage-resume each running agent by `agent_id`; then write one tick line.
  - A resumed builder runs `git status -- crates/hsk-studio-{crate}` first. Uncommitted edits in its own crate are its own: compile, commit if green, else continue.
- Cheapest verification: after the next restart, minutes from restart to the first resumed agent output, with a target under 5.

### F12. Gameplan hook friction (observed twice)
- Evidence:
  - The hook matches gated words anywhere in the text and ignores inline `GAMEPLAN_ROLE`.
  - The pointer resolves from the leading `cd`. From the kernel root, it picks the WP-KERNEL-012 gameplan, whose binding is broken.
  - A commit message such as `-m "add cargo test for X"` or an `echo` containing a gated phrase will block a builder.
- Mitigation:
  - Builder template rule: no gated phrases in commit messages, echo text or heredocs. Write such text with the Write tool.
  - On a block, run the gameplan check and fix the failing step. Never rephrase to get past the hook.
  - The root-cause fix (tokenized command-position matching) is in the global hook, which the Operator owns. Propose it to the Operator; do not edit it.
- Cheapest verification: count hook blocks reported in lane blockers per tick.

### F13. Black-box Adobe oracle depends on the Operator being present
- Evidence: foreground windows need Operator approval (START_LOG decisions; CX-SAFE-002).
- Mitigation: no lane is gated on Adobe exports. Use free oracles first:
  - spec numeric tables;
  - PSD merged composite: when Maximize Compatibility is on, the Image Data section holds Photoshop's own flattened render. Check `hasRealMergedData`, because otherwise it is white or a placeholder (LoC FDD, Adobe forum, GIMP 2018 commit). The Craft complaint that PSDs look different from Adobe is directly testable this way.
  - Batch all Adobe exports into one Operator-attended session prepared in advance (input corpus, script, expected outputs).
- Cheapest verification: lanes blocked on "needs Adobe" equals 0.

### F14. Cost
- Evidence: 10 agents run concurrently, including 4 Sonnet researchers whose notes no brief consumes yet.
- Mitigation: researchers end after their note. Research is re-spawned on demand from a builder blocker or diagnosis. A note not cited by any brief or diagnosis within 2 ticks is not refreshed.
- Cheapest verification: count note citations.

</topic>

<topic id="workflow-v2" status="active" version="2" summary="Minimal adopt-today changes">

## Workflow v2 proposal (adopt today; machine form in `workflow/workflow.json`)

Changes needing an Operator decision (the lead asks once, in one message):
- D1. Builders run focused tests and the validator's failing argv, overriding CX-EXEC-014 for this session under CX-021. Operator message 1 ("build it, break it ... fix it") already implies it; record it explicitly.
- D2. CX-984-002 one build per physical disk: waive it for rounds running beside builders on D:, or rounds queue behind builders.
- D3. Global gameplan hook: match gated words at command position only. This is an Operator-owned tool and stays a proposal.

Adopt now (lead authority):
1. Scaffold first: no builder before the steward's single pushed scaffold commit with `cargo metadata --offline` green.
2. Waves: wave 1 (17) streams as briefs land; wave 2 and 3 lanes start when their upstream lanes have a pushed compile-green commit; every lane keeps compile-green at the end of each turn.
3. READY requires the brief's tests green on the pushed SHA. Resubmission requires the failing argv green first. Type spends its single round only after both owning filters and the affected Observe/workspace tests are green locally.
4. Tests aggregate all case failures per run.
5. `Cargo.lock` and root `Cargo.toml` are steward-only. A lock-sync commit precedes each round; a lock-only failure is infrastructure.
6. One state store per lane: `builders/lanes/<crate>.json`, written at every commit and before long operations (with PID). `ledger.json` is the lead index. At verdict time, one verdict goes into the lane's foundation MT.
7. Continuous union rounds on a warm persistent round target, with one reusable round script and one reused validator session. Re-verdict a passed lane only when the CX-VAL-002 intersection is non-empty.
8. Model routing: Opus for engine/numeric lanes, Sonnet for plumbing. Every brief carries at least 1 independent oracle; free oracles before Adobe.
9. Lead-loss recovery: `resume` and `lead-restart` templates, with checks for stale `index.lock` and orphan PIDs.
10. Governance budget: at most 1 kernel commit per tick; no new governance file or gameplan step without a named observed failure; compiler errors never become gameplan steps.
11. Deadlines: no pushed commit for 30 min gets a nudge; 45 min gets a fresh agent from the lane file. Fresh agent per slice.
12. Researchers end after their note; research is spawned on demand.

</topic>

<topic id="offset-tick" status="active" summary="Workflow agent tick and checks">

## Offset tick

Keep the +7 min offset, cron `11,26,41,56`. The lead ticks at :04/:19/:34/:49, so each workflow tick sees the lead's last tick line and decisions. The lead then reads the review at its next tick, 8 min later. Proposal: after 2 consecutive workflow ticks with no change proposed, drop to `11,41` to save cost. Return to 15 min on any red round or lead restart. The lead decides.

Each tick, bounded and read-only:
1. Product: `git log --since=<prev tick> --format='%h %ci %s' --name-only` gives commits per lane, and flags any path outside the committing lane or any root `Cargo.toml`/`Cargo.lock` change by a non-steward.
2. Lanes: lane-file age, no commit for 30 min or more, the same blocker error seen twice or more, and lock-wait seconds.
3. Ledger against lane files: status mismatches.
4. Rounds: time from READY to verdict, FAIL cause class, re-PASS count, new scripts in `studio-round/`.
5. Governance: kernel commits against product commits in the interval, and gameplan step-count delta.
6. Hazards: stale `index.lock` presence, cargo/rustc processes on D: (count and PIDs only, never stopped), hook blocks reported.
7. Output: 25 lines or fewer appended to `workflow/workflow-review-ticks.md`, covering what slowed the team, what repeated, and 1-2 concrete changes.

</topic>

<topic id="sources" status="active" summary="External sources used">

## External sources (checked 2026-10-11)

- Cargo coarse build-dir lock; fine-grained locking not confirmed stable: https://rust-lang.github.io/goals/2025h2/cargo-build-dir-layout.html , https://github.com/rust-lang/cargo/pull/16155
- Workspace member manifest errors stop every command: https://github.com/amethyst/amethyst/issues/2723 , https://rust.googlesource.com/rust-lang/cargo/+/HEAD/tests/testsuite/member_errors.rs
- sccache does not cache incremental workspace crates (`CARGO_INCREMENTAL=0` needed): https://neosmart.net/blog/tag/rustc/ , https://depot.dev/blog/sccache-in-github-actions
- nextest `slow-timeout` / `terminate-after`: https://nexte.st/book/running (exact keys to be re-checked against nexte.st config reference before use)
- PSD merged composite and `hasRealMergedData`: https://loc.gov/preservation/digital/formats/fdd/fdd000523.shtml , https://community.adobe.com/bug-reports-562/p-psds-exported-from-merged-photos-don-t-have-correct-compatibility-layer-causing-problems-w-indes-666960 , https://mail.gnome.org/archives/commits-list/2018-June/msg02314.html

</topic>
