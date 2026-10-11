---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-10-11T21:45:00Z
wp: WP-KERNEL-STUDIO
latest_snapshot: snapshots/SNAPSHOT-20261011T214451Z.json
authority: ledger.json, team.json, workflow/plausible-register.json, validation/*, START_LOG.md (this file is a projection; they win on conflict)
---

<topic id="read-first" status="active" summary="What to read before acting">

## Read first (any provider, no chat history needed)

1. `START_LOG.md` — Operator instructions, repo rules, cargo rules, every lead decision, Operator items D1-D27.
2. `builders/DISPATCH_TEMPLATES.md` — builder, validator and pre-review templates; workflow rules V4-01..V4-21 (V4-21: never put personal data in any request or header).
3. `ledger.json` — every lane (status, commits, verdicts, blockers, claims); `resume_plan` = ordered next steps; `round_R*` = round history.
4. `team.json` — agents and what they owned (status `running` = in flight when this was written).
5. `snapshots/SNAPSHOT-20261011T214451Z.json` — machine-readable point-in-time copy of all of the above plus repo heads.
6. `validation/` — every round result (`round-R*.json`), review (`R*-review-*.json`) and pre-review (`R*-prereview-*.json`).

</topic>

<topic id="repos" status="active" summary="Where the code and governance live">

## Repositories

| What | Repo / branch | Local worktree | State at handoff |
|---|---|---|---|
| Governance (this folder) | `Nuntissura/Handshake` / `gov_kernel` | `wt-gov-kernel` | pushed after every commit; other WPs (WP-KERNEL-012) also push here — integrate, never branch, never force |
| Product (35 Studio crates) | `Nuntissura/assets` / `codex/handshake-studio-assets`, subfolder `handshake-studio-assets` | `wt-studio-assets` | HEAD == origin (7410f29); uncommitted on purpose: local Cargo.lock patch bumps (not steward-reviewed) and stat-only score files |
| Build artifacts | `Handshake_Artifacts/WP-KERNEL-STUDIO/` (sibling of the worktrees) | — | cargo targets, round exports, red-proof scratch trees; never build on C: |

Push helper for governance: `tools/lead-kernel-push.sh <message-file> <paths...>` (commits the paths, merges `origin/gov_kernel` only when no path overlaps, pushes; never branches/forces). The kernel root `.gameplan-active` belongs to WP-KERNEL-012; run gated git/cargo commands with a leading `cd .GOV/docs_repo/wp-studio-module-work` (local Studio pointer) or from the product tree.

</topic>

<topic id="done" status="active" summary="What has been built and validated">

## Done

- 35 Rust crates exist as a standalone multi-crate module (not wired into Handshake), ~108k lines incl. tests; every crate builds and tests green on each canary (59/59 units).
- 106 work lanes: 89 PASS by an independent validator, 5 READY, 2 in work, 6 blocked on the Operator, 4 queued (see snapshot `lanes`).
- 12 validation rounds run (R4 abandoned): R5 ... R11 complete; latest complete R11 = 13 PASS / 0 FAIL on 6956d3f.
- Fully passed crates (27 of 35, every lane PASS): accord, chronicle, components, composite, controls, develop, folio, harness, interop-fig, interop-layout, interop-lrcat, interop-psd, interop-vector, kernel-bindings, layout, motion, motion-js, nib, package, prism, pulse, render-cpu, score, score-device, score-plugin, session, type. Not fully passed: interop-media, observe (no lane yet), pigment (plan-limits lane in R12), press, reel, reel-native, render-gpu, web (snapshot `crates`).
- Strongest areas: Photoshop-style layer document, compositing (CPU + GPU), adjustments/effects/Blend-If, colour management, PSD import; raw develop; video indexing/AV1 worker/exact export; audio engine.

</topic>

<topic id="in-flight" status="active" summary="Running when this was written">

## In flight at handoff

- Round R12 validator on candidate 3d795a5 (web lanes + pigment plan limits; review A complete). Output: `validation/round-R12.json`.
- Press pre-review R14 on 7410f29 (D27 flatten + narrow remediation). Output: `validation/R14-prereview-press.json`.
- If either is missing on takeover: re-dispatch from `builders/DISPATCH_TEMPLATES.md` (validator / pre-review) with the candidate in `ledger.json` `round_R12` and lane `SMB-PRESS` `candidate`.

</topic>

<topic id="not-done" status="active" summary="What is not done">

## Not done

- press (PDF redaction proof): 4 earlier designs failed pre-review; Operator chose D27 "Flatten + narrow" (redacted pages become a raster via hayro-interpret 0.7.0; other pages canonically rewritten; claim narrowed and limitations documented). READY at 7410f29, pre-review R14 running.
- web (HTML/CSS/SVG asset gate): spec-exact CSS tokenizer, 61-row URL carrier inventory, data: policy, load-context-first decisions; SMB-WEB has 2 FAILs (one more exhausts it); R13/R14 pre-reviews cleared it; judged in R12.
- Blocked on Operator: render-gpu + round entrypoint (D20), interop-media (D9), ProRes/DNx/H.264 and Windows hardware decode (D22-D26).
- Follow-ups registered (75 open items in `workflow/plausible-register.json`): notably PLR-069/070 (envelope reachability + backend adoption of caller plan limits), PLR-072..075 (pigment), PLR-063..068 (psd/reel), PLR-071 (web doc), SMB-PRESS-GEOMETRY, SMB-FMT-SWEEP (rustfmt pass after verdicts).
- No app UI yet and no Handshake integration (kernel_bindings is an in-memory adapter only). Spec-catalogue coverage is not measured.
- `.GOV/task_packets/WP-KERNEL-STUDIO/STUDIO_MODULES.json` still says PLANNED_NOT_IMPLEMENTED for all 35 crates (stale; ledger is live).

</topic>

<topic id="next" status="active" summary="Next steps in order">

## Next steps

Follow `ledger.json` `resume_plan` (authoritative). Short form:
1. Record R12 verdicts (validator writes `round-R12.json`, ledger verdicts); route any FAIL to remediation on the same lane.
2. Press: if R14 pre-review is SAFE_TO_JUDGE, steward sync + canary, then a round for press; if FIX_FIRST, fix within D27 or bring the Operator a decision.
3. Then the queued follow-ups (pigment PLR-069/070/072-075; reel PLR-066-068; psd PLR-063-065; reel-native PLR-059-061), fmt sweep, module-map status refresh.
4. Keep pushing `gov_kernel` after every commit while weekly usage is high.

</topic>

<topic id="operator-items" status="active" summary="Decisions only the Operator can make">

## Waiting on the Operator

D20 (allow a 4th verdict for render-gpu and the round entrypoint — lead recommends yes), D22-D26 (codecs/licensing/spec conflict/downloads), D9, D17 (Figma fixtures), D5 (Photoshop oracle session), D13/STU-CON-044, the repo-local 3.8 GB `handshake-studio-assets/target`, the empty `x` folder at the C: root, D21 (optional AGL download). Details: `START_LOG.md`.

</topic>
