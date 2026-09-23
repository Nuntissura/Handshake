# HANDOFF — WP-KERNEL-012 IV orchestration (2026-09-23, session 5)

You are the INTEGRATION VALIDATOR acting as orchestrator for `WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1`. You steer sub-agents (KERNEL_BUILDER, WP_VALIDATOR, cheap review agents); you do not write product code. Supersedes the session-4 handoff (`HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-17_session4.md`), which stays as history: its §3 decisions still hold unless changed below; its host rules §5 are REPLACED by §5 here.

Read the LIVE kernel copies (`wt-gov-kernel/.GOV/...`, never `handshake_main/.GOV`): Codex `codex/Handshake_Codex_v1.4.md` (compact v1.5), `roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`, and root `CLAUDE.md`/`AGENTS.md`. Codex CX-021 + Operator: no `just`, no repomem/ACP/receipts/dossiers; MT-json is the only status surface; status correctness + state recovery are the only governance goals.

## 0. Read first: the two mistakes that cost the most last session

1. **Test-by-test discovery of one systemic defect (the biggest waste).** Builder rounds 2–4 took about 6.5 h and about 3.2M tokens (≈685k+712k, 819k, 945k) and produced **0 new PASS**. Each round fixed one failure layer and exposed the next. A read-only static audit then found the whole defect class in **18 min and ≈450k tokens** (159-route matrix, §3.2).
   - Why it happened: SurrealDB fails denied writes silently, so one design gap (routes running as the root session) looked like many unrelated 403/500/timeouts. Each round showed some fixes landing, so continuing felt like progress. The orchestrator applied the 2-attempt rule to the builders' individual reruns but not to its OWN rounds, and relayed the builder's "next blocker" framing instead of asking whether the pattern was the finding.
   - Rule for you: apply Codex CX-EXEC-003/003A to your own orchestration. If a fix round's representative tests fail in a NEW place instead of passing, you get at most one more round of the same approach. After that, stop testing and run a static audit (read-only Opus agent over code + schema + spec) before the next build. Ask every round: "is this one cause showing up in many places?"
2. **Broad validation over an unexplained systemic failure.** The C1 shared validation ran 3 h 50 min and ≈500k tokens; most of it reproduced the same auth failure across binaries, which masked every later assertion.
   - Rule for you: before a broad validation run, check that the known failure classes are fixed. If the first ~10–20% of rows show one dominant class, stop the run, route the class to a builder, and resume after the fix.

Smaller losses: about 1 h of a validator stalled on an approval prompt (now fixed by the allowlist); a false disk-cap alarm from hardlink double counting; C3 gates blocked by a host-wide busy check (§10).

## 1. Goal

Close WP-KERNEL-012: every MT at validator-proven `PASS_Vn`, then the whole-WP IV verdict, then merge to `main`. Push real progress: parallel lanes on separate disks, fix-then-validate-once, no test-by-test discovery loops.

## 2. Verified state (2026-09-23 11:45 local)

- Product: branch `feat/WP-KERNEL-012`, worktree `wtc-native-editors-v1`, pushed and clean at `0cfbff64` (builder C3). Always verify with `git ls-remote origin refs/heads/feat/WP-KERNEL-012`.
- Governance: `gov_kernel` pushed. The lean `roles/coder/CODER_PROTOCOL.md` is committed (`0d48d398`); the old one is in `roles/coder/archive/`. MT-153..157, packet.json and all MT records are committed with this handoff. The Operator-approved authority changes (Codex CX-EXEC-003A/005, CX-SAFE-002, CX-984-014; IV/KB/WPV protocol updates; HANDSHAKE_BUILD_RULES.json; the two `*_CONTRACT_TEMPLATE.json`) are committed too, and the tree is clean. Commits need no Operator approval (Operator, 2026-09-23); just use explicit paths.
- `handshake_main` local `main` is 2 commits ahead of `origin/main` (the Operator's docs commit and `71298891d` `.claude/settings.local.json` allowlist). Both go out with the WP merge.
- MT tally: 115 PASS; 29 FAIL (008 023 027 033 034 036 046 064 065 066 067 068 074 079 088 098 108 113 116 117 120 121 122 127 128 130 140 141 143); 3 BLOCKED_ON_DEPENDENCY (026 070 111); 4 PARTIAL (045 124 125 142); 1 READY_FOR_VALIDATION (131). Plus MT-153 IN_PROGRESS (C3 scope, gates pending) and MT-154..157 READY (§4, §7).
- C: free 333 GB at handoff (after the approved cleanup; includes V2's stale export and partial target-v2, which the next validator removes). The C: stop line for WP builds stays at 248 GB free.

## 3. What happened (sessions of 2026-09-22/23)

1. Batch-fix round (builder C1) → shared validation (C1 validator, 1 new PASS: MT-132). Rounds C1-3/4 kept fixing one failure layer and exposing the next.
2. Root cause found by a STATIC audit (not tests): the WP's PostgreSQL→SurrealDB port left most product routes running as the root/system DB session. The Master Spec forbids that (02-system-architecture.md:2773 "privileged SurrealDB sessions MUST NOT execute ordinary protected-resource flows"; :2776 record-user permissions + ResourceBroker are the non-bypassable boundary; LM-RLS-001/002 at 11-shared-dev-platform-and-oss-foundations.md:580-582). SurrealDB 3.2.0 fails denied writes SILENTLY, so each missing permission surfaced one test at a time.
   - Audit (durable copy): `Handshake_Artifacts/WP-KERNEL-012/MT-109/authority-audit/authority_matrix.md` (159 routes: 39 OK, 12 GAP, 61 ROOT-WRITE, 46 ROOT-READ, 1 UNSURE), `authority_scope.md` (111 of 112 non-OK rows are IN this WP), and `lessons_wp_kernel_012.md` (L1–L17). Give these paths to builders; the audit reflects `2f49f2a6`, so later commits have closed some rows.
3. Builder C2 (`ab618f46`) fixed: workspace delete cascade (the Operator's 2026-09-22 cascade decision), rename 500, memory routes + grants, knowledge-doc backlinks/sources, auth on `/debug/sessions*`, `/source-control/*` and `/kernel/events/aggregates`, and 16 backend harnesses moved to account sessions. Records: MT-109 `remediation_v24`.
4. Spec rulings recorded as `spec_basis` (not operator decisions): members may create all five LoomBlockContentTypes (note, file, annotated_file, tag_hub, journal; `canvas` kept); user-initiated writes must run under the account session; the workspace-delete cascade must be permitted by DB record-user permissions plus a ResourceBroker recheck (no system transaction).

## 4. Lane state at handoff (all agents stopped cleanly, 2026-09-23 ~12:30)

- Lane A, KERNEL_BUILDER-C3, STOPPED-FOR-HANDOFF. Pushed `0cfbff64` (commits a1e18d79, 19922490, 0cfbff64; tree clean). Record: MT-109 `remediation_v26` (covers MT-153), with pointers in 14 MTs.
  - Done: all 37 Loom ROOT-WRITE rows, all 29 ROOT-READ rows and the PATCH GAP row now run as the account record user. The schema was re-pinned once. C2 follow-ups a–c are fixed and proven (b was a placement lookup that did not accept a text card's rich-document resource); d needed no change. Representative runs: kb-c3 logs 06 (15 of 17 pass, including every pin, upgrade and workspace-delete test) and 08 (graph/search passes).
  - Open:
    1. Its gates never ran (runs 09–15 logged BUSY because its script waits on ANY host cargo, and the V2 build was running on C:). First action: run `Handshake_Artifacts/WP-KERNEL-012/MT-109/kb-c3/gates.sh`, after changing its busy check to per-disk.
    2. Stage-card create/compensate still write as root after the grant check; asset import and tier retry still start preview jobs as root.
    3. The pre-existing test `mounted_record_user_loom_creates_are_atomic_and_denied_writes_leave_no_rows` still fails: a record user's `workspace_id` update is silently ignored (SurrealDB silent deny) where the test expects a 403. Decide from spec/code whether the route must detect the no-op and return 403 (probably yes: deny-by-default must be observable); one confirm run is left.
    4. 11 native Loom clients send no account session, so they now get 403. Put this in C4 (native/app-host).
  - D: target `kb-c3/target` is 19 GB and warm at `0cfbff64`; reuse it for C4.
- Lane B, WP_VALIDATOR-V2, STOPPED-FOR-HANDOFF before any test ran. The clean export of `ab618f46` is complete (7,139 files) at `C:/.target/WP-KERNEL-012/export-ab618f46`, with a partial cold build in `C:/.target/WP-KERNEL-012/target-v2`. `ab618f46` is now superseded by `0cfbff64`: make a NEW export of the current pushed SHA for the next validation, delete the old export and target-v2 (approved routine cleanup inside the C: grant), and do not validate `ab618f46`.
- MT author: DONE. MT-153..MT-157 are written and registered (packet `mt_plan.declared_ids` and both `mt_status` maps now cover MT-001..MT-157). Audit references point to the durable D: copy. Notes from the author:
  - MT-157: durable breakpoint storage already exists (`database.rs:1476` → `state_store.rs:680/706`); the real gaps are authentication, root execution, the `knowledge_debug_breakpoints` table being `PERMISSIONS NONE`, and receipts written under a fixed actor.
  - MT-155: today the request body chooses which program runs (`node_binary`, `adapter_script_path`) with no timeout; the MT forbids caller-chosen programs and requires a timeout and cleanup.
  - MT-156: also covers three sibling routes on the same router (status, validate, exportable), since they expose the same bundle data.
  - MT-154: the revision-158/159 checksum tests fail because four schema commits (edb54c0e, c792b8a2, 64ca7c82, 01df5ebf) changed the schema after pinning; the fix rebuilds the old schema exactly and does not change old pins.
  - MT-154 carries three pre-filed questions, each defaulting to "must be logged in" until decided: D-154-1 the legacy `/workspaces/:ws/documents` route; D-154-2 how the Flight Recorder reads DuckDB events; D-154-3 scoping for ~60 Atelier tables plus the locus and global-preference tables, which have no workspace or owner column. Resolve them from the Master Spec first (Operator rule: research spec/online, use sub-agents); only escalate what the spec cannot answer.
- The packet `mt_status` maps are projections and may be stale for older MTs. MT-*.json `lifecycle` is the authority.

## 5. Build, disk and host rules (HARD; replace session-4 §5)

- Two disks, two lanes: C: (Samsung 870 QVO SSD) and D: (16 TB HDD). At most ONE cargo per physical disk at a time (Codex CX-984-002); a C: build and a D: build may run concurrently.
- C: grant (Operator, 2026-09-22; Codex CX-984-014): `C:\.target\WP-KERNEL-012\` for BUILD TARGETS only, WP-scoped, 100 GB cap, cleaned at WP close. Measure the cap by C: free space (baseline 341.9 GB after cleanup, so stop cargo below 248 GB). Never sum files under the target (hardlinks double-count).
- Test runtime roots (Surreal store, TMP/TEMP/TMPDIR, HANDSHAKE_WORKSPACE_ROOT) always on D: under `Handshake_Artifacts/WP-KERNEL-012/<MT>/<lane>/` (tests hung with C: runtime roots; Defender exclusions now exist for `C:\.target` and `D:\Projects`, but D: runtime is the proven setting).
- **Operator decision 2026-09-23 15:37 (session 6): test runtime roots (Surreal store, TMP/TEMP/TMPDIR, HANDSHAKE_WORKSPACE_ROOT, HANDSHAKE_TEST_ARTIFACTS_ROOT) move to the C: SSD under `C:.targetWP-KERNEL-012<MT><lane>untime` (the C: grant now covers WP-scoped test runtime too; same 248 GB free-space stop line; cleaned at WP close). This supersedes the D:-runtime rule in the line above. Reason: DB-heavy tests crawled on the D: HDD (~70 KB/min WAL writes), and schema-upgrade tests stalled in both lanes. If a test hangs with C: runtime roots, fall back to D: for that test only and record it.**
- **IV decision 2026-09-23 16:00 (session 6), root cause of the zero-CPU test stalls:** embedded SurrealDB 3.2.0 RocksDB defaults to `SyncMode::Every` (fsync per commit; `surrealdb-core-3.2.0/src/kvs/rocksdb/cnf.rs:598`), so DB tests are fsync-latency-bound on the HDD and the QLC SSD alike. Set `SURREAL_DATASTORE_SYNC=never` (env key `datastore_sync`, `cnf.rs:759`, value `never` per `cnf.rs:863-867`) for all test runs EXCEPT crash/kill/restart/reopen/durability/recovery tests, which keep the default. Record the setting in every proof record.
- Every cargo call: `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `--locked`, and `CARGO_TARGET_DIR` inside the lane. `-j 2` for link-heavy steps on the D: HDD.
- Builder readiness gate: `cargo check --locked --tests` per feature set (A `surreal-test-support,test-utils`; B `app-runtime,surreal-test-support,test-utils`; C per `MT-141/wpv-v2/logs/exe-map.tsv`; native crate), native clippy `-D warnings` (default + integration), core clippy with 0 errors on changed lines (395–422 pre-existing core clippy errors are recorded debt, not in scope). Do NOT link every test binary (~394 set-A targets × ~250 MB ≈ 98 GB).
- Validators link, run and delete in chunks (WPV-ART-006), from a clean `git archive` export whenever a builder is editing the worktree at the same time.
- Deletes inside `C:\.target\WP-KERNEL-012\` and `Handshake_Artifacts\WP-KERNEL-012\` are approved routine cleanup: `.claude/settings.local.json` on main allows `PowerShell(*)`, and an autoMode rule allows those deletes. Anything else destructive follows root `CLAUDE.md`: list the exact targets, then wait for `approved`/`proceed`, then run it yourself (never hand the Operator a raw command instead).
- Never stop processes you did not start without `PROCESS_STOP_APPROVED:<PIDs>`. No python (Windows opens an installer). Non-interactive tools only (Codex CX-SAFE-002).

## 6. Sub-agent rules (learned the hard way)

- Every brief states: the authority files to read and acknowledge (builders: Codex + KERNEL_BUILDER_PROTOCOL + CLAUDE.md/AGENTS.md; validators: Codex + WP_VALIDATOR_PROTOCOL + CLAUDE.md/AGENTS.md; small coders: Codex + CODER_PROTOCOL + the MT), the exact scope list, the lane/target/runtime paths, the gates, the record fields, and a ≤20-line report.
- Run budget per failing check: 2 runs, then a written diagnosis (failing assertion, change per attempt, root cause at file:line, fix), then 1 probe + 1 confirm, then record and move on (Codex CX-EXEC-003/003A). Enforce it: agents rationalise each rerun as "a new blocker"; check lane ledgers and steer when the same test shows up a third time.
- When two rounds fail to converge, switch to static analysis (read-only Opus agent over code + schema) instead of another test round.
- Fresh builder per scope batch (C2, C3, C4…) keeps context small; resume the same agent only for a short follow-up in the same batch. Retire any agent whose resumes cost more than ~600k tokens.
- No commands that can wait on interactive approval. If one would prompt, the agent records a blocker and continues.
- Verify every agent claim yourself before relaying: HEAD == ls-remote, clean tree, MT JSON parses, status == validator_verdict, counts taken from `results.jsonl`.
- Snapshot MT records after every round: `git -C wt-gov-kernel commit -m "..." -- <explicit paths>` (NEVER a bare `git commit`, which sweeps in staged files; that is how the Coder-protocol defect in §2 happened), then `git push origin gov_kernel`.
- Monitoring tick 20–30 min (cron at off-minutes). Stall = no lane/MT-json/process change for 20 min → status request. Delete the cron when no lane is active.

## 7. Scope expansion (Operator 2026-09-23: no new WP; extra MTs inside WP-KERNEL-012)

- MT-153: Loom routes to account sessions (C3 implementing).
- MT-154: remaining areas to account sessions (calendar, atelier/intake, stage, preferences, kernel/event writers, flight recorder, locus, canvases), plus: the native create-note drain regression (menu confirm never reaches the panel handler, context_menu.rs:236, since 1236078e); receipt writers without an account session; the schema-upgrade predecessor-checksum tests (revisions 158/159).
- MT-155: authenticate `/kernel/product_screenshot_capture/execute` (pre-existing, unauthenticated, runs a process).
- MT-156: debug bundle export/download under the account session.
- MT-157: durable debugger breakpoint storage (MT-136 residual; MT-136 stays PASS).
Each MT carries materialized acceptance (with spec quotes), model_tier, attempt_budget, write paths, stop conditions, implementation steps with file:line, exact proof commands with timeouts, and risks, so a no-context model can execute it.

## 8. Known blockers and how to resolve them

| Blocker | Resolution |
|---|---|
| Root-session routes (authority migration) | C3 (Loom) → C4 (MT-154 + MT-155/156), from the audit list; then MT-157. Fix in batches; one validation per batch. |
| Native live binaries (~47) never reached their real checks | Run once after C4 lands: one validator, clean export, C: target. |
| MT-124 / MT-125 RED halves | Patches ready: `Handshake_Artifacts/WP-KERNEL-012/MT-124/kb-c1/red-half.patch` and `.../MT-125/kb-c1/red-half.patch` (`git apply --check` OK at c1060bca; re-check at the new HEAD). The validator applies, shows red, reverses, and checks `git diff` is empty. The Operator gave standing authorization 2026-09-22. |
| MT-045 release perf proof | One release build at the end (Operator-approved once), on the C: grant; manifest writes already moved off the tracked tree. |
| MT-142 extended swarm stress (64 workers / 50k ops) | Last item, solo on an idle host. |
| Canvas text-card undo placement DELETE 403, transclusion 403, tag-edge PATCH timeout | C3 scope. |
| Two schema-upgrade tests (rev 158/159 predecessor checksum) | MT-154. |
| 395–422 pre-existing core clippy errors | Recorded debt (MT-079); not in this WP. |
| Unauthenticated screenshot route, bundles export | MT-155/156. |
| WP-boundary full suite + HBR/Argus/UserManual/diagnostics closure | IV duty after all MTs pass (IV protocol: HBR Gate Obligations). |

## 9. Closure sequence

C3 lands → C4 (MT-154/155/156) in the worktree while the validator checks the C3 Loom MTs from an export → native live run → MT-157 → RED halves 124/125 → MT-045 release proof → MT-142 solo stress → WP-boundary full suite + HBR closure → IV verdict → artifact hygiene + final cleanup of `C:\.target\WP-KERNEL-012` → merge into local `main` (backup push first), then sync-gov-to-main, then push `origin/main`, with Operator approval at each git-rewriting step per root `CLAUDE.md`.

## 10. Mistakes to avoid (cumulative with session-4 §11)

1. Iterating test-by-test for 4 rounds before switching to a static audit (should switch after 2).
2. A validator stalled 1 h on an approval prompt overnight; now fixed with the allowlist, and the brief rule is "never wait on prompts".
3. Summing `C:\.target` file sizes (hardlinks) triggered a false cap alarm; measure free space instead.
4. Handing the Operator a raw delete command instead of the listed-targets → `approved` → execute flow; and a bash-quoted `$_` that silently did nothing.
5. A bare `git commit` swept a staged rename into an MT snapshot (§2 defect).
6. Lane scripts that wait on ANY host cargo deadlock two-disk parallelism (C3 gates never ran). Busy checks must be per disk (the target path in the cargo command line).

## 11. First actions

1. `git ls-remote` + status for `feat/WP-KERNEL-012` (expect `0cfbff64`) and `gov_kernel`; process scan (cargo/rustc/link/test_); C: free.
2. Two lanes at once:
   - Lane A (D: HDD, warm `kb-c3/target`): builder C4 = C3's gates first (fix `gates.sh` to check per disk), then C3 open items 2–4, then MT-154 + MT-155 + MT-156, then MT-157. Fresh Opus builder, briefed per §6.
   - Lane B (C: SSD): validator = new clean export of `0cfbff64`, clean up the old export and target-v2, validate per MT per Codex [CX-EXEC-008] (queue of MTs with named blocking tests; each verdict written immediately; no broad suites before the WP boundary), and group failures by area for C4. (Amended 2026-09-23 by Operator-approved [CX-EXEC-006..011] / [ORC-OUT].)
3. Resolve MT-154's D-154-1..3 from the Master Spec with a read-only sub-agent while the lanes run.
4. Re-arm the two-lane tick. Snapshot every round by explicit path.
