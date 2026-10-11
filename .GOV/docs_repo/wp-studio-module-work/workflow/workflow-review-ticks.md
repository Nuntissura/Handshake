---
file_id: WP-STUDIO-MODULE-WORK-WORKFLOW-REVIEW-TICKS
file_kind: workflow_tick_reviews
updated_at: 2026-10-11T02:14:00Z
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
