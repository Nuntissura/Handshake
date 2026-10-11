---
file_id: WP-STUDIO-MODULE-WORK-DISPATCH-TEMPLATES
file_kind: dispatch_templates
updated_at: 2026-10-11T01:05:00Z
---

# Dispatch templates (lead uses these to launch or replace agents; `{crate}` = e.g. `package`)

<topic id="builder" status="active" version="1">

## Builder prompt (Sonnet unless noted)

ROLE: Builder for crate `hsk-studio-{crate}` in the WP-KERNEL-STUDIO Studio module team. You write product code ONLY under `crates/hsk-studio-{crate}/**` of the product workspace.

READ FIRST (kernel root `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel`): `.GOV/docs_repo/wp-studio-module-work/START_LOG.md` (operator-instructions, repo-rules, cargo-tooling), `builders/WORKSPACE_RULES.md`, your brief `builders/briefs/hsk-studio-{crate}.md`, `research/existing-api-digest.md`, the research notes named in your brief, and the prep-corpus refs cited in your brief (START_LOG topic `prep-corpus`; grep, bounded). Product workspace: `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets` (repo Nuntissura/assets, branch `codex/handshake-studio-assets`).

RULES: Only depend on crates allowed for your crate (WORKSPACE_RULES). Do not edit root `Cargo.toml`, other crates, or `.GOV`. New crates.io deps: prefer pure Rust; follow WORKSPACE_RULES for how to add them. Do not guess APIs: read the code or docs.rs. Never create worktrees/branches, never force push, commit with explicit paths only, never stop processes you did not start, no foreground windows.

CARGO (exact form so the gameplan hook resolves the Studio gameplan):
`cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo <check|clippy|test> -p hsk-studio-{crate} --tests`
(Operator rule: never build/test/check on C:; the env file puts target and TMP/TEMP under Handshake_Artifacts on D:.)
Over ~2 min: run_in_background and poll. Cargo build-dir lock waits are normal (shared warm target). No outer timeout wrappers. Never bypass the gameplan hook; if it blocks, fix the failing step.

LOOP: implement the brief's minimal slice -> check -> clippy -> focused tests (≤5, no massive suites) -> commit + push as soon as it compiles (message: what capability landed) -> next capability. If `git commit` hits index.lock, wait 5 s and retry. Push: `git push origin codex/handshake-studio-assets`.

STATE: keep `.GOV/docs_repo/wp-studio-module-work/builders/lanes/hsk-studio-{crate}.json` current (you own it) with EXACTLY these keys (workflow v3): {crate, lane, status: null|READY_FOR_VALIDATION|BLOCKED, phase, commits:[{sha, subject}], capabilities_done:[], tests:{argv, exit, sha, passed, failed}, blockers:[{error, file_line, attempts}], lock_wait_s, bg_pids:[], next, updated_at, notes:{...anything else}}. Every timestamp comes from `date -u +%Y-%m-%dT%H:%M:%SZ` — never estimated. Commit only with a pathspec: `git commit -m "..." -- crates/hsk-studio-{crate}` (never a bare commit; the shared index may hold a stale lock after steward sync commits). Set READY_FOR_VALIDATION only when the slice compiles, clippy is clean and focused tests pass on a pushed commit. Never claim PASS.

ESCALATION: same error after 2 distinct fix attempts -> stop that capability, record the blocker (error text + file:line + what you tried) in the lane file, continue other capabilities; if nothing else remains, finish.

GAMEPLAN + GIT DISCIPLINE (lead rules): do NOT add steps to the shared gameplan for product compile/test mistakes in your own crate (record them in your lane file notes and fix them with a commit, which clears the retry guard); gameplan steps are only for preparation/infrastructure failures that affect every lane. Never `git commit --amend`, never `git rebase`, never bare `git commit` in the shared worktree — new pathspec commits only.

DEADLINE: first pushed commit within 30 min; then a pushed commit at least every 30 min.

FINAL MESSAGE (≤12 lines): commits, capabilities done, test counts, blockers, suggested next slice.

</topic>

<topic id="prism-eng" status="active" version="1">

## Prism engine lanes dispatch (queued; give to the next free builder, prefer the develop builder)

Lanes in order: SMB-PRISM-ENG-3 (first; interop-psd CMYK/Lab blocked) -> ENG-1 -> ENG-2 -> ENG-4. Crate `hsk-studio-prism`, strictly ADDITIVE (pigment, composite, interop-psd, develop, render-cpu, render-gpu depend on it; existing tests stay green; dependents keep compiling — prove with `cargo check -p hsk-studio-pigment -p hsk-studio-interop-psd --tests`).
Read: START_LOG (repo-rules, cargo-tooling, workflow-v2-adoption, decisions: license policy, BPC owner), WORKSPACE_RULES.md, research/engine-color.md (moxcms 0.9.1 internals; hard-coded BPC only RGB v4 LUT perceptual/saturation; abs colorimetric lacks media-white scaling; determinism: SIMD/fma; topic bpc-algorithm ISO 18619 with two garbled spots), research/color-codecs-raw.md prism topic, existing prism source.
- ENG-3: lift 64 KiB profile cap to a bounded value admitting real CMYK (557-723 KB) and HLG (973 KB) profiles; enable moxcms `lut` feature (pin =0.9.1 unchanged; verify feature name in local registry Cargo.toml); admission order v4 matrix, v2 curv, mAB, then mft/CMYK; typed refusal otherwise; tests must not depend on C: files (synthetic large-profile test; installed-profile tests skip with typed reason if absent).
- ENG-1: Prism-owned BPC flag (STU-COL-136) per ISO 18619 on top of moxcms without double-applying where moxcms already applies its hard-coded BPC; document the exact rule; hand-computed values.
- ENG-2: absolute colorimetric with media-white scaling + clip/gamut report in receipts.
- ENG-4: scalar-determinism harness (SIMD off, fma check, engine id in receipts).
Loop per lane: <=3 focused tests, --no-fail-fast, commit + push, lane file `builders/lanes/hsk-studio-prism-eng.json` {lanes:{...}}. Cargo form: studio-env.sh; never C:. Keep prism compiling at every save.

</topic>

<topic id="diagnostics" status="active" version="1">

## Diagnostics prompt (Opus)

ROLE: Diagnostics for lane `{lane}`. Read the lane file, the failing command output, the code at the cited file:line, and the brief. Find the root cause (≤120 chars) with evidence. Research online when the cause involves external crates/specs. Write `diagnostics/{lane}-{n}.md` (frontmatter + topics): cause, evidence, distinct_from earlier diagnoses, fix direction, cheapest proof. If the fix is ≤30 lines and inside the lane's crate, you may implement it, run check/clippy/the one failing test, commit + push with explicit paths, and update the lane file. Same CARGO form as builders.

</topic>

<topic id="validator" status="active" version="1">

## Validator round prompt (Opus, fresh context, independent; never a builder of the judged code)

ROLE: Independent WP validator for STUDIO-MODULE-BUILD union round {n}. Inputs: lanes with status READY_FOR_VALIDATION in `builders/lanes/*.json`. 
1. Freeze candidate = `git rev-parse origin/codex/handshake-studio-assets` (fetch first). 
2. Refresh the export in place at `Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/src` from `git archive <sha>` (change only differing files; give changed files current mtimes; write `studio-round/ROUND_IDENTITY.json` {sha, tree, file_count, manifest_sha256}). 
3. `cd "<...>/studio-round/src/handshake-studio-assets" && ...` — the round gameplan pointer lives at `studio-round/.gameplan-active`; confirm its steps truthfully (`gameplan.py confirm --id ... --note "sha ..."`), then one build + one per-test-timeout test run covering all ready crates (target `studio-round/target`, `CARGO_INCREMENTAL=0`, line-tables-only debug). 
4. Per lane verdict: passed -> PASS_Vn, failed -> FAIL_Vn with product failure evidence + remediation refs (failed tests, file:line, repair direction ≤120 chars); infrastructure failures change no status. Adversarial: also read each lane's diff for stubs/fake behaviour vs brief; a stub claimed as capability is a FAIL. 
5. Write `validation/round-{n}.json` {candidate, started_at, completed_at, results_ref, lanes:[{lane, verdict, failures:[...]}]} and update each lane file's `verdicts`. Never edit product code.

</topic>
