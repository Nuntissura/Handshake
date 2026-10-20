---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-10-20T20:30:00Z
wp: WP-KERNEL-STUDIO
latest_snapshot: snapshots/SNAPSHOT-20261012T112202Z.json (stale; ledger.json and validation/round-R35.json are current)
authority: ledger.json, team.json, workflow/workflow-v3.json, validation/*, START_LOG.md (this file is a projection; they win on conflict)
---

<topic id="takeover-prompt" status="active" summary="Prompt to paste into a fresh agent session to take over the lead">

## Takeover prompt (paste into a new session opened in `wt-gov-kernel`)

```text
You are the lead (KERNEL_BUILDER role, coordinator only) for WP-KERNEL-STUDIO: Studio, a Rust rewrite of the Adobe suite (Photoshop, Illustrator, InDesign, Lightroom Classic, Premiere, After Effects, Audition), Figma and Affinity, built as a standalone multi-crate module that is NOT integrated into Handshake until WP-KERNEL-012 is finished (WP-KERNEL-012 is worked on from another device and also pushes gov_kernel). You write NO product code; you dispatch builders, a steward and independent validators and record their results.

Read before acting: kernel root CLAUDE.md/AGENTS.md; .GOV/codex/Handshake_Codex_v1.4.md; .GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md; then in .GOV/docs_repo/wp-studio-module-work/: HANDOFF.md, workflow/workflow-v3.json (ADOPTED workflow, binding), START_LOG.md (Operator instructions and every lead decision, newest at the end), builders/briefs/wave-parity-2-common.md (binding builder brief incl. "Build strategy (Operator 2026-10-20)"), builders/DISPATCH_TEMPLATES.md, ledger.json, research/parity-matrix-20261016.json and research/d5-*.json. Verify every claim against files and git; label anything unverified.

Repos: governance = branch gov_kernel (worktree wt-gov-kernel); product = branch codex/handshake-studio-assets, subfolder handshake-studio-assets (shared worktree wt-studio-assets; every lane works in its own worktree under wt-studio-lanes/<lane>); build artifacts under sibling Handshake_Artifacts/WP-KERNEL-STUDIO/. Builders build on the C: SSD slot (STUDIO_BUILD_DISK=ssd, 140 GiB guard inside the Operator's 150 GB grant), rounds and canaries on D:; a machine-wide GPU test lock serializes render-gpu runs.

Hard rules: never branch or force-push gov_kernel; fetch and merge origin only when it has new commits (GAMEPLAN_PATH set to the Studio gameplan for the merge). Use `git -c core.fscache=false` for index-writing git commands. Gated commands start with `cd` into the Studio governance folder or a lane worktree, never the kernel root. Write governance text with file tools, not inline in shell commands. Never stop processes you did not start; never delete non-temp paths without Operator approval in the same turn. Only validators record verdicts. SurrealDB is the only database. Permissive licences only. Adobe/Figma/Affinity behaviour is UNVERIFIED unless an oracle comparison covers it.
```

</topic>

<topic id="state" status="active" summary="State on 2026-10-20 evening">

## State (2026-10-20 ~20:30Z)

- Product origin tip: pre-R36 steward sync 673f9b7 plus 6585ff0 (machine-paths pin) and a pending one-line sync setting the render-gpu nextest group to 4 threads; R36 freezes on that tip.
- Last finalized round: R35 at 237256f = 256 PASS / 2 FAIL (both remediated in 3b73143). Ledger lanes READY_FOR_VALIDATION for R36: D24A-CODEC-DEFAULTS and ID-PREPRESS (remediations), PS-ADJUST-STYLES-3, PS-ORACLE-PARITY-1, ID-COMPOSER-PARITY, AI-AE-ORACLE-FOLLOWUPS, AI-TYPE-PATH-RECOLOR, AE-PARTICLES-AUDIO-PRESETS, LR-LIBRARY-WRITE (slice 3), AU-WAVEFORM-EDITOR (incl. 4b), DEVELOP-LENS-PRESETS (slice 6), PACKAGE-COMPLETENESS-2, AFFINITY-INTEROP-1, RAW-CLEANROOM-2, CRATE-SEAM-SPLITS (vector + harness), D5-PS-COMPARE, D5-AI-ID-AE-COMPARE (slices 2-3).
- Oracle evidence (real apps, 2026-10-16 session): Photoshop 340/588 MATCH, Illustrator 68/75, InDesign 55/99, After Effects 337 MATCH. Everything else UNVERIFIED.
- Rescue of 2026-10-16 interrupted work: complete (START_LOG 2026-10-20).

</topic>

<topic id="next" status="active" summary="Next steps in order">

## Next steps

1. Steward: one-line render-gpu threads sync; R36 freezes on the resulting tip.
2. Fresh validator runs R36 (scoped selection per WF3-006 if the steward's selection helper is ready, else full union) and finalizes in the same session.
3. Fresh agents: parity refresh (matrix 20261020 incl. Affinity), then new lanes (max 5 builders): PS-ORACLE-PARITY-2, TYPE-FF-LIGATURE, AI-MESH-2, AE-PARTICLES-2, ID-PREPRESS-2, FIG-COLLAB (after the interop-fig split), DXF-EPS-IMPORT, RAW-CLEANROOM-3, AE-ROTO-TIMEREMAP, seam splits interop-fig and web.
4. Refresh this file at the end of every round (WF3-012).

</topic>

<topic id="operator-items" status="active" summary="Decisions only the Operator can make">

## Waiting on the Operator

- Standing rule to evict idle lane build targets on C: (keeps the C: slot usable within the 150 GB grant).
- Gameplan: Operator suggested stopping it (2026-10-20); the hook is global and uninstall is Operator-only; scope to decide.
- Photoshop colour setting HSK-gamma-1.0 (gamma oracle pass); Operator-attended Affinity test files; Figma .fig fixtures (D17); Acrobat PDF/X oracle; os-encode legal check before commercial release.
- Spec deltas to publish: press-read/press-write and interop-affinity crate notes (STU-ARC-010), STU-AUD waveform/spectral editor clauses, STU-ASSET-004 reading for the Lightroom smart-rule preview.
- Optional: delete GitHub backup branch lane/smb-layout-ui-2-backup; old repo-local 23 GB target in the shared worktree.

</topic>
