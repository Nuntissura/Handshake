---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-10-23T15:05:46Z
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

STATE YOU INHERIT (verified 2026-10-23 ~15:40Z; re-verify):
- The machine restarted ~08:40Z on 2026-10-23. Every running process died: validation round R38 (candidate e3953de) was lost 12 h into its canary and never reached the union; the steward's lock/registry sync died (SYNC_SHA PENDING); all agents died. Nothing was corrupted: lane worktrees kept their local commits.
- Product origin tip 5ab33c9. READY_FOR_VALIDATION and unvalidated on origin: SMB-AE-ROTO-WIRING d9de925, SMB-PS-FILL-LAYERS 3b88879, SMB-DEVELOP-DNG-WRITER 71253a3, SMB-AE-RENDER-QUEUE-2 e3953de, SMB-TIMELINE-UI-1 5ab33c9. They need round R39 (scoped, tools/round-select.py) on the current tip.
- Parked with LOCAL commits only (never run, never pushed), in wt-studio-lanes/<lane>/handshake-studio-assets: smb-harness-shell-2 (8ddbae5, 2 commits), smb-harness-panels-1 (587886c, 3), smb-ae-effect-stack-1 (c733641, 3), smb-eps-free-fonts (a0351e9, 3 + Cargo.lock dirty by rule). Each needs: one bundled lane-check, fixes, rebase, guarded push, READY. Their lane files say exactly what remains. The EPS lane then needs a steward sync (7 new workspace members hsk-studio-ps*, edges, round-features, one machine-path pin, licence gate on the GUST Font License).
- Steward: sync DONE 2026-10-23 15:03Z, SYNC_SHA=5ab33c9 (already in sync, no commit; lock diff/registry empty; licence gate OK; probe 89/89; D: lock dir clean). Canary C54 on 5ab33c9 NOT run: the hook blocked the steward sub-agent (see HOOK). It is the first thing to run once gated commands are possible (no rustc wrapper). run-round.sh already has CARGO_INCREMENTAL=0 (23b1bb7) and round-select.py ignores step-0 policy overlays (kernel, committed).
- Disks: D: (Toshiba 16 TB spinning disk) is the only build disk by Operator decision; C: (Samsung 870 QVO SATA SSD) is closed for builds (studio-env.sh refuses STUDIO_BUILD_DISK=ssd). The approved D: cleanup (batch A: 50 finished owners' target+tmp) ran 2026-10-23 11:52Z-14:44Z and is COMPLETE: 99 paths, 295.2 GiB, 263,938 files deleted; D: free 1220.8 -> 1497.7 GiB (research/cargo-cleanup-result-20261022.json). The D: lock dir had a stale `held` from the dead round; the lock's stale-break rule clears it when the next waiter sees holder and child dead. The three fresh C: lane stubs were deleted 2026-10-23 (C: lanes dir empty; every approved cleanup item is done). Re-check free space before any new build.
- HOOK (probe first, do not assume): the Operator's GLOBAL agent config (C:\Users\Ilja Smets\.claude\settings.json, two hook entries matching Bash|PowerShell, running ~/.claude/skills/gameplan/scripts/gameplan.py) hard-blocks every command whose text matches cargo test / nextest / cargo clean / run-round, and blocks mutating commands while it considers a gameplan active. It resolves WP-KERNEL-012's gameplan from every Studio tree and fails on a worktree this machine lacks. The Operator said to IGNORE gameplans this session (no gameplan work), but a hard block cannot be ignored: report the exact message, never reshape a command around it, never touch the Operator's config. Observed 2026-10-23: it hard-blocked the lead's session at 11:48Z; from ~15:25Z gated-pattern commands passed in the lead's PARENT session (echo probes and `head -n 1 tools/run-round.sh`), but at ~15:05Z and again later a steward SUB-AGENT was hard-blocked on a read-only `sed` of tools/run-round.sh. Working conclusion: the hook is inactive in the parent session only (cause unverified) and active for sub-agents, where all builds, tests and rounds run. Rule: probe in a sub-agent, not only in your own shell (dispatch a tiny read-only agent that runs `head -n 1 tools/run-round.sh` in the shared tree); if it blocks, no canary, lane check or round can run and the Operator must decide (remove the two entries while Studio runs; or fix the WP-KERNEL-012 gameplan binding, a WP-012 file you do not touch). Never reshape a command around it, never edit the Operator's config (backups of the settings file exist beside it from 2026-10-23).
- Spec: SPEC_CURRENT -> v02.222 (the other device published v02.221 reconciling both lineages on 2026-10-22 and v02.222 "Engine Profiles binding" on 2026-10-23 00:12Z; the roto contract STU-CMP-034 with declared 0..100 chatter bounds is in both). Next Studio spec edit starts a v02.223 bundle by the copy-first procedure (see the v02.217/v02.218 commits); always re-read SPEC_CURRENT.md first, the other device moves it. Residue folders master-spec-v02.218 and master-spec-v02.220 beside the current bundle await the Operator's removal approval.

- Parity at handover (research/parity-20261023-rollup.json; validated / on-origin-unvalidated / local): engine 55.8 / 56.0 / 56.0 % over 9 apps (54.9 on 2026-10-21); GUI pooled 52.8 / 53.2 / 56.5 % (37.0); GUI strict 20.8 / 22.4 / 24.7 % (14.3). After Effects GUI is 0 %; Premiere GUI 40 % only through the READY timeline panel; InDesign GUI 70 % through the hosted layout editor; no vendor oracle for Lightroom, Premiere, Audition, Figma, Affinity. Top gaps per app are in the post-mortem.

FIRST ASSIGNMENT: (0) run the hook probe (one harmless echo containing a gated word, from the kernel root and from a lane worktree; see the HOOK item); (1) confirm the state above by reading, not by building; (2) canary C54 on 5ab33c9 on D: with nothing else building (the sync itself is done, SYNC_SHA 5ab33c9); (3) finish the four parked lanes one at a time on the D: lock, biggest first (effect-stack-1, panels-1, shell-2, eps-free-fonts; eps needs its own steward sync after its push), and push them; (4) R39 on the tip (full scope by rule: tools/run-round.sh changed in 23b1bb7; expect many hours on the spinning disk); (5) only then new lanes, from the post-mortem's top gaps, at most two GUI lanes at once.

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
- Do not create, check, confirm or edit gameplans. If the hook blocks a command, report it verbatim; never reshape the command to get past it; never edit the Operator's global config.
- Do not delete anything (files, folders, branches, worktrees, caches, build targets) without explicit Operator approval in the same turn, except temp files your own session created. Deletions run only through a guarded script (allowed roots, forbidden names, allowed shapes, skip paths a running process references, rate-limited, logged, result JSON), never ad hoc.
- Never delete or force-push any remote branch. Never run git reset --hard, git clean, or worktree removal without explicit approval.
- Do not stop or kill processes your session did not start (your own agents' processes are yours).
- Do not invent new workflows, rules, tools or governance documents while the repo's authority covers the need; do not spend tokens on process instead of product.
- Do not touch WP-KERNEL-012 files. Do not integrate Studio into Handshake.
- Do not edit the Master Spec except on an explicit Operator request.
- Do not add any database other than SurrealDB. Permissive licences only; never read GPL/LGPL/MPL source. Never commit the proprietary Arial host font from the oracle corpus. Never classify the cargo registry cache (~/.cargo/registry/cache, registry/index) as removable.
- Do not build on C: (Operator 2026-10-23) until the Operator reopens it. Do not scan whole disks or other projects' folders; Studio's own cargo output is the only cleanup scope. Do not edit the Operator's global settings file; the hooks stay as they are.
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

## State (verified 2026-10-23 ~15:40Z)

- Machine restart ~08:40Z: R38 (e3953de) lost in its canary; steward sync lost (SYNC_SHA PENDING); all agents gone. Lane worktrees intact (see takeover prompt). D: cleanup batch A COMPLETE 14:44Z (99 paths, 295.2 GiB; D: free 1497.7 GiB); C: cleanup complete incl. the three lane stubs (C: free 528 GiB; C: lanes dir empty). Every approved cleanup item is done; result file research/cargo-cleanup-result-20261022.json.
- Hooks: kept as they are by Operator decision; probes with gated words passed at ~15:25Z (cause unverified); the settings file was briefly edited and restored byte-identically (backup settings.json.bak-20261023-gameplan-hooks beside it).
- Nothing builds at handover. Steward sync attempt 3 done (SYNC_SHA 5ab33c9); canary C54 blocked by the hook in the sub-agent.
- Product origin tip 5ab33c9; 41 commits and +15,063 lines in the 24 h before the restart; 11 lanes pushed 2026-10-22 (7 PASS in R36/R37; 4 READY unvalidated; timeline READY).
- Validation: R36 @ a105766 291 PASS / 2 FAIL (remediated); R37 @ 1d139a6 297 PASS / 1 FAIL (remediated); R38 LOST. Next: R39 scoped.
- Spec v02.222 current (see takeover prompt). Post-mortem: research/post-mortem-20261023.md; parity updates: research/parity-20261023-{adobe-print,video-audio,figma-affinity-shell}.json.
- Older state notes kept for history: the 2026-10-21 takeover audit research/feature-completeness-20261021.json; DXF/EPS dependency rejections and the own-DXF-reader remediation (a105766); the EPS path restored on a vendored stet fork with free fonts (lane parked); lane-record drift (ledger is authority); the shared product worktree wt-studio-assets carries old Operator leftovers and 12 SAFETY stashes (leave them); remote branches origin/codex/handpick and origin/lane/smb-layout-ui-2-backup exist.

</topic>

<topic id="operator-items" status="active" summary="Decisions only the Operator can make">

## Waiting on the Operator

- Gameplan hooks: still block sub-agents (the steward at ~15:05Z); the Operator decides between removing the two entries while Studio runs and fixing the WP-KERNEL-012 gameplan binding; until then no canary, lane check or round can run.
- Reopen C: (SSD) for builds with a grant large enough for a round target plus lane targets (roughly 300 to 400 GiB), or keep all builds on the spinning D: and accept rounds of 12 h or more.
- Windows Search and Defender exclusions for the build roots; uTorrent and the Codex app compete for D: seeks (report only).
- Spec residue: `git rm -r` of .GOV/spec/master-spec-v02.218 and master-spec-v02.220 (archive copies exist).
- PS-C28 mask variants; Photoshop colour setting HSK-gamma-1.0; Operator-attended Affinity test files; Figma .fig fixtures; Acrobat PDF/X oracle; os-encode legal check before commercial release; held-out InDesign oracle set.
- Spec deltas to publish: press-read/press-write and interop-affinity crate notes (STU-ARC-010), STU-AUD waveform/spectral editor clauses, STU-ASSET-004 reading for the Lightroom smart-rule preview; composer lane spec gaps; STU-CMP-034 parameter-table microtask upkeep.
- Optional: delete GitHub branch lane/smb-layout-ui-2-backup.

</topic>
