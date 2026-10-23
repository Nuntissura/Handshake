---
file_id: WP-STUDIO-POST-MORTEM-20261023
file_kind: post_mortem
updated_at: 2026-10-23T12:40:00Z
wp: WP-KERNEL-STUDIO
authority: research/parity-20261023-rollup.json and the three research/parity-20261023-*.json files hold the figures; ledger.json, validation/*, START_LOG.md hold the events. This file is the operator-facing projection.
---

<topic id="scope" status="active" summary="Period, question, method">

## Scope

Period: the lead session from the takeover on 2026-10-21 to the machine restart on 2026-10-23 ~08:40Z. Question: how feature-complete is Studio against Photoshop, Lightroom Classic, Illustrator, InDesign, Premiere Pro, After Effects, Audition, Figma and Affinity, and why did progress feel slow. Method: three read-only auditors re-verified every row that changed since the 2026-10-21 audit against the code and tests on origin 5ab33c9 and the round verdicts (R36 @a105766, R37 @1d139a6), using the 2026-10-21 formulas and row sets. No build or test was run for this report. Figures are given in three tiers: VALIDATED (PASS in a finished round), UNVALIDATED (on origin, never judged: R38 was lost), LOCAL (commits only in lane worktrees, never run).

</topic>

<topic id="parity" status="active" summary="Feature parity per app and overall">

## Feature parity

Engine = code capability by vendor-defined rows (validated 100 %, built-not-validated 50 %, partial 25 %). GUI pooled = workflows a user can do in Studio's own window. GUI strict = vendor UI rows doable with the same semantics. Values: 2026-10-21 → validated / unvalidated / local.

| App | Engine % | GUI pooled % | Top remaining gap |
|---|---|---|---|
| Photoshop | 47.2 → 47.2 / 47.2 / 47.2 | 61.1 → 61.1 / 61.1 / 61.1 | soft proofing (STU-COL-022) |
| Lightroom Classic | 57.9 → 57.9 / 57.9 / 57.9 | 65.4 → 69.2 / 69.2 / 69.2 | healing (STU-RAW-124); no catalog |
| Illustrator | 64.8 → 64.8 / 64.8 / 64.8 | 47.1 → 47.1 / 47.1 / 64.7 | area type (STU-TYP-114) |
| InDesign | 54.0 → 54.0 / 54.0 / 54.0 | 0 → 70.0 / 70.0 / 70.0 | vertical/CJK text (STU-TYP-137/138) |
| Figma | 60.9 → 60.9 / 60.9 / 60.9 | 14.3 → 28.6 / 28.6 / 28.6 | design editing UI absent |
| Premiere Pro | 56.0 → 56.0 / 57.0 / 57.0 | 0 → 40.0 / 45.0 / 45.0 | multicam (STU-VID-033); ripple/roll/slip/slide trimming engine-only |
| After Effects | 58.3 → 62.5 / 64.6 / 64.6 | 0 → 0 / 0 / 0 | composition/keyframe timeline, graph editor (STU-SHL-091, STU-MOT-050) |
| Audition | 81.8 → 86.4 / 86.4 / 86.4 | 25.0 → 60.0 / 60.0 / 65.0 | multitrack clip/automation editor |
| Affinity | 43.1 → 43.8 / 43.8 / 43.8 | 60.0 → 66.7 / 66.7 / 66.7 | EPS import regressed on origin (fix is LOCAL) |

| Overall | 2026-10-21 | Validated | Unvalidated | Local |
|---|---|---|---|---|
| Engine, 9 apps | 54.9 | 55.8 | 56.0 | 56.0 |
| Engine, 8 apps (no Affinity) | 59.1 | 60.0 | 60.3 | 60.3 |
| GUI pooled (108 workflows) | 37.0 | 52.8 | 53.2 | 56.5 |
| GUI strict (308 rows) | 14.3 | 20.8 | 22.4 | 24.7 |

Reading: the period moved the GUI, not the engine. No engine row changed level in the four print apps: every lane closed one named part of a row but each row still lacks another vendor-defined part. The GUI gain comes from the shell (menus, file dialogs, panel hosting: 28 of 38 proven surfaces now reachable from the window, was 22; 7 more only in the local panels lane; 3 unreachable), the hosted layout editor (InDesign 0 → 70 %), the first timeline panel (Premiere 0 → 40 %) and the audio panels (Audition 25 → 60 %). After Effects has no GUI at all: the clip timeline does not serve compositions. Oracles exist for Photoshop, Illustrator, InDesign and After Effects only; everything about Lightroom, Premiere, Audition, Figma and Affinity behaviour is UNVERIFIED against the real app, and the InDesign composer is fitted to the same oracle it is scored on (F-R36-4). Every PASS on composite, develop, folio, interop-psd, pigment and harness-view is stale until R39.

</topic>

<topic id="delivered" status="active" summary="What the period delivered">

## Delivered 2026-10-21 to 2026-10-23

- 41 product commits, 141 files, +15,063 lines on origin in the 24 h before the restart; 11 lanes pushed on 2026-10-22; 7 PASS in R36/R37 (own DXF reader replacing a licence-blocked crate, two Photoshop oracle-parity lanes, crate seam splits, TIFF export, the shell with menus/keys/dialogs, InDesign composer wiring), 5 READY on origin unvalidated (roto wiring, fill layers, DNG writer, AE render queue, timeline panel).
- Spec: v02.217 (assisted rotoscoping contract with a 23-row parameter table) and v02.218 (declared chatter bounds), reconciled by the other device into v02.221, now v02.222.
- Validation: R36 (291 PASS / 2 FAIL, both remediated), R37 (297 PASS / 1 FAIL, remediated). R38 lost.
- Infrastructure: round selection no longer counts step-0 policy overlays (R37 would have been 30 of 49 crates instead of full); incremental compilation state off for builders and rounds; 6 h lock wait for builders; per-lane target retention rules P2/P3.
- Disk: 339 GiB purged 2026-10-21; 21 dead C: lane targets and the 99 GiB stale C: debug target deleted 2026-10-22/23 (C: free 326 → 528 GiB); the approved D: batch (50 finished owners, 210 GiB known plus regrown incremental) started 2026-10-23 11:52Z.
- Four lanes parked with local commits only (shell-2: descriptors, HELP menu, vendor-name fix; panels-1: 13 surfaces hosted; effect-stack-1: AE effect stack with 7 distort/generate kernels; eps-free-fonts: stet fork as 7 members with TeX Gyre, STIX Two and DejaVu fonts, EPS worker restored).

</topic>

<topic id="what-went-wrong" status="active" summary="Root causes of slow progress, with evidence">

## What went wrong and why

1. **The build disk is a spinning disk.** One build per disk, and D: (Toshiba 16 TB HDD) was the only build disk after C: (SATA SSD) tripped its 140 GiB guard and was then closed by decision. Measured: D: at 32 queued requests, 200 ms per transfer, 2 MB/s. R38's canary ran 12 h without reaching the union and died with the machine; every builder queued 2 to 4 h behind rounds, three gave up at the 7200 s lock default. This is the dominant cause. (START_LOG 2026-10-22/23; lane files lock_wait_s.)
2. **Rounds ran wider than needed.** R37 was forced to full scope because the selector counted step-0 config overlays as round inputs (F-R37-2): hours of disk for nothing. Fixed in the kernel; but the fix to run-round.sh makes the next round full scope again by rule.
3. **The same bug class failed three rounds** (package externals classifier on new Folio record fields) before it became brief rule 4b. Each FAIL cost a lane round-trip.
4. **Steward syncs were dispatched while rounds held the disk**; two attempts waited out their timeouts and died; SYNC_SHA is still pending.
5. **Lead errors**: the cargo registry cache deleted on a misclassification (one steward attempt redone); an over-scoped disk inventory agent that walked both disks with three parallel walkers while a round ran (stopped by the Operator's complaint); a push chained after `;` and a commit-message temp file git could not read; the C: guard decision left on the Operator's list for two days instead of forced into chat with numbers.
6. **A test defect cost five runs** (timeline drag tests aimed at the first egui frame); the loop cap and a fresh diagnostics agent fixed it in one run. Agents resumed into their own failing loops do not.
7. **Three lanes edited the same shell files at once** (timeline, shell-2, panels-1): guaranteed rebase conflicts in shell.rs/mod.rs.
8. **External blocks**: the deleted GitHub branch (2026-10-21) held all pushes; an Operator app exhausted local ports and broke sccache (dropped); the global gameplan hook returned on 2026-10-23 and hard-blocks every test run and round until disabled; Windows Search and Defender scan the build roots with no exclusions.
9. **Tokens**: ~280k per validator round, ~400k per builder lane, 220k for a diagnostics agent, 8 audit agents plus 3 parity auditors; the killed inventory agent produced only the C: walk. Record keeping and lane-file upkeep were done by Opus agents or the lead; they belong to a low-reasoning tier.

</topic>

<topic id="what-to-change" status="active" summary="Decisions and rules going forward (details in HANDOFF.md lessons)">

## What to change

- Builds on the SSD, exports and results on the HDD; grant large enough for a round target plus lane targets (Operator decision).
- One disk-heavy job per disk: round, then steward canary, then lanes by size; never a steward sync while a round holds the disk.
- Scoped rounds and canary reuse always; batch several lanes per round.
- Retire lane targets at lane close (P2), cap live lane targets (P3), no incremental state (P1).
- Loop cap of two failed runs per failure; fresh diagnostics agent instead of resuming the builder.
- At most two GUI lanes at once; sequence lanes that share a crate.
- Low-reasoning agents for records, lane-file upkeep, dependency filing, rebases of finished lanes, log summaries; high-reasoning only for verdicts, licence and edge decisions, spec-bound engine work, diagnostics, parity audits.
- Every blocking Operator decision goes to chat with numbers the first time.
- Deletions only through the guarded script pattern; never a disk scan with parallel walkers.
- The full "lessons" list with evidence is in HANDOFF.md.

</topic>

<topic id="open-decisions" status="active" summary="Operator decisions outstanding at handover">

## Open decisions

Disable the two gameplan hook entries in the global settings for this work; reopen C: for builds with a larger grant (or accept 12 h rounds on D:); Search/Defender exclusions for the build roots; spec residue folders v02.218/v02.220 removal; the vendor-oracle items (PS gamma, Affinity files, Figma fixtures, Acrobat PDF/X, held-out InDesign set); the spec deltas listed in HANDOFF.md.

</topic>
