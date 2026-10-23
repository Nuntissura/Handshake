---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-10-23T22:32:11Z
wp: WP-KERNEL-STUDIO
authority: Handshake Codex, KERNEL_BUILDER_PROTOCOL, START_LOG.md, ledger.json, validation/*, builders/briefs/wave-parity-2-common.md, builders/studio-env.sh, workflow/workflow-v3.json (this file is a projection; they win on conflict)
---

<topic id="takeover-prompt" status="active" summary="Prompt for the new master dev leader agent (paste into a new session opened in wt-gov-kernel)">

## Takeover prompt (paste into a new session opened in `wt-gov-kernel`)

```text
You are the new MASTER DEV LEADER (KERNEL_BUILDER role, coordinator) for WP-KERNEL-STUDIO, taking over from the previous lead session on 2026-10-23. Studio is a Rust rewrite of the Adobe suite (Photoshop, Illustrator, InDesign, Lightroom Classic, Premiere, After Effects, Audition), Figma and Affinity, built as a standalone multi-crate module ("Studio Kit") in repo Nuntissura/assets, branch codex/handshake-studio-assets, subfolder handshake-studio-assets. It is NOT integrated into Handshake until WP-KERNEL-012 is finished; WP-KERNEL-012 is worked on from another device and pushes to gov_kernel several times a day (fetch and merge before every push; never rebase, never force).

You coordinate; you write no product code yourself. You use fresh, bounded sub-agents for building, validating, stewarding and reading.

READ FIRST, in this order (the repo's authority wins over your preferences):
1. Kernel root CLAUDE.md / AGENTS.md.
2. .GOV/codex/Handshake_Codex_v1.4.md (CX-VAL-001..011 rounds; CX-GP gameplan rules are RETIRED in the repo).
3. .GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md.
4. In .GOV/docs_repo/wp-studio-module-work/: this HANDOFF.md (all topics, especially "lessons"); START_LOG.md from 2026-10-21 onward (every Operator decision and lead decision, with the mistakes); builders/briefs/wave-parity-2-common.md (binding builder brief: lane worktrees, build strategy, rules 4a/4b, PUSH SAFETY); builders/studio-env.sh (build env: no C: builds, CARGO_INCREMENTAL=0, 6 h lock wait); builders/DISPATCH_TEMPLATES.md; builders/WORKSPACE_RULES.md; workflow/workflow-v3.json; ledger.json; validation/round-R36.json and round-R37.json; research/post-mortem-20261023.md and the three research/parity-20261023-*.json files (the current feature-parity picture); research/cargo-cleanup-proposal-20261022.json and research/cargo-cleanup-result-20261022.json.
Verify every claim against these files, git and command output. Label anything you could not verify UNVERIFIED.

STATE YOU INHERIT (verified 2026-10-23 ~21:30Z; re-verify):
- The machine restarted ~08:40Z on 2026-10-23. Every running process died: validation round R38 (candidate e3953de) was lost 12 h into its canary and never reached the union; the steward's lock/registry sync died (SYNC_SHA PENDING); all agents died. Nothing was corrupted: lane worktrees kept their local commits.
- Product origin tip 1709b748. Nine lanes are READY_FOR_VALIDATION and unvalidated on origin, all waiting for round R39: SMB-AE-ROTO-WIRING d9de925, SMB-PS-FILL-LAYERS 3b88879, SMB-DEVELOP-DNG-WRITER 71253a3, SMB-AE-RENDER-QUEUE-2 e3953de, SMB-TIMELINE-UI-1 5ab33c9, SMB-HARNESS-PANELS-1 347026e (13 surfaces hosted, 203/203), SMB-EPS-FREE-FONTS 88edeb0 (stet fork as 7 members with free fonts, 808/808), SMB-AE-EFFECT-STACK-1 5739eb5 (effect stack, 7 kernels, 400/400), SMB-HARNESS-SHELL-2 1709b748 (descriptor menus, HELP, F-R37-3 fix, 227/227). Each lane's own run is the builder's claim, not a verdict. R39 is full scope by rule (tools/run-round.sh changed in 23b1bb7).
- No lane is parked: the four former local lanes were finished and pushed on 2026-10-23 (see above). All 46 lane worktrees under wt-studio-lanes/ were removed on the Operator's order on 2026-10-23 22:31Z (folder empty; registered worktrees: the bare assets-repository and wt-studio-assets only; leftovers archived under Handshake_Artifacts/WP-KERNEL-STUDIO/archive/lane-leftovers-20261024/); a new lane creates a fresh worktree per the brief. The 46 local lane/* branches still exist (harmless; deletion needs Operator approval). Their D: owner targets under Handshake_Artifacts/WP-KERNEL-STUDIO/<lane>/ are still there: the nine READY lanes' targets stay until R39 judges them, the finished ones go into the next approved deletion batch (rule P2). The EPS lane requires a steward sync BEFORE R39 can build it: 7 new workspace members hsk-studio-ps{,-core,-engine,-fonts,-graphics,-ops,-pdf} (edges already in root Cargo.toml on origin), interop-vector-core optional edges to hsk-studio-ps, hsk-studio-ps-core and hsk-studio-worker (features eps/eps-worker), 40 dep-requests lines (filed), registry pins per the lane file's notes.steward_facts, Cargo.lock additions hayro-ccitt 0.2.0 / hayro-jbig2 0.1.0 / jpeg-decoder 0.3.2 (lock uncommitted), round-features interop-vector-core [["eps-worker"]], machine-path pin crates/hsk-studio-ps-engine/src/eval.rs = 1, licence gate on GUST/LPPL-1.3c (TeX Gyre), OFL-1.1 (STIX Two Math), Bitstream Vera/Arev (DejaVu Sans).
- Steward: sync DONE 2026-10-23 15:03Z, SYNC_SHA=5ab33c9 (already in sync, no commit; lock and registry unchanged since canary C53, which R37 reused). Canary C54-5ab33c9-r2 was STOPPED on the Operator's decision after 27 of ~134 units as redundant: C53 is the reused proof for the unchanged dependency set (CX-VERIFY-002). Canary rule from now on: only after a sync that changes the lock, scoped to the changed dependency closure (tools/run-round.sh --canary <crates>), never --all on an unchanged lock; reuse a same-lock canary. The killed run-round left a stale `held` in Handshake_Artifacts/WP-KERNEL-STUDIO/.disk-build-lock; the lock's stale-break rule clears it when the next waiter sees the holder (win pid 20736) dead: do not delete it by hand. run-round.sh already has CARGO_INCREMENTAL=0 (23b1bb7) and round-select.py ignores step-0 policy overlays (kernel, committed).
- Disks: D: (Toshiba 16 TB spinning disk) is the only build disk by Operator decision; C: (Samsung 870 QVO SATA SSD) is closed for builds (studio-env.sh refuses STUDIO_BUILD_DISK=ssd). The approved D: cleanup (batch A: 50 finished owners' target+tmp) ran 2026-10-23 11:52Z-14:44Z and is COMPLETE: 99 paths, 295.2 GiB, 263,938 files deleted; D: free 1220.8 -> 1497.7 GiB (research/cargo-cleanup-result-20261022.json). The D: lock dir had a stale `held` from the dead round; the lock's stale-break rule clears it when the next waiter sees holder and child dead. The three fresh C: lane stubs were deleted 2026-10-23 (C: lanes dir empty; every approved cleanup item is done). Re-check free space before any new build.
- HOOK (gameplan hooks STAY by Operator decision; they are there to help): the Operator's GLOBAL agent config runs ~/.claude/skills/gameplan/scripts/gameplan.py before every Bash/PowerShell command; a gated command (test run, cargo clean, the round runner) needs an active gameplan whose steps pass. It resolves the gameplan from a .gameplan-active pointer upward from the command's cwd, else the sole active WP gameplan (WP-KERNEL-012's, which binds to a worktree this machine lacks and carries WP-012 confirm steps: it can never pass for Studio). Studio's STUDIO-MODULE-BUILD (three typed checks) passes. RULES: (a) at lane start run `bash .GOV/docs_repo/wp-studio-module-work/tools/lane-gameplan.sh <lane-id-lowercase>` from the kernel (idempotent; gives the lane worktree its pointer bound to the lane HEAD); (b) every gated command runs with a leading `cd` into the Studio shared tree or the lane workspace, never from the kernel root (its pointer is WP-012's); (c) a hook refusal is reported verbatim to the Operator; never reshape the command, never create/confirm/edit gameplan steps, never touch the Operator's settings or WP-012's gameplan. Verified 2026-10-23 ~16:30Z by a sub-agent probe: passes from the shared tree and a lane worktree (and currently even from the kernel root).
- Spec: SPEC_CURRENT -> v02.222 (the other device published v02.221 reconciling both lineages on 2026-10-22 and v02.222 "Engine Profiles binding" on 2026-10-23 00:12Z; the roto contract STU-CMP-034 with declared 0..100 chatter bounds is in both). Next Studio spec edit starts a v02.223 bundle by the copy-first procedure (see the v02.217/v02.218 commits); always re-read SPEC_CURRENT.md first, the other device moves it. Residue folders master-spec-v02.218 and master-spec-v02.220 beside the current bundle await the Operator's removal approval.

- Parity at handover (research/parity-20261023-rollup.json; validated / on-origin-unvalidated / local): engine 55.8 / 56.0 / 56.0 % over 9 apps (54.9 on 2026-10-21); GUI pooled 52.8 / 53.2 / 56.5 % (37.0); GUI strict 20.8 / 22.4 / 24.7 % (14.3). After Effects GUI is 0 %; Premiere GUI 40 % only through the READY timeline panel; InDesign GUI 70 % through the hosted layout editor; no vendor oracle for Lightroom, Premiere, Audition, Figma, Affinity. Top gaps per app are in the post-mortem.

FIRST ASSIGNMENT: (0) check for live build processes and the D: lock state before dispatching anything that builds (the previous session's canary agent may still be running or may have died with that session; never stop a process you did not start); (1) confirm the state above by reading, not by building; (2) run `tools/lane-gameplan.sh` only for lanes you open (the nine READY lanes already have their pointers or do not need them); (3) steward sync for the EPS members (listed above), then ONE scoped canary on the new SYNC_SHA (the 7 ps crates, interop-vector-core, interop-vector and their dependents) on D: with nothing else building; (4) R39 on the then-tip, full scope by rule, as one independent validator (fresh agent; brief shape of validation/round-R37.json; expect many hours on the spinning disk; the validator runs from inside the export tree); it judges all nine READY lanes and re-judges every PASS lane whose crates changed since 1d139a6; (5) after R39: list the finished lanes' D: owner targets for the next approved deletion batch (rule P2); (6) only then new lanes from the post-mortem's top gaps, at most two GUI lanes at once, each with its lane pointer at lane start.

DO:
- Follow the Operator's instructions directly and fully. Do not send an instruction back to the Operator or ask them to re-confirm something they already decided.
- Follow the repo's authority files. If something is missing or contradictory, propose a fallback and get Operator approval before acting on it.
- Commit and push gov_kernel and the product branch regularly. gov_kernel: fetch, merge origin/gov_kernel only when it has new commits (never rebase, never force). Use `git -c core.fscache=false` for commits.
- Guard every push: produce the sha with &&, check it is non-empty, push only that sha; never chain a push after `;`; check the push exit code yourself (a pipe hides it). Commit messages through `-F -` heredocs or `-m`, never a temp file under $TMPDIR.
- Keep at most 5 builders + 1 validator + 1 steward. Fresh agents, one bounded task each, report only when finished or blocked. Read the "lessons" topic before dispatching anything.
- Builders follow the brief: one bundled run per push over the whole suites of every crate they change (rule 4a) plus hsk-studio-package when a Folio record field changes (rule 4b); commit per slice; push per green batch; loop cap two failed runs on the same failure.
- Report outcomes plainly, including your own and your agents' mistakes, without softening. Repair breakage with non-destructive actions and tell the Operator.
- Update HANDOFF.md at the end of every round and before any expected pause; keep ledger.json as the authority for lane state.

DO NOT:
- Do not create, confirm or edit gameplan steps, and do not edit the Operator's global config or WP-012's gameplan. The only gameplan action allowed is the per-lane pointer (`tools/lane-gameplan.sh <lane>`) at lane start. If the hook blocks a command, report it verbatim; never reshape the command to get past it.
- Do not delete anything (files, folders, branches, worktrees, caches, build targets) without explicit Operator approval in the same turn, except temp files your own session created. Deletions run only through a guarded script (allowed roots, forbidden names, allowed shapes, skip paths a running process references, rate-limited, logged, result JSON), never ad hoc.
- Never delete or force-push any remote branch. Never run git reset --hard, git clean, or worktree removal without explicit approval.
- Do not stop or kill processes your session did not start (your own agents' processes are yours).
- Do not invent new workflows, rules, tools or governance documents while the repo's authority covers the need; do not spend tokens on process instead of product.
- Do not touch WP-KERNEL-012 files. Do not integrate Studio into Handshake.
- Do not edit the Master Spec except on an explicit Operator request.
- Do not add any database other than SurrealDB. Permissive licences only; never read GPL/LGPL/MPL source. Never commit the proprietary Arial host font from the oracle corpus. Never classify the cargo registry cache (~/.cargo/registry/cache, registry/index) as removable.
- Do not build on C: (Operator 2026-10-23) until the Operator reopens it. Do not scan whole disks or other projects' folders; Studio's own cargo output is the only cleanup scope. The hooks stay as they are.
- Do not dispatch a steward sync or a canary while a validation round holds the D: lock; do not run more than one disk-heavy job per disk.
```

</topic>

<topic id="lessons" status="active" summary="What to do and not do; cargo, agents, reasoning tiers, parallelism (lead 2026-10-23, from the records)">

## Lessons (lead 2026-10-23, each from a recorded event)

### Cargo and disks
- The build disk decides the pace. D: is a spinning disk: a round's canary ran 12 h without reaching the union (R38) and died with the machine; builders queued 2 to 4 h behind it and gave up at the 7200 s lock default (SHELL-2 exit 75 after 13666 s). Builds belong on an SSD. If the Operator reopens C:, a round target plus a few lane targets without incremental state is roughly 300 to 400 GiB; the current 150 GB grant is too small for that.
- Incremental state was the single largest regrowing class of Studio cargo output (288 GiB purged 2026-10-21, regrown daily). CARGO_INCREMENTAL=0 is now set for builders (studio-env.sh) and rounds (run-round.sh 23b1bb7). Keep it.
- Every lane, probe and export owner gets a complete target bound to its own worktree and nothing retired it: about 95 owner dirs on D: for 6 active lanes, 1.1 TiB on D: on 2026-10-21. Rule P2: at lane PASS (commits on origin) or abandonment the lead lists the lane's <owner>/target and tmp in the next approved deletion batch. Rule P3: at most 6 live lane targets per disk.
- Deletions: only through the guarded script pattern (research/cargo-cleanup-proposal-20261022.json + cleanup2/3/4.ps1 in the lead scratchpad): allowed roots, forbidden names (steward-export, oracle-corpus, downloads, tools, app-inspect, engine-video-oracle, studio-round, studio-team-dev on D:, every active lane, anything outside WP-KERNEL-STUDIO), allowed shapes, skip paths a running process references, rate limit (150 files/s, 40 MiB/s), background priority, log + result JSON. Never scan disks with parallel walkers (the inventory agent hammered both disks); size from existing inventories and directory listings.
- A validation round and a cleanup on the same disk hammer it; a cleanup and builds on the SSD do too. One disk-heavy job per disk.
- Steward syncs need the disk (probe compile, canary): never dispatch one while a round holds D:; it waits its timeout and dies (two attempts lost).
- A canary costs hours here: ~134 units (crate x round feature set), no artifact sharing across feature sets, an export with fresh mtimes (full rebuild), the spinning disk. Run one only after a sync that changes the lock, scoped to the changed closure; reuse a same-lock canary (C53 reused for 5ab33c9 after the lead wasted 5 h on a redundant full one). Review the round-features matrix for combinations that prove nothing extra.
- The D: lock protocol: builders 900 s max hold, rounds 5400 s; over-hold only logs; stale locks break only when holder and child are provably dead. After a crash the `held` dir of the dead round is cleared by the next waiter.
- Windows Search indexing and Defender real-time scanning cover the build roots (no exclusions); uTorrent and the Codex app add disk ops. These are Operator settings; the lead only reports them.

### Agents (what cost time and tokens)
- Validators: ~280k tokens per round; a full-scope round costs hours of disk. R37 ran full scope for nothing because round-select counted step-0 config files as round inputs: fixed (kernel tools/round-select.py, self-test 25/25). Always scope rounds (CX-VAL-011) and reuse a same-sha canary.
- Three round FAILs had one cause class (package externals classifier on new Folio record fields) before it became brief rule 4b. When a validator FAILs twice on the same class, write the rule into the brief the same day.
- Loop caps work: the timeline lane burned 5 runs on a test defect (pointer aimed at the first egui frame); after the cap and a fresh diagnostics agent it shipped in one run. Give every builder "two failed runs on the same failure, then stop and report" and dispatch a fresh diagnostics agent instead of resuming the builder.
- Agents that wait on locks waste nothing but wall-clock; agents that scan or build concurrently waste disk. Dispatch builders when the disk can serve them, not when you have ideas.
- Lane worktrees + per-lane targets isolate lanes but multiply disk; three lanes in harness-view at once (timeline, shell-2, panels-1) also guarantee rebase conflicts in shell.rs/mod.rs. Prefer two GUI lanes at most at once, and sequence lanes that share a crate.
- Briefs must carry: the exact spec clauses to grep (never "read the module"), the predecessor lane file, the test shape, rules 4a/4b, PUSH SAFETY, the loop cap, the disk, and "report blockers verbatim". Every omission came back as a question or a wasted run.
- Agents cannot be stopped from inside their own session when the permission classifier denies it (steward attempt 2 ran unmonitored). Design agent commands so a stray process is harmless (bounded timeouts, own owner dir) rather than relying on stopping it.
- The lead's own errors this period: registry cache deleted on a misclassification (never again: forbidden in every cleanup script); a push chained after `;` and a $TMPDIR commit-message file git could not read; an over-scoped disk inventory agent (whole disks, three parallel walkers); a steward sync dispatched while a round held D:.

### Reasoning tiers (which agent for what)
- High reasoning (Opus-class): validators (adversarial reading, verdict attribution), stewards (lock/licence/edge decisions), builders on engine lanes with spec parameter tables (roto, effect stack, DNG), diagnostics agents on a failing test, parity auditors. Their cost is justified only with a tight brief and a loop cap.
- Low reasoning (Sonnet/Haiku-class): record keeping (ledger/lane-file updates from a report), lane-file drift fixes, dependency-request filing, reading a lane's log and summarising, directory listings and size reports from existing inventories, copying round results into records, rebasing a finished lane and re-running one green bundled run, writing START_LOG lines from a dictated outcome. None of these need judgment; all were done by the lead or an Opus agent this period and should not be.
- Never give a low-reasoning agent a verdict, a deletion, a licence call, a spec reading or a merge-conflict resolution in a shared file.

### Parallelism and speed
- The real limit is one build per disk. Parallel agents only help while they write code or wait; the moment two need the disk, they serialise. Plan the disk queue explicitly: round, then steward canary, then lanes by size.
- Cheapest throughput gains, in order: (1) SSD for builds (Operator decision); (2) scoped rounds and canary reuse (done); (3) no incremental state (done); (4) batch lane close-outs: one guarded push per lane after one green bundled run, then one scoped round for several lanes, instead of a round per lane; (5) sequence shared-crate lanes; (6) keep the steward's work (edges, pins, licence) ahead of the lanes that need it so no lane blocks on a sync.
- Do not hold the Operator's open decisions silently: the C: guard sat on the list for two days while builders queued on D:. Put every blocking decision in chat with the numbers the first time.
- A machine restart kills everything in flight: every agent must leave durable state (lane files, local commits, logs) after every slice, and every round must write its running record so a restart leaves a known state. That held; R38's 12 h did not.

</topic>

<topic id="state" status="active" summary="Verified state on 2026-10-23 after the machine restart">

## State (verified 2026-10-23 ~21:30Z)

- Machine restart ~08:40Z: R38 (e3953de) lost in its canary; steward sync lost (SYNC_SHA PENDING); all agents gone. Lane worktrees intact (see takeover prompt). D: cleanup batch A COMPLETE 14:44Z (99 paths, 295.2 GiB; D: free 1497.7 GiB); C: cleanup complete incl. the three lane stubs (C: free 528 GiB; C: lanes dir empty). Every approved cleanup item is done; result file research/cargo-cleanup-result-20261022.json.
- Hooks: kept by Operator decision. Studio satisfies them through its own gameplan: lane pointers set for the four parked lanes (lane-gameplan.sh), gated commands only from inside Studio trees. The settings file was briefly edited and restored byte-identically (backup settings.json.bak-20261023-gameplan-hooks beside it).
- 2026-10-23 evening: all four parked lanes finished and pushed (PANELS-1 347026e, EPS-FREE-FONTS 88edeb0, AE-EFFECT-STACK-1 5739eb5, HARNESS-SHELL-2 1709b748 = origin tip); nine READY lanes await R39; the EPS steward sync is owed first. Canary C54-5ab33c9-r2 stopped as redundant (C53 reused). Nothing of the previous session is running; the previous lead stopped on the Operator's instruction.
- Product origin tip 5ab33c9; 41 commits and +15,063 lines in the 24 h before the restart; 11 lanes pushed 2026-10-22 (7 PASS in R36/R37; 4 READY unvalidated; timeline READY).
- Validation: R36 @ a105766 291 PASS / 2 FAIL (remediated); R37 @ 1d139a6 297 PASS / 1 FAIL (remediated); R38 LOST. Next: R39 scoped.
- Spec v02.222 current (see takeover prompt). Post-mortem: research/post-mortem-20261023.md; parity updates: research/parity-20261023-{adobe-print,video-audio,figma-affinity-shell}.json.
- Older state notes kept for history: the 2026-10-21 takeover audit research/feature-completeness-20261021.json; DXF/EPS dependency rejections and the own-DXF-reader remediation (a105766); the EPS path restored on a vendored stet fork with free fonts (lane parked); lane-record drift (ledger is authority); the shared product worktree wt-studio-assets carries old Operator leftovers and 12 SAFETY stashes (leave them); remote branches origin/codex/handpick and origin/lane/smb-layout-ui-2-backup exist.

</topic>

<topic id="operator-items" status="active" summary="Decisions only the Operator can make">

## Waiting on the Operator

- (resolved 2026-10-23) Gameplan hooks stay; Studio passes them through its own gameplan pointers.
- Reopen C: (SSD) for builds with a grant large enough for a round target plus lane targets (roughly 300 to 400 GiB), or keep all builds on the spinning D: and accept rounds of 12 h or more.
- Windows Search and Defender exclusions for the build roots; uTorrent and the Codex app compete for D: seeks (report only).
- Spec residue: `git rm -r` of .GOV/spec/master-spec-v02.218 and master-spec-v02.220 (archive copies exist).
- PS-C28 mask variants; Photoshop colour setting HSK-gamma-1.0; Operator-attended Affinity test files; Figma .fig fixtures; Acrobat PDF/X oracle; os-encode legal check before commercial release; held-out InDesign oracle set.
- Spec deltas to publish: press-read/press-write and interop-affinity crate notes (STU-ARC-010), STU-AUD waveform/spectral editor clauses, STU-ASSET-004 reading for the Lightroom smart-rule preview; composer lane spec gaps; STU-CMP-034 parameter-table microtask upkeep.
- Optional: delete GitHub branch lane/smb-layout-ui-2-backup.

</topic>
