---
file_id: WP-STUDIO-MODULE-WORK-HANDOFF
file_kind: handoff
updated_at: 2026-10-12T09:05:08Z
wp: WP-KERNEL-STUDIO
latest_snapshot: snapshots/SNAPSHOT-20261012T090508Z.json
previous_snapshots: snapshots/SNAPSHOT-20261012T072627Z.json, snapshots/SNAPSHOT-20261012T061647Z.json, snapshots/SNAPSHOT-20261012T051110Z.json, snapshots/SNAPSHOT-20261011T214451Z.json
authority: ledger.json, team.json, workflow/plausible-register.json, validation/*, START_LOG.md (this file is a projection; they win on conflict)
---

<topic id="read-first" status="active" summary="What to read before acting">

## Read first (any provider, no chat history needed)

1. `START_LOG.md` — Operator instructions, repo rules, cargo rules, every lead decision, Operator items D1-D27.
2. `builders/DISPATCH_TEMPLATES.md` — builder, validator and pre-review templates; workflow rules V4-01..V4-21 (V4-21: never put personal data in any request or header). Validator runs take build/test settings only from the checked-in round entry script (CX-VAL-007).
3. `ledger.json` — every lane (status, commits, verdicts, blockers, claims); `resume_plan` = ordered next steps; `round_R*` = round history (latest `round_R20`).
4. `team.json` — agents and what they owned (no agent is running at this handoff).
5. `snapshots/SNAPSHOT-20261012T090508Z.json` — machine-readable point-in-time copy of all of the above plus repo heads.
6. `validation/` — every round result (`round-R*.json`), review (`R*-review-*.json`) and pre-review (`R*-prereview-*.json`).

</topic>

<topic id="repos" status="active" summary="Where the code and governance live">

## Repositories

| What | Repo / branch | Local worktree | State at handoff |
|---|---|---|---|
| Governance (this folder) | `Nuntissura/Handshake` / `gov_kernel` | `wt-gov-kernel` | pushed after every commit; other WPs (WP-KERNEL-012) also push here — integrate, never branch, never force |
| Product (35 Studio crates) | `Nuntissura/assets` / `codex/handshake-studio-assets`, subfolder `handshake-studio-assets` | `wt-studio-assets` | HEAD == origin == 2dde832; uncommitted on purpose: local Cargo.lock patch bumps (not steward-reviewed) and stat-only score files |
| Build artifacts | `Handshake_Artifacts/WP-KERNEL-STUDIO/` (sibling of the worktrees) | — | cargo targets, round exports, canary + round results, red-proof scratch trees; never build on C: |

Push helper for governance: `tools/lead-kernel-push.sh <message-file> <paths...>` (commits the paths, merges `origin/gov_kernel` only when no path overlaps, pushes; never branches/forces). The kernel root `.gameplan-active` belongs to WP-KERNEL-012; run gated git/cargo commands with a leading `cd .GOV/docs_repo/wp-studio-module-work` (local Studio pointer) or from the product tree. Snapshot generator used for this handoff: rebuild from `ledger.json` + `team.json` + `workflow/plausible-register.json` (fields as in the snapshot file).

</topic>

<topic id="done" status="active" summary="What has been built and validated">

## Done

- 35 Rust crates exist as a standalone multi-crate module (not wired into Handshake); every crate builds and tests green on each canary (C25 at 3930a7c: 59/59 units, only NOT_PROVEN = interop-fig D17).
- 117 lanes: 105 PASS, 7 BLOCKED (Operator decisions), 4 queued/held, 1 done (steward).
- Validation rounds R5-R20 complete (R4 abandoned). Latest: R20 at 3930a7c = 24 PASS, 0 FAIL (new SMB-PRESS-DOCS-2, SMB-REEL-PARTIAL-PTS-2, SMB-INTEROP-PSD-LOSS-DETAIL PASS_V1; all four two-FAIL lanes re-passed; PLR-085..090 closed). Before that: R19 at fdc01f1 = 21 PASS, 0 FAIL (new SMB-PRESS-DOCS, SMB-REEL-PARTIAL-PTS, SMB-INTEROP-PSD-FOLIO-CEILING PASS_V1; reopened press, reel, reel-native, interop-psd lanes PASS; PLR-063/076/077/083/084 closed). Before that: R18 at bfa1bbf = 3 PASS, 0 FAIL — SMB-PRESS PASS_V4, SMB-PRESS-DIFFERENCES PASS_V2, SMB-NOTES-BATCH-1 PASS_V1; press 147/147 tests. Press redaction proof closed via D27 "flatten + narrow" plus the MINIMAL DRAW SET rule (START_LOG).
- Fully passed crates (29; every lane PASS): accord, chronicle, components, composite, controls, develop, folio, harness, interop-fig, interop-layout, interop-lrcat, interop-psd, interop-vector, kernel-bindings, layout, motion, motion-js, nib, package, pigment, prism, pulse, render-cpu, score, score-device, score-plugin, session, type, web.
- Not fully passed: press (only SMB-PRESS-GEOMETRY queued), reel + reel-native (codec lanes blocked on D22-D26), render-gpu (D20), interop-media (D9); observe has no lane yet.
- Strongest areas: Photoshop-style layer document, compositing (CPU + GPU), adjustments/effects/Blend-If, colour management, PSD import; raw develop; PDF read/edit/redact with own strict renderer; web export with spec-exact CSS tokenizer; video indexing/AV1 worker/exact export; audio engine.

</topic>

<topic id="in-flight" status="active" summary="Running when this was written">

## In flight at handoff

- 2026-10-12T09:05:08Z: all three lanes READY at 2dde832 (SMB-PRESS-STRUCT-FAILCLOSED claim d26d2fd+2566f1b+54c60f9+2dde832 = STRUCTURE TREE = NOT PROVEN; SMB-REEL-PTS-TESTS 172952f and SMB-INTEROP-PSD-DESC-CAP 0c1be05 SAFE in R23). Running: Opus narrow pre-review of press 54c60f9..2dde832 (output `validation/R24-prereview-press-narrow.json`). Then steward sync + canary C26 + round R21 (`ledger.json` resume_plan step 1). Weekly usage 98% at this point: if the limit hits, resume exactly there.

</topic>

<topic id="not-done" status="active" summary="What is not done">

## Not done

- Next small lanes: press SMB-PRESS-STRUCT-FAILCLOSED (PLR-094 lead decision FAIL CLOSED: catalog-direct /StructTreeRoot with /Pg-less elements -> NotProven; + PLR-093 doc + PLR-091 test); interop-psd SMB-INTEROP-PSD-DESC-CAP (PLR-095: 8192-item descriptor cap in DESCRIPTOR, mapped-count wording, measure items per instance); reel SMB-REEL-PTS-TESTS (PLR-092); then SMB-PRESS-GEOMETRY.
- Notes residuals: config-OBU tag, exact-64 FRAME test, psd group-own-lfx2/zero-tile. SMB-FMT-SWEEP (rustfmt pass).
- Blocked on Operator: render-gpu + round entrypoint + SMB-RENDER-GPU-ADJ/PROBE + PLR-082 (D20), interop-media (D9), ProRes/DNx/H.264 and Windows hardware decode (D22-D26).
- 80 open items in `workflow/plausible-register.json`.
- No app UI yet and no Handshake integration (kernel_bindings is an in-memory adapter only). Spec-catalogue coverage is not measured.
- `.GOV/task_packets/WP-KERNEL-STUDIO/STUDIO_MODULES.json` still says PLANNED_NOT_IMPLEMENTED for all 35 crates; no other status value is defined anywhere checked, so it was not changed (ledger is live).

</topic>

<topic id="next" status="active" summary="Next steps in order">

## Next steps

Follow `ledger.json` `resume_plan` (authoritative). Short form:
1. Dispatch (Sonnet builders, new lane ids per V4-15): SMB-PRESS-STRUCT-FAILCLOSED (PLR-094 first), SMB-INTEROP-PSD-DESC-CAP (PLR-095), SMB-REEL-PTS-TESTS (PLR-092).
2. V4-13 narrow pre-review before the round (two-FAIL lanes SMB-PRESS, SMB-INTEROP-PSD-BUDGET, SMB-REEL-AV1-ROUTE, SMB-REEL-NATIVE-AV1-WORKER via reel).
3. Round R21: steward sync + canary + 1 Opus validator; the brief lists every two-FAIL lane reopened (PROC-R19-V1).
4. Then SMB-PRESS-GEOMETRY, SMB-FMT-SWEEP, notes residuals.
5. Keep pushing `gov_kernel` after every commit while weekly usage is high.

</topic>

<topic id="operator-items" status="active" summary="Decisions only the Operator can make">

## Waiting on the Operator

D20 (allow a 4th verdict for render-gpu and the round entrypoint — lead recommends yes), D22-D26 (codecs/licensing/spec conflict/downloads), D9, D17 (Figma fixtures), D5 (Photoshop oracle session), D13/STU-CON-044, the repo-local 3.8 GB `handshake-studio-assets/target`, the empty `x` folder at the C: root, D21 (optional AGL download), D28 (strip the PDF structure tree on a redaction save so tagged documents can be proven, at the cost of accessibility tags — or keep NotProven), status vocabulary for `STUDIO_MODULES.json`. Details: `START_LOG.md`.

</topic>
