---
file_id: diagnostics-stale-artifacts
file_kind: diagnostics
status: current
role: infra-researcher (read-only; no product edits, no cargo builds)
toolchain: cargo 1.97.1 (c980f4866 2026-06-30), rustc 1.97.1 (8bab26f4f 2026-07-14), x86_64-pc-windows-msvc, NTFS on C: and D:
updated_at: 2026-10-11T06:52:26Z
---

<topic id="summary" status="current" summary="ranked answer in 8 lines" updated_at="2026-10-11T06:52:26Z">

- Top cause (VERIFIED mechanism, VERIFIED writer for the develop incident): steward canaries compile EXPORT trees into the edit tree's target (`studio-team-dev/target`). Cargo hashes workspace-member paths relative to the workspace root, so both trees use the same fingerprint dirs and artifact names. Freshness is mtime-only, and the canary's `touch` of every export file makes its build start time newer than every edit-tree source, so the edit tree reuses artifacts compiled from older export content.
- The develop incident is traced to the second: the stale `libhsk_studio_develop-7d43f6a3707a753d.rlib` (mtime 08:31:40 local) was written by canary C9's `cargo test --doc` step (`doctest/hsk-studio-develop.stderr.txt`, mtime 08:31:40.8, log line `Compiling hsk-studio-develop ... steward-export\src\...`).
- Second cause (VERIFIED, lead's backup-restore case): content changed with an old mtime. On Windows `cmd copy`, PowerShell `Copy-Item`, `robocopy`, `cp -p`, `shutil.copy2` all preserve the old mtime (measured), so V4-11 is too narrow.
- Not causes: edit-during-build race (cargo rewinds the dep-info mtime to the unit start; exclusive build lock; measured NTFS tie window is at most about 4 ms), NTFS granularity, `-Zchecksum-freshness` absent (not available on stable).
- Stable 1.97.1 has NO checksum freshness (cargo rejects `-Z` on stable; nightly-only through Cargo 1.100 beta; absent from Rust 1.97 to 1.99 release notes).
- Separate target dirs per tree fully prevent cause 1 (cross-tree sharing is keyed by the target/build dir), but not cause 2, not contamination already in `studio-team-dev/target`, and not a tree that forgets `STUDIO_OWNER`.
- Live exposure at 2026-10-11T06:50:38Z: 34 edit-tree units (8 crates) are compiled from export content that differs from the edit tree and look fresh to cargo (upper bound). Touch list: 30 files.
- Recommendation: add the tree-binding guard to `studio-env.sh` (exact snippet in topic `recommendation`), seed the marker for the edit tree, and touch the 30 listed files once.

</topic>

<topic id="cause-candidates" status="current" summary="ranked causes with evidence and sources" updated_at="2026-10-11T06:52:26Z">

Times: local = UTC+2. "VERIFIED" = read in cargo source at tag `rust-1.97.0` or observed on disk; "INFERRED" = follows from verified facts but not observed; "UNVERIFIED" = no evidence either way.

## Rank 1. Cross-tree artifact sharing: canary/export builds write the edit tree's target

Mechanism, each step verified in cargo `rust-1.97.0`:

1. Unit identity. The `-C metadata` hash is built from `PackageId::stable_hash(ws_root)` plus features, profile, mode, kind, target name (`compilation_files.rs:210,747`). For path packages `SourceId::stable_hash` strips the workspace prefix; its doc comment: "For paths, remove the workspace prefix so the same source will give the same hash in different locations, helping reproducible builds" (`source_id.rs:562`). The fingerprint stores a hash of the source path relative to the root, and the dep-info stores relative paths (disk: all 1001 `deps/hsk_studio_*.d` list `crates\hsk-studio-...\src\...`, none absolute). Result: the same crate, features and profile in two trees maps to the same `.fingerprint/<crate>-<hash>/` and the same `deps/<name>-<hash>.rlib`. This answers the lead's path-relativity question: CONFIRMED.
2. Freshness is mtime only. Source mtimes (resolved against the CURRENT tree's package root) are compared with the dep-info mtime; cargo rewinds that mtime to the unit's `invoked.timestamp` (`compiler/mod.rs:424,507`); `path_mtime <= reference_mtime` means fresh (`fingerprint/mod.rs:2186`). The fingerprint JSON on disk shows `"checksum":false` and a relative `CheckDepInfo` path. Nothing identifies which tree or which content produced the artifact.
3. The canary writes the shared target. `studio-team-dev/steward-export/results/C5..C9/round.json` all have `target_dir` = `.../studio-team-dev/target` (C2 to C4 not checked). Export mode runs `find "$root" -type f -print0 | xargs -0 touch` (`tools/run-round.sh:173`), so every export file is newer than any dep-info and every unit rebuilds from export content, with T0 = canary time.
4. Back in the edit tree, a unit whose sources all have mtime <= T0 is fresh even if its content differs from the export. The export SHA always lags HEAD and builders hold uncommitted edits made before T0, so this is common. Matching symptoms: "library lacks items that exist in the source", "tests behave like old code", `touch` fixes it.
5. The reverse direction was already observed and half-fixed: `run-round.sh:169-172` comment and `START_LOG.md:162` ("10 crates fresh on a never-built export"). The touch fixes only the export side. `START_LOG.md:206` ("different source path = separate fingerprints") is wrong for workspace members; `START_LOG.md:228` already supersedes it.

Direct evidence:

| ID | Evidence | Status |
|---|---|---|
| E1 | Develop incident (`builders/lanes/hsk-studio-develop.json:418`): rlib mtime 08:31:40, pre-change content. Canary C9 (06:24:01Z to 06:35:03Z = 08:24:01 to 08:35:03 local) wrote `doctest/hsk-studio-develop.stderr.txt` at 08:31:40.82 with `Compiling hsk-studio-develop v0.1.0 (...\steward-export\src\handshake-studio-assets\crates\hsk-studio-develop)` then `Finished test profile in 2.75s`. Same second as the rlib. | VERIFIED (log mtime and rlib mtime match to the second; the rlib content was not re-inspected) |
| E2 | Live example, harness lib unit `hsk-studio-harness-0aef9148f810c9ba`: `invoked.timestamp` 08:24:35.99 (inside C9); sources `cli.rs` 07:39:11, `manual.rs` 07:39:25, `harness.rs` 07:40:18, `lib.rs` 07:40:28 (all older); `git status` shows those files modified and uncommitted, so the export (2074f5e) lacks them. Cargo would call the unit fresh. | VERIFIED state; consequence INFERRED from rule in step 2 |
| E3 | Detector snapshot 2026-10-11T06:50:38Z (`canary_exposure.py`): 2299 hsk-studio fingerprint units; last compiled inside a canary window: C9 394, C8 48, C7 29, C6 14, C4 13, C5 11, C2 2; 261 of those look fresh from the edit tree; 34 of them were compiled from content that differs from the edit tree: harness 4, interop-lrcat 5 (C7), kernel-bindings 4, pigment 8 (C7+C9), pulse 2 (C5), reel 6, score-device 1, web 4 (C7). Upper bound (does not model dependency-output ordering; counts only `crates/` sources). 60 at 06:41Z, 40 at 06:48Z: builders are rebuilding or touching meanwhile. | VERIFIED snapshot |
| E4 | Cargo maintainer on a reproducer with two worktrees and a shared target/build-dir (rust-lang/cargo#16642, comment 2026-02-14), paraphrased: the cache entry is keyed by a path relative to the workspace root, so both trees share one entry, and the build ran after both modifications so its mtime is newer and cargo sees no reason to rebuild. Fixes named: separate caches, checksums, user-wide cache. Issue is open (label A-rebuild-detection). Related: #7078, #2383. | VERIFIED (read on GitHub) |
| E5 | Validator rounds already use their own dir: R6 command line shows `--target-dir D:/.../studio-round/target` (`studio-round/RUNNING-R6.jsonl`). So the shared-target writers are the steward canaries (C2 to C9), not the validator round. | VERIFIED |

Which of the three incidents this explains:

- Develop rlib: VERIFIED (E1).
- Backup-restore case (press, V4-11): lead-verified; this is rank 2, not rank 1.
- Press `slice5_infra_note` ("library lacked items that exist in the source", `builders/lanes/hsk-studio-press.json:213`): UNVERIFIED. Canaries C7 (05:48:32Z to 05:54:50Z) and C8 (05:55:25Z to 06:01:04Z) preceded the 06:05Z V4-11 note, so rank 1 fits; rank 2 also fits.
- Discriminator for any future incident: `stat` the unit's `.fingerprint/<crate>-<hash>/invoked.timestamp`. Inside a canary `started_at..ended_at` window (`steward-export/results/C*/round.json`) = rank 1. Outside, with the failing source's mtime older than that timestamp and its content newer than the last build = rank 2.

### Do separate target dirs per tree fully prevent rank 1?

Yes for the class, with these limits:

- Prevented: any two trees compiling into different `CARGO_TARGET_DIR` (or different build-dir) share nothing, because unit hashes and fingerprint dirs are tree-independent and the only isolation boundary is the directory that holds them. Cargo has no per-tree key (#16642/#7078 are open design issues).
- Not prevented: (a) units already contaminated in `studio-team-dev/target` (E3) until their sources are touched or edited after the contaminated build; (b) any tree that sources `studio-env.sh` without `STUDIO_OWNER` (default owner `studio-team-dev`): `steward-sync.sh` sources it inside `sync-export` (it runs `cargo metadata`, which does not compile; whether its probes compile is UNVERIFIED), and probe or red-proof copies; (c) re-pointing one owner dir at a different checkout later; (d) rank 2, which is same-tree.
- Both validator and canary must therefore use distinct dirs, and a guard (topic `recommendation`) is the only way to keep it true without relying on 20 agents.
- Cost note: the canary gets no benefit from the warm shared target anyway, because export mode touches every file and rebuilds the workspace each run (C9 compiled hundreds of units). A separate canary target costs disk, not time.

## Rank 2. Content changed but mtime did not advance (restore, copy or move with preserved timestamps)

- Rule (VERIFIED): a unit is dirty only if some source mtime is newer than its dep-info (rewound to unit start). A file whose content changes but whose mtime is not newer than the last build start is invisible. Cargo's own module doc: "Filesystem mtime tracking is notoriously imprecise and problematic" (`fingerprint/mod.rs:42-43`).
- Windows-specific probe, run read-only in a scratch dir (`copy_mtime_probe.sh`), source mtime set to 2026-01-01:

| Mechanism | Destination mtime |
|---|---|
| Git Bash `cp` | now (safe) |
| Git Bash `cp -p` | 2026-01-01 (hazard) |
| `cmd copy` | 2026-01-01 (hazard) |
| PowerShell `Copy-Item` | 2026-01-01 (hazard) |
| `robocopy` default flags | 2026-01-01 (hazard) |
| Python `shutil.copy2` | 2026-01-01 (hazard) |
| Python `shutil.copy` | now (safe) |

  Rename or move keeps the file's own mtime. Git checkout, restore and stash write files with the current time (INFERRED from git behavior, not re-tested).
- V4-11 names only `cp -p` and archive extraction. It should name `copy`, `Copy-Item`, `robocopy`, Explorer paste, `shutil.copy2`, and rename or move of a `.bak` back into place.
- Backup restore into the shared tree is the lead-verified case (press). Status: VERIFIED by the lead, mechanism consistent with cargo rule.

## Rank 3 (unlikely). Edit during a build of that crate; mtime ties

- Cargo handles it conservatively: `invoked.timestamp` is written when the unit starts and the dep-info mtime is rewound to it, so a source modified after the start is newer and the unit rebuilds next time ("rewind ... to handle the case where the user modifies a source file while a build is running", `fingerprint/mod.rs:245-252`). VERIFIED.
- Residual: equal mtimes count as fresh (`<=`; history: #5918/#5919 changed to rebuild on equality in Cargo 1.30, reverted in 1.33 by #6484). The code comment says kernels can cache time between timer ticks, so a file updated shortly after a build starts can tie. Measured here (`mtime_granularity.py`, 300 back-to-back small-file writes): C: 186 distinct mtimes, D: 62 distinct, median step 1.5 to 2.0 ms, max step about 4 ms, mtime lags wall clock by 0.08 to 2.9 ms. So ties span at most a few ms; rustc needs far longer than that to start and read sources, so a write inside the tie window cannot land after rustc read the file. Not a plausible cause of the symptoms. NTFS resolution is 100 ns units, but the recorded values come from the cached system time (Microsoft: default timer period 15.6 ms, finer when some process raises the resolution; this host is finer).
- Two cargo processes on one target do not race on fingerprints: `Layout::new` takes `.cargo-build-lock` exclusive (`open_rw_exclusive_create`, `layout.rs:273`; only `-Zfine-grain-locking` on nightly relaxes it) and `prepare_units()` (`build_runner/mod.rs:175`, defined at 410) runs before per-unit `fingerprint::prepare_target` (`compiler/mod.rs:215`), so fingerprints are computed after the lock is held. Lock files on disk: `.cargo-lock`, `.cargo-build-lock`, `.cargo-artifact-lock` (1.96 lock split, #16708). The lock is silently skipped if `LockFileEx` returns `ERROR_INVALID_FUNCTION` (virtual or network filesystems; #16790 discussion); local NTFS on D: is not affected.
- Real compile errors from reading another agent's half-edited crate are NOT staleness. Signature: the error names an item missing from the source. Stale artifacts name an item that exists in the source (press note).

## Rank 4 (remote or eliminated)

- Microsoft File Times: "the file time is correctly reflected when the handle that makes the change is closed" and "last write time is not fully updated until all handles that are used for writing are closed". Relevant only to a writer that keeps a handle open across a build start. UNLIKELY for edit-tool writes.
- Eliminated by version: rmeta mtime not refreshed after rustc incremental skips rmeta generation (fixed Cargo 1.93, #16262); dep-info trailing backslashes on Windows (fixed 1.93, #16223); lld dylib fingerprint on Windows (1.45, #8290). All are older than 1.97.1.
- Cargo-side races reported in #16790 (rmeta stub errors, 1.94) were closed 2026-10-08 as a fault in the reporter's virtual filesystem, not Cargo. `cargo clean` without `-p` takes no locks (maintainer comment); the gameplan hook gates `cargo clean`.
- Other writers into the shared target from other trees: see the limits under rank 1.

</topic>

<topic id="stable-availability" status="current" summary="what Rust/Cargo 1.97.1 supports; checksum freshness is not available" updated_at="2026-10-11T06:52:26Z">

| Question | Answer | Verification |
|---|---|---|
| Checksum-based freshness on stable 1.97.1 | NO | `cargo -Z checksum-freshness version` in the edit tree: "the `-Z` flag is only accepted on the nightly channel of Cargo, but this is the `stable` channel". Cargo unstable docs list `checksum-freshness` as unstable, tracking issue #14136 (open, labels Z-checksum-freshness, S-waiting-on-feedback, updated 2026-09-08). |
| Since when / state | Added as nightly in Cargo 1.83 (#14137). Config key `build.fingerprint = "content"` (default `"mtime"`, env `CARGO_BUILD_FINGERPRINT`) merged 2026-08-22 (#17382), listed under "Nightly only" in Cargo 1.100 (2026-11-12) of the changelog; the stable config docs have no `build.fingerprint`. | Cargo book CHANGELOG (grep of all sections), PR #17382 body via `gh api`. |
| Stabilized in 1.97 to 1.99? | NO | Rust `RELEASES.md`: 1.97.0, 1.97.1, 1.98.0, 1.98.1, 1.99.0 contain no checksum/fingerprint/freshness/mtime item. 1.97.1 (2026-07-16) is an LLVM miscompilation backport only. Stable at today's date is 1.99.0 (2026-10-01); the pinned 1.97.1 is two minors behind. |
| rustc side | The checksum is emitted by rustc only on nightly: cargo's fingerprint module doc: "Only nightly rustc will emit the needed metadata at the time of writing. This is dependent on the unstable feature `-Z checksum-hash-algorithm`." | `fingerprint/mod.rs:36-40` at `rust-1.97.0`. |
| Forcing it on stable (`RUSTC_BOOTSTRAP=1`) | Not viable: unsupported, needs a nightly rustc to emit checksums, and changes the pinned toolchain contract. Not recommended. | Follows from the two rows above. |
| `CARGO_BUILD_BUILD_DIR` / `build.build-dir` | Stable since Cargo 1.91 (2025-10-30, #15833). Intermediate artifacts (`.fingerprint`, `deps`, `incremental`, `build`) go to build-dir; final artifacts (bins, doc) to target-dir. Supports templates `{workspace-root}`, `{cargo-cache-home}`, `{workspace-path-hash}` (hash of the manifest path) at `rust-1.97.0`. Default = target-dir. | Config docs and `build-cache.md` at tag `rust-1.97.0`. Splitting the dirs does not change sharing: build-dir is what holds the shared fingerprints. |
| Build-dir locking on 1.97 | `.cargo-build-lock` exclusive for the whole invocation, `.cargo-artifact-lock` exclusive, `.cargo-lock` shared (1.96 split, #16708/#16887). Fine-grained unit locking is nightly-only (`-Zfine-grain-locking`). | `layout.rs:268-302` at `rust-1.97.0`; the three lock files exist in the target dir. |
| Path-relative unit hashing | Present in 1.97.0 (same code as master). | `source_id.rs:562`, `compilation_files.rs:210,747` at `rust-1.97.0`. |
| sccache | Installed: `C:\Users\Ilja Smets\.cargo\bin\sccache.exe`, v0.18.0 (latest release 0.18.0, 2026-09-14). No `RUSTC_WRAPPER` or `SCCACHE_*` in the environment; `studio-env.sh` does not use it. A small `studio-team-dev/sccache` dir exists on D: (created 03:25; not otherwise examined). sccache docs: "rustc's incremental compilation needs to be disabled"; bin, dylib, cdylib and proc-macro crates cannot be cached. | `where sccache`, `sccache --version`, sccache `docs/Rust.md`. Side effect: my `sccache --show-stats` started an idle sccache server (no cache written). |

Practical reading for 1.97.1: the only freshness signal is mtime (plus the unit hash), so the fix must make stale mtimes impossible by construction or detect them, not rely on content hashing.

</topic>

<topic id="mitigations" status="current" summary="options with cost, risk, verification" updated_at="2026-10-11T06:52:26Z">

Disk context: D: has 3122 GB free (C: 634 GB), both NTFS. Disk is not the constraint; time and build parallelism are (today the exclusive build lock serializes all builders).

| ID | Change | Cost (disk / time) | Risk | How to verify |
|---|---|---|---|---|
| M1 | Every tree that is not the edit tree builds into its own target: steward canary/export, sync-export, probes, red-proof copies (`STUDIO_OWNER=<tree-owner>`; validator already uses `studio-round/target`). Lead has already ordered this for the steward. | Disk: one more full target, same order as `studio-round/target`. Time: none extra for the canary (export mode already rebuilds everything each run; first run is cold once). | Low. Does not clear existing contamination; relies on every script setting the owner. | After the next canary, no edit-tree unit has `invoked.timestamp` inside a canary window: rerun `canary_exposure.py`; "last compiled inside a canary window" should stop growing and exposed units reach 0. |
| M2 | Tree-binding guard in `studio-env.sh`: marker `${CARGO_TARGET_DIR}/.studio-workspace-root` holds the cargo workspace root; sourcing from another root fails closed. Snippet tested offline (5 cases) in topic `recommendation`. | About 250 to 520 ms per source (`cargo locate-project`, measured); no disk. | A path-spelling change makes the guard refuse (fail-closed, noisy but safe). The first sourcing after the change must be the edit tree or the marker pins the wrong tree: pre-seed it. | `tree-bind-guard-test.sh`: edit tree binds and passes (also from a crate subdir), export tree with the same target refuses (exit 1), non-workspace dir refuses. |
| M3 | One-time clean-up of already contaminated units: `touch` the files in `canary_exposure.touch-list.txt` (30 files, 8 crates). Any source mtime newer than the unit start makes cargo rebuild the unit and its dependents. Rerun the detector right before, because the list shrinks as builders rebuild. | Time: rebuild of those crates plus dependents under the build lock (minutes). | None to content (mtime only). Touching a file an agent is mid-edit on is harmless. | Detector reports 0 exposed units. |
| M4 | `CARGO_BUILD_BUILD_DIR=<studio-team-dev>/build-{workspace-path-hash}` with a shared `CARGO_TARGET_DIR`: per-manifest-path isolation by construction (stable since 1.91, template present at 1.97). | Disk: one build-dir per tree. | Moves test executables (`deps/`) out of `$CARGO_TARGET_DIR/debug/deps`; any script that globs that path breaks (UNVERIFIED which). Final uplifted bins stay shared across trees. | Would need a dry run of `run-round.sh` and nextest listing. Not recommended while M1+M2 are available. |
| M5 | Per-agent target dirs (`STUDIO_OWNER` per agent). | Disk: about 20 targets. Time: each agent cold-builds the dependency closure; builds then run in parallel and compete for CPU and RAM (today the lock serializes them, `START_LOG.md:107`). | Removes cross-agent staleness but does not touch rank 2; heavy. | Not needed for the observed causes. Reject. |
| M6 | sccache with per-agent targets. | Needs `CARGO_INCREMENTAL=0` (loses rustc incremental); bin, proc-macro and test executables are not cacheable. | Does not fix the shared-target bug: a unit cargo judges fresh never reaches the wrapper, and sccache keys differ per path anyway (reported in #16804 discussion). | Reject for this problem. Only a cold-start accelerator if M5 were ever chosen. |
| M7 | Agents `touch` edited or restored files after each save (V4-11 extended with the measured command list). | Time: negligible. | Fails open: depends on 20 agents; does nothing for canary contamination. | Keep the rule, extend the list (rank 2 table). |
| M8 | "Freshen" script before each build: keep a per-file content-hash manifest, `touch` files whose content changed while mtime is not newer than the last scan. Mechanical catch for rank 2 only. | About one pass over about 3k source files per cargo command (not measured). | New moving part; stale manifest if it is skipped. | Restore a file with `Copy-Item` from an old copy; confirm the next build rebuilds the crate. Optional, only if rank 2 recurs after M7. |
| M9 | Build only the agent's own crate and check dependents with `--locked`. | None. | `--locked` only forbids Cargo.lock changes; it has no effect on freshness. Fewer compiled dependents does reduce exposure to others' in-flight edits. | n/a for staleness. |
| M10 | Checksum freshness. | n/a | Not available on 1.97.1 (topic `stable-availability`). | n/a |

</topic>

<topic id="recommendation" status="current" summary="one minimal change for studio-env.sh plus a one-time touch; lead decides and applies" updated_at="2026-10-11T06:52:26Z">

Recommendation: keep the lead's M1 (export/canary builds into their own target) and make it impossible to regress with M2, the tree-binding guard in `builders/studio-env.sh`. Then run M3 once. No other change is needed for the verified causes; extend V4-11's command list (rank 2 table) as a documentation edit.

### 1. Append to `builders/studio-env.sh` (after the existing `mkdir -p` line; sourced scripts must keep the `cd "<dir>" &&` prefix)

```bash
# Bind the target dir to ONE cargo workspace root. Cargo hashes workspace-member paths relative to the
# workspace root (SourceId::stable_hash), so a second tree built into the same target shares unit hashes,
# fingerprint dirs and artifacts, and mtime freshness then reuses artifacts compiled from the other tree.
# See diagnostics/stale-artifacts.md. Export, canary, probe and red-proof trees must set STUDIO_OWNER.
_studio_ws="$(cargo locate-project --workspace --message-format plain 2>/dev/null)"
_studio_bind="${CARGO_TARGET_DIR}/.studio-workspace-root"
if [ -z "${_studio_ws}" ]; then
  echo "studio-env: no cargo workspace at ${PWD}; cd to the workspace root before sourcing" >&2
  return 1
fi
if [ -f "${_studio_bind}" ] && [ "$(cat "${_studio_bind}")" != "${_studio_ws}" ]; then
  echo "studio-env: ${CARGO_TARGET_DIR} is bound to $(cat "${_studio_bind}"), not ${_studio_ws}; set STUDIO_OWNER to a per-tree owner (export, canary, probe and red-proof trees never share the edit tree's target)" >&2
  return 1
fi
[ -f "${_studio_bind}" ] || printf '%s' "${_studio_ws}" > "${_studio_bind}"
```

Tested offline against a scratch target with `cargo locate-project` only (no build): edit tree binds and passes, also from `crates/hsk-studio-accord`; export tree `.../steward-export/src/handshake-studio-assets` with the same target refuses with exit 1; a non-workspace dir refuses. `return 1` makes a `cd ... && source studio-env.sh && cargo ...` chain stop before cargo runs. Costs about 250 to 520 ms per source.

### 2. Seed the marker before the first sourcing (once), so the steward cannot bind the edit tree's target to the export

```bash
printf '%s' 'D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-studio-assets\handshake-studio-assets\Cargo.toml' > 'D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/target/.studio-workspace-root'
```

The string equals what `cargo locate-project --workspace --message-format plain` prints in the edit tree.

### 3. One-time clean-up of contaminated units (M3)

Rerun the detector, then touch the listed files in the edit tree (workspace-relative paths, mtime only):

```bash
python -I "<scratchpad>/infra-stale-artifacts/canary_exposure.py"
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && xargs -d '\n' touch -- < "<scratchpad>/infra-stale-artifacts/canary_exposure.touch-list.txt"
```

Snapshot at 2026-10-11T06:50:38Z: 30 files in harness, interop-lrcat, kernel-bindings, pigment, pulse, reel, score-device, web. Rerun the detector afterwards; expected 0 exposed units.

### 4. Doc edits (no code)

- `DISPATCH_TEMPLATES.md` V4-11: add `copy`, `Copy-Item`, `robocopy`, Explorer paste, `shutil.copy2`, and rename/move of a backup into place to the mtime-preserving list.
- `START_LOG.md:206`: replace "different source path = separate fingerprints" with "workspace members hash relative to the workspace root; separate trees need separate target dirs" (`:228` already points here).
- Triage rule for builders: on a compile error naming an item that exists in the source, first `touch` the unit's crate sources and rerun (V4-11), then report which `invoked.timestamp` window the unit fell in.

### Not recommended now

M4, M5, M6 (reasons in topic `mitigations`); `-Zchecksum-freshness` is unavailable on 1.97.1.

</topic>

<topic id="evidence-index" status="current" summary="scripts, outputs and sources" updated_at="2026-10-11T06:52:26Z">

Scripts and outputs (read-only; under `C:\Users\ILJASM~1\AppData\Local\Temp\claude\D--Projects-LLM-projects-Handshake-Handshake-Worktrees-wt-gov-kernel\3d7cf734-c553-4e65-b0c2-c7b829f4ba00\scratchpad\infra-stale-artifacts\`):

- `canary_exposure.py` (+ `canary_exposure.out.txt`, `canary_exposure.touch-list.txt`): which edit-tree units were last compiled inside a canary window from content that differs from the edit tree.
- `depinfo_trees.py`: rustc dep-info files carry relative source paths.
- `mtime_granularity.py`: back-to-back write tie measurement (writes only its own scratch files; removed).
- `copy_mtime_probe.sh`: which copy/move tools preserve an old mtime (scratch dir, removed).
- `tree-bind-guard.sh`, `tree-bind-guard-test.sh`: the candidate snippet and its offline test.
- `dl/`: downloaded cargo sources and changelog (text only, never executed).

Local facts used: `rust-toolchain.toml` pins 1.97.1; `.fingerprint/hsk-studio-accord-284f39ed08ae0cc5` (dep-info mtime 04:21:00.2355691 equals `invoked.timestamp` mtime: the rewind, `"checksum":false`); three lock files in `target/debug`; `steward-export/results/C5..C9/round.json`; `studio-round/RUNNING-R6.jsonl`; `studio-team-dev/steward-export/results/C9-2074f5e/doctest/hsk-studio-develop.stderr.txt`; `tools/run-round.sh:169-173`; `START_LOG.md:107,162,206,228`.

Side effects of this research: none on project files other than this file. `sccache --show-stats` started an idle sccache server (no cache dir created at the default location). Temporary probe dirs on C: and `studio-team-dev/tmp/infra-mtime-probe` were created and removed. Two long `du`/directory scans I started were stopped (they were mine).

Sources:

- Cargo source at tag `rust-1.97.0`: [fingerprint/mod.rs](https://github.com/rust-lang/cargo/blob/rust-1.97.0/src/cargo/core/compiler/fingerprint/mod.rs) (module doc lines 15-45, 217-260, 336-362; `find_stale_file` ~2109-2200), [source_id.rs](https://github.com/rust-lang/cargo/blob/rust-1.97.0/src/cargo/core/source_id.rs) (line 562), [compilation_files.rs](https://github.com/rust-lang/cargo/blob/rust-1.97.0/src/cargo/core/compiler/build_runner/compilation_files.rs), [layout.rs](https://github.com/rust-lang/cargo/blob/rust-1.97.0/src/cargo/core/compiler/layout.rs), [compiler/mod.rs](https://github.com/rust-lang/cargo/blob/rust-1.97.0/src/cargo/core/compiler/mod.rs).
- Cargo issues: [#16642 stale artifacts across worktrees](https://github.com/rust-lang/cargo/issues/16642), [#7078 canonicalize src path](https://github.com/rust-lang/cargo/issues/7078), [#2383 shared target-dir stale builds](https://github.com/rust-lang/cargo/issues/2383), [#14136 checksum freshness tracking](https://github.com/rust-lang/cargo/issues/14136), [#16790 race report, closed as VFS fault](https://github.com/rust-lang/cargo/issues/16790), [#16804 concurrent build cache](https://github.com/rust-lang/cargo/issues/16804), [PR #17382 build.fingerprint](https://github.com/rust-lang/cargo/pull/17382), [PR #16708 build-dir lock split](https://github.com/rust-lang/cargo/pull/16708).
- [Cargo changelog](https://doc.rust-lang.org/nightly/cargo/CHANGELOG.html) (1.83 checksum-freshness nightly; 1.91 build-dir stable; 1.93 #16262 and #16223; 1.96 lock split; 1.100 `build.fingerprint` nightly-only), [Cargo unstable features: checksum-freshness](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#checksum-freshness), [Cargo config: build.build-dir](https://doc.rust-lang.org/cargo/reference/config.html#buildbuild-dir), [Rust RELEASES.md](https://github.com/rust-lang/rust/blob/master/RELEASES.md).
- [Microsoft: File Times](https://learn.microsoft.com/en-us/windows/win32/sysinfo/file-times), [Microsoft archived blog: platform timer resolution (default 15.6 ms)](https://learn.microsoft.com/cs-cz/archive/blogs/maartenb/who-changed-my-platform-timer-resolution) (found via search, not fetched in full).
- [sccache Rust docs](https://github.com/mozilla/sccache/blob/main/docs/Rust.md).

</topic>
