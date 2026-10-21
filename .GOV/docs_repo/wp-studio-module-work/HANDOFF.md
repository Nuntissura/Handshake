---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-10-21T01:35:48Z
wp: WP-KERNEL-STUDIO
authority: Handshake Codex, KERNEL_BUILDER_PROTOCOL, START_LOG.md, ledger.json, validation/*, builders/briefs/wave-parity-2-common.md, workflow/workflow-v3.json (this file is a projection; they win on conflict)
---

<topic id="takeover-prompt" status="active" summary="Prompt for the new dev leader agent">

## Takeover prompt (paste into a new session opened in `wt-gov-kernel`)

```text
You are the new DEV LEADER (KERNEL_BUILDER role, coordinator) for WP-KERNEL-STUDIO, taking over from a previous lead session. Studio is a Rust rewrite of the Adobe suite (Photoshop, Illustrator, InDesign, Lightroom Classic, Premiere, After Effects, Audition), Figma and Affinity, built as a standalone multi-crate module ("Studio Kit"). It is NOT integrated into Handshake until WP-KERNEL-012 is finished; WP-KERNEL-012 is worked on from another device and also pushes to gov_kernel.

You may use sub-agents to read, write, research and deliberate on blockers and failures. You write no product code yourself.

READ FIRST, in this order, and follow what you read (the repo's authority wins over your own preferences):
1. Kernel root CLAUDE.md / AGENTS.md.
2. .GOV/codex/Handshake_Codex_v1.4.md (the CX-GP gameplan rules are RETIRED; CX-VAL-011 scoped rounds was added 2026-10-20).
3. .GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md.
4. In .GOV/docs_repo/wp-studio-module-work/: this HANDOFF.md; START_LOG.md (every Operator decision and lead decision; read from 2026-10-14 onward closely); builders/briefs/wave-parity-2-common.md (binding builder brief, incl. "Build strategy (Operator 2026-10-20)", rule 4a and PUSH SAFETY); builders/DISPATCH_TEMPLATES.md; workflow/workflow-v3.json (adopted by the Operator 2026-10-20); ledger.json; validation/round-R34.json and round-R35.json; research/parity-matrix-20261020.json and research/d5-*.json.
Verify every claim against these files, git and command output. Label anything you could not verify UNVERIFIED.

FIRST ASSIGNMENT (report to the Operator before starting new build work):
A. Use sub-agents (read-only) to inspect the Studio Kit work delivered so far: product branch codex/handshake-studio-assets (repo Nuntissura/assets, subfolder handshake-studio-assets) history since 2026-10-14, the lane records in builders/lanes/, and the R34/R35 verdicts. Confirm what is pushed, what is validated (PASS in a round), what is built but not validated, and what exists only locally in lane worktrees under wt-studio-lanes/.
B. Check feature parity of Adobe Photoshop, Lightroom Classic, Illustrator, InDesign, Premiere Pro, After Effects, Audition, Figma and Affinity against the Studio Kit, using research/parity-matrix-20261020.json only as a starting point: verify rows against the code itself, and against real-app oracle results where they exist (research/d5-*.json; Photoshop/Illustrator/InDesign/After Effects only).
C. Report how feature complete Studio is, separately for (1) code/engine and (2) GUI (what a user can actually do in Studio's own viewer/panels), per app and overall, with the evidence level of each figure (validated / built-not-validated / partial / missing / unverified against the real app). Write the report to research/ as a dated file and give the Operator a plain-language summary in chat.

THEN TAKE OVER AS DEV LEADER: dispatch sub-agents that perform the work, record results, run validation rounds through the independent validator role, and push regularly. Good first candidates (you may reorder if you find more important work; say why):
- Steward registration (one batched sync per round) for the new DXF/EPS libraries (lane SMB-DXF-EPS-IMPORT, already pushed, deps approved in START_LOG 2026-10-21) and the two crate splits interop-fig and web (lane SMB-CRATE-SEAM-SPLITS-2: committed only locally in wt-studio-lanes/smb-crate-seam-splits-2 as fef312e and 67a7496; the web split is UNTESTED; also repoint two .config paths noted in that lane file).
- Validation round R36 (a fresh independent validator; earlier R36 attempt froze 8bb6f64 but never ran because the gameplan hook blocked it; re-freeze the current tip).
- The Operator-requested roto spec update ("update the spec so rotoscope can be feature complete"): NOT DONE. Spec is still v02.216; STU-CMP-034/STU-CMP-100 still declare assisted rotoscoping a gap. A new spec version v02.217 must be built by the repo's own version procedure (see the v02.215 -> v02.216 commits) and committed/pushed. The already-pushed lane SMB-AE-ROTO-TIMEREMAP implements assisted roto and cannot pass validation on that slice until the spec allows it.

DO:
- Follow the Operator's instructions directly and fully. Do not send an instruction back to the Operator or ask them to re-confirm something they already decided.
- Follow the repo's authority files. If something is missing or contradictory, propose a fallback and get Operator approval before acting on it.
- Commit and push gov_kernel and the product branch regularly. gov_kernel: fetch, merge origin/gov_kernel only when it has new commits (never rebase, never force). Use `git -c core.fscache=false` for commits (fscache caused index write failures).
- Guard every push: produce the sha with &&, check it is non-empty, push only that sha; never chain a push after `;`.
- Keep at most about 5 builders plus 1 validator plus 1 steward running at once. Use fresh agents with one bounded task each; do not resume agents with very large contexts. Agents report only when finished or blocked.
- Builders follow the brief: one bundled build per push over the whole test suites of every crate they change; commit per slice; push per green batch.
- Report outcomes plainly, including your own and your agents' mistakes, without softening. If you or an agent breaks something, repair it immediately with a non-destructive action and tell the Operator.
- Update HANDOFF.md at the end of every round and before any expected pause.

DO NOT:
- Do not use the gameplan skill or create, check or confirm gameplans. The Operator retired it everywhere on 2026-10-20 and the hook was removed from settings on 2026-10-21. Ignore old gameplan pointer files; if a hook still blocks a command, report it to the Operator; never reshape a command to get past it.
- Do not delete anything (files, folders, branches, worktrees, caches, build targets) without explicit Operator approval in the same turn, except temp files your own session created. The C: build-target eviction agent was stopped: nothing may be evicted without new Operator approval.
- Never delete or force-push any remote branch. Never run git reset --hard, git clean, or worktree removal without explicit approval.
- Do not stop or kill processes your session did not start.
- Do not invent new workflows, rules, tools or governance documents while the repo's authority covers the need; do not spend tokens on process instead of product.
- Do not touch WP-KERNEL-012 files. Do not integrate Studio into Handshake.
- Do not edit the Master Spec except for the Operator-requested roto update or another explicit Operator request.
- Do not add any database other than SurrealDB. Permissive licences only; never read GPL/LGPL/MPL source. Never commit the proprietary Arial host font from the oracle corpus.
- Do not build on C: outside the granted C:/HandshakeBuild/WP-KERNEL-STUDIO area (150 GB grant; it is currently over its 140 GiB guard, so builders fall back to D:).
```

</topic>

<topic id="state" status="active" summary="Verified state on 2026-10-21">

## State (verified 2026-10-21, takeover audit by the new lead)

- Takeover audit written: `research/feature-completeness-20261021.json` (per-app engine and GUI completeness with evidence levels) over the eight read-only audit files in `research/audit-20261021/` (git inventory, validation state, nine parity verifications, GUI surfaces, cargo disk inventory). Read it before any new build work.
- gov_kernel: see git log (this handoff commit). Untracked only: `refund-claim-20261021.md` at the kernel root (Operator's document, not part of the repo).
- Product branch codex/handshake-studio-assets: tip e64eacd, 705 commits since 2026-10-14, 79 commits and 28 crates since the R35 candidate 237256f. The branch was deleted on GitHub on 2026-10-21 by an unguarded lane push, restored at 8bb6f64, then four finished lanes were rebased and pushed on top: 3432d5c (AE roto/time remap), 85b3446 (DXF/EPS import), 2e0f53e (type ff ligature / InDesign 93 of 99 oracle matches), e64eacd (Photoshop oracle parity 2, 489 of 588 matches). Patch-ids verified by the lead: the pre-rebase shas in those lane files are exact copies of the pushed commits.
- Not pushed: SMB-CRATE-SEAM-SPLITS-2 (fef312e fig split, 67a7496 web split UNTESTED) in wt-studio-lanes/smb-crate-seam-splits-2; one empty steward probe commit d44a708 in wt-studio-lanes/smb-steward-lock-release-bug (not meant to be pushed).
- Uncommitted source edits outside the shared tree (decision needed): wt-studio-lanes/smb-premiere-color-titles composite/src/composition.rs (PASS lane leftover); wt-studio-lanes/smb-raw-cleanroom two untracked develop/src/raw_cr drafts superseded by SMB-RAW-CLEANROOM-2. Six lane trees carry Cargo.lock-only edits (by rule).
- Validation: 274 PASS lanes, 255 with stale proof (crate changed since their round); 19 READY_FOR_VALIDATION, 1 BLOCKED. Last finalized round R35 at 237256f (256 PASS, 2 FAIL, both remediated in 3b73143 and READY again). R36 never ran and must be full scope (Cargo.lock, .config and tools/run-round.sh changed since R35).
- Lockfile gap: Cargo.lock at e64eacd has no entries for the dxf/stet* optional deps declared by interop-vector-core; the steward sync (deps approved 2026-10-21) must land before any locked round build.
- Known regression candidate for R36: harness fig_paints_2 bold text p99 5.700324 > pinned 5.698 at 6585ff0; suspects 30ea18e, 3d5fbda, then a95f819, d69a277, f588e45 (pigment/render-cpu).
- Lane-record drift (ledger is authority): lane files for AE-ROTO-TIMEREMAP, DXF-EPS-IMPORT, TYPE-FF-LIGATURE, PS-ORACLE-PARITY-2 still show null/BLOCKED and pre-rebase shas; ACCORD-COMPOSITION-PREFIX cites "see git log"; AU-WAVEFORM-EDITOR cites 8411820 (pre-rebase copy of 557ac06).
- Spec: v02.216. The Operator-requested roto spec update (v02.217 specifying assisted rotoscoping; STU-CMP-034/STU-CMP-100 still declare it a gap) is NOT done.
- Shared product worktree wt-studio-assets (HEAD 8bb6f64, 25 behind origin) has old uncommitted leftovers not made by this session (Cargo.lock patch, two stale test edits from 2026-10-15, two old press_desc JSON notes) and 12 SAFETY stashes; leave them unless the Operator decides.
- C: build area over its 140 GiB guard; eviction stopped pending Operator approval. Measured candidates: `research/audit-20261021/cargo-disk-inventory.json`.
- Remote branches outside the Studio branch: origin/codex/handpick (7511afa), origin/lane/smb-layout-ui-2-backup (f3c2a32).

</topic>

<topic id="operator-items" status="active" summary="Decisions only the Operator can make">

## Waiting on the Operator

- Eviction of idle lane build targets on C:.
- PS-C28 mask variants (rule measured; enabling them changes another lane's refusal tests).
- Photoshop colour setting HSK-gamma-1.0 (gamma oracle pass); Operator-attended Affinity test files; Figma .fig fixtures; Acrobat PDF/X oracle; os-encode legal check before commercial release.
- Spec deltas to publish: press-read/press-write and interop-affinity crate notes (STU-ARC-010), STU-AUD waveform/spectral editor clauses, STU-ASSET-004 reading for the Lightroom smart-rule preview.
- Optional: delete GitHub branch lane/smb-layout-ui-2-backup.

</topic>
