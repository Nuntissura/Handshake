---
file_id: WP-STUDIO-MODULE-WORK-WORKFLOW-REVIEW-TICKS
file_kind: workflow_tick_reviews
updated_at: 2026-10-11T05:45:00Z
---

<topic id="tick-1" status="active" updated_at="2026-10-11T02:14:00Z" summary="Interval 01:20Z-02:10Z; lock-fix challenge">

## Tick 1 — 2026-10-11T02:14Z (interval 01:20Z–02:10Z)

**Lock fix (lead's plan: entrypoint derives the lock at round time, verdict binds SHA + lock hash, steward commits the lock back) — reject as the verdict basis; keep the derivation code, move it to commit time.**
1. CX-VAL-009/002: the tested tree (candidate + derived lock) matches no commit. The commit-back puts lock(candidate) on a later HEAD whose manifests have changed (8 of the 24 commits after faf1be4 touched a crate `Cargo.toml`), so it is stale again. Proof reuse also fails, because `Cargo.lock` is an input of every crate.
2. CX-VAL-007: at run time the round would rewrite a version-controlled input from machine-local registry-cache state. That is not reproducible, and it disables `--locked`, which is the check that catches this drift.
3. Incomplete: root `[workspace.metadata.studio.registry]` and the probe APPROVED table drift with manifests too. R2 probe already rejected score with `registry_features_missing`. The entrypoint would also have to rewrite the root `Cargo.toml`.
4. Hazard in the temp-index commit:
   - `commit-tree`/`update-ref` without the old value silently drops concurrent builder commits, so require CAS (`update-ref HEAD <new> <old>`) or `GIT_INDEX_FILE` + `git commit`.
   - The shared index keeps the old lock blob, so a builder doing `git add` plus a bare `commit` reverts the lock. Builders may commit only with `git commit -m .. -- crates/hsk-studio-<own>`.
- Alternatives:
  - Builders commit the lock: no. The shared-tree lock merges every lane's uncommitted manifests (` M Cargo.lock` now).
  - Steward lands every dep change first: always consistent, but that is 25 crate-manifest commits in 40 min queued on one steward.
  - **Freeze only on steward sync commits: yes.** The steward runs `registry_sync.py --write` plus an offline lock derivation from HEAD's manifests and commits with CAS. The lead dispatches the round naming that SHA, and the validator freezes it (CX-VAL-001 does not require HEAD).
  - CX-EXEC-015(4): push a `run-round.sh` step 0 that runs `cargo metadata --locked --offline` and the `registry_sync` check before the export. A stale lock then costs ~1 s, not R2's 12.5 min (open 01:53:58Z, canary 02:03Z, close 02:06:25Z).

**Interval:**
- 66 product commits (01:27Z–02:07Z), all single-crate except steward faf1be4; 4 kernel commits (ratio 0.06, target ≤0.5 met).
- Bottleneck is now verdicts: 16 lanes READY, 1 verdict (Type PASS_V4, R1), R2 not opened. HEAD was lock-consistent for 53 s after faf1be4 (87c1cd9 changed a manifest), then 23 stale HEADs in a row.
- Repeated: guessed timestamps. Lead ticks 3–5 were corrected, and 12 of 17 lane files are stamped in the future (up to 09:10Z).
- Lane files have no `lock_wait_s`, `bg_pids` or `foundation_mt`. They use 40 distinct keys, and 6 disagree with the ledger. Whether a running test holds the build lock (F4) is still unmeasured.
- Shared-tree compile break repeated (pigment E0583); a rule now covers it. Subagent cap of 20 hit; 13 running (11 Sonnet, 2 Opus).

**Changes:**
1. Rounds run only on steward sync commits, at most every 20 min while READY lanes wait, plus the 1 s locked pre-check in `run-round.sh`.
2. Lane template:
   - stamps only from `date -u`;
   - fixed keys: crate, mt, status, commits, tests{argv,exit,sha}, blockers, lock_wait_s, bg_pids, next, updated_at;
   - everything else goes under `notes`.

</topic>

<topic id="tick-2" status="active" updated_at="2026-10-11T05:45:00Z" summary="R5 failure classes verified; workflow v4 amendments A1-A10">

## Tick 2 — 2026-10-11T05:45Z (R5 at sync 1d404f8; lead classes checked against `validation/R5-review-{a,b,c}.json` and the HEAD source)

**Verification of the lead's classes:**
- C1 hostile arithmetic/allocation: 6 findings in 5 lanes, not 6 lanes. They are reel `video.rs:269`, reel-native `host.rs:1056` and `wire.rs:514` (matroska.rs only admits the dimensions), develop `dng.rs:449`, lrcat aggregate bytes, and interop-layout clone amplification.
- C2 silent green: all 5 confirmed (render-gpu, prism C: paths, press `is_proven`, harness `verify`, lrcat `reconciles`). It is also latent elsewhere: `NOT_PROVEN` appears 33 times in 10 test files at HEAD, in interop-fig, score-device, score-plugin, prism and render-gpu.
- C3 CAP duplicates: in R5 the confirmed ones are the reel-native supervisor and the interop-media timecode, which repeats R3. The interop-fig ZIP reader is D16 text, not an R5 CONFIRMED finding. Confirmed CAP duplicates across reviews: R2 6, R3 2, R5 2, so 10 in total and present in every round.
- C4 test hooks: 2 confirmed. `test_fault|TestFault` appears in 7 `src/` files at HEAD.
- C5 hook: confirmed live. This tick's shell heredoc append was blocked by the hook, which resolved WP-KERNEL-012's gameplan from the kernel cwd. I re-did the append with the Edit tool.
- C6 same-class re-FAIL: 4 lanes, not 2. They are controls (integer step), motion-js (classify), components (`check_shape` with vars=None) and interop-media (timecode, "as in R3").
- New C7: guarantees claimed in doc comments are false. Examples: dng "a tiny file cannot force a large allocation", reel-native `wire.rs:8-9`, kernel-bindings "refuses everything", components "never return Ok", psd-wide "BPC is the identity", and R2 package "nothing is ever inflated". That is 6 or more findings.
- New C8: hostile text handled with substring scans. Web misses CSS escapes and `<base href>`; vector finds `<!DOCTYPE` inside a comment.
- Ops: the rate limit killed R4 after 72 min unfinished (R3 took 34 min; R5 reviews took 29 min).

**v4 amendments.** E0 means round entrypoint step 0, which is checked-in, read-only, about 1 s, owned by the steward, and compliant with CX-VAL-007. Steward adds `--step0` so builders can self-check before READY with the same script. Config extends the existing `.config/round-features.toml`; no new files.

1. **Overflow lint on decoders (C1).**
   - Rule: every module that decodes file, wire or worker bytes starts with `#![deny(clippy::arithmetic_side_effects)]`, and all size math uses `checked_*`.
   - Lives in: brief checklist (list the hostile modules), builder template.
   - Enforcement: the builder's existing clippy run fails; E0 runs `git grep -L` for the attribute over the listed modules and expects nothing; the round adds a clippy unit.
2. **Bound before allocate (C1).**
   - Rule: check the per-item bound and the running aggregate against a declared `Limits` before allocating. Each decoder has 2 fixed tests, `dims_u32_max` and `aggregate_over_budget`, both expecting a typed refusal.
   - Lives in: brief tests section, builder template.
   - Enforcement: E0 reads the per-test runner's list and requires both names in every crate marked `hostile`.
3. **No silent green (C2).**
   - Rule: a test that depends on the environment uses `#[ignore = "requires X"]` or fails; it never returns Ok early. Tests contain no machine-local paths. Verdict functions such as `is_proven`, `reconciles` and `verify` return not-proven when zero checks ran or any input could not be examined, and each has a negative test.
   - Lives in: builder template, entrypoint.
   - Enforcement: the entrypoint reports a passing test whose stored output contains `NOT_PROVEN` as NOT_PROVEN, not pass (this aligns with steward-2's current work). E0 greps `tests/` for `C:/|Program Files|System32|/Users/`.
4. **Fault hooks out of product (C4).**
   - Rule: fault injection exists only behind `#[cfg(test)]` or the non-default feature `test-faults`; it is never `pub` and never read from the environment in a default build.
   - Lives in: WORKSPACE_RULES, builder template.
   - Enforcement: E0 greps `crates/*/src` for fault-hook identifiers without a `cfg(feature = "test-faults")` within the 2 lines above. The probe refuses `test-faults` in any `default` feature list.
5. **Capability ratchet (C3).**
   - Rule: each cross-cutting capability has one owner, or is FROZEN(Dn) with no new copy allowed (ZIP read D16, worker supervision D18, timecode belongs to pulse).
   - Lives in: WORKSPACE_RULES capability table, brief checklist ("capabilities needed from outside the crate -> owner").
   - Enforcement: E0 fingerprint grep for `0xEDB88320`, `0x04034b50`, `CREATE_NO_WINDOW` and `17_?982`. Any hit outside the allow-listed files in the config is refused and names the lane. The allow-list may only shrink.
6. **Fix the class, not the instance (C6).**
   - Rule: a remediation dispatch names the `failure_class` and gives a sibling regex taken from the reviewer. Before READY the builder lists every hit under `notes.siblings` as fixed (with a test) or not applicable (with a reason).
   - Lives in: lead remediation dispatch, builder template.
   - Enforcement: the validator re-runs `git grep -c <regex>` and compares the count with the `notes.siblings` entries. A mismatch means the lane is not admitted (CX-VAL-008). A second FAIL still escalates to Opus.
7. **Guarantees cite a proof (C7).**
   - Rule: an absolute doc claim (never, always, cannot, refuses, every, guarantee) carries `proof: <test::name>` in the same doc block; otherwise the doc states the actual limit.
   - Lives in: builder template.
   - Enforcement: the builder greps its own crate's `src` doc comments for those words where `proof:` is missing. The hit list goes into the reviewer prompt, which makes review targeted and cheap.
8. **Tokenize hostile text (C8).**
   - Rule: CSS, XML, SVG and HTML input goes through a tokenizer that handles comments, escapes, CDATA and processing instructions, never `find`/`contains` on raw text. Each such parser has at least 3 tests named `adversarial_*` (escaped token, token inside a comment, decoy).
   - Lives in: brief checklist.
   - Enforcement: E0 lists `find("<!`, `find("url(` and `contains("<!DOCTYPE` hits plus the count of `adversarial_*` tests for crates marked `hostile_text`.
9. **Text payloads go through files (C5).**
   - Rule: commit messages, JSON and notes are written with the Write/Edit tool and passed by file (`git commit -F <file> -- <paths>`). Gated commands are still typed literally and never wrapped. Shell commands run with a Studio-scoped leading `cd`, never from the kernel cwd.
   - Lives in: builder template, lead procedure.
   - Enforcement: count hook blocks in lane notes. The root-cause fix remains D3, Operator-owned.
10. **Rate-limit resilience (ops).**
    - Rule: rounds and reviews checkpoint per lane and resume on the same SHA. No reviewer fan-out. At most 8 Opus agents and at most 18 agents running in total, keeping 2 free under the cap of 20 for the validator and diagnostics. When budget is tight, verdict agents are launched before new slices.
    - Lives in: lead procedure.
    - Enforcement: the lead counts running members in `team.json` before each dispatch.

**Next tick:** pinged at :11/:26/:41/:56 local when the lead is idle. I will check:
- whether A1-A10 were adopted;
- R5 formal verdicts;
- the re-FAIL rate of the 9 R5 remediation lanes;
- E0 hit counts;
- running Opus and total agents against the cap.

</topic>
