# HANDOFF — WP-KERNEL-012 validation/remediation orchestration (2026-09-17, session 4)

You are the INTEGRATION VALIDATOR acting as orchestrator for `WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1`. You spawn and relay between sub-agents (KERNEL_BUILDER, WP_VALIDATOR). You do not write product code yourself. Read in full before acting: `.GOV/codex/Handshake_Codex_v1.4.md`, `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md` — read the LIVE kernel copies under `wt-gov-kernel/.GOV/` (the `handshake_main/.GOV/` copies are a stale mirror; the previous session read the wrong ones first). Operator instruction overrides protocol ceremony (§6 below). Supersedes `HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-17.md` (session 3).

## 1. Goal (unchanged)

Bring every MT of WP-KERNEL-012 to a validator-proven PASS. One MT (or one shared batch) at a time with a fresh KERNEL_BUILDER + WP_VALIDATOR pair. You orchestrate, relay, monitor with a repeating tick; you never sit idle.

## 2. Verified state at handoff (2026-09-17 ~21:00Z)

- Product branch `feat/WP-KERNEL-012` @ **`c0766697664a416304dd08c8620cb400724c9104`** (MT-141 v2 candidate), worktree `wtc-native-editors-v1` **clean**, pushed (verify with `git ls-remote origin refs/heads/feat/WP-KERNEL-012`; never trust `origin/*` tracking refs — the worktree's are stale).
- Governance `gov_kernel` @ **`c532acf6`**, pushed. `wt-gov-kernel` has one uncommitted Operator edit (`HANDOFF_..._2026-09-17.md`, session-3 brief) — leave it.
- `handshake_main` local `main` is 1 Operator docs commit ahead of `origin/main` and cannot fetch (`bad object refs/codex/turn-diffs/captures/*.pyc`) — not yours; do not work there.
- Cargo targets (all Operator-approved, do NOT create more; reuse per §5):
  - warm validator target `../Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-v15/target` — being rebuilt at c0766697 by the active v3 lane (set A `--tests` build started 18:36Z, still linking at 20:30Z);
  - builder target `../Handshake_Artifacts/WP-KERNEL-012/MT-135/kb-b2/target` (55 GB, at c0766697);
  - cold export target `../Handshake_Artifacts/WP-KERNEL-012/MT-148/wpv-v2/target` (30 GB, at ba03f563: core lib + handshake-native + palmistry) built from the clean git-archive export `MT-148/wpv-v2/workspace/src-ba03f563` (7131 files, NOT a worktree).
- **Active lane at handoff:** `WP_VALIDATOR-B3-20260917-182941`, lane `../Handshake_Artifacts/WP-KERNEL-012/MT-141/wpv-v3/`, validating MT-141 candidate c0766697 (plan: setA `--tests` → lib → setB → setC; re-execute all 80 validation_v2 reds + diff-affected set; cite unchanged greens by sha). If you are a new session it is gone with the old one: read its `00-lane.txt`/`results.jsonl`; if the setA build finished (`logs/10-norun-setA.log.exit` == 0) spawn a fresh `wpv-v3` validator that RESUMES the lane (same lane dir, continue numbering, reuse the built exes on the warm target — do not rebuild); if the build was cut off, the fresh validator re-runs the `--no-run` (cargo resumes incrementally).
- Host: 0 cargo/rustc/agents unless the lane above is still alive. Tick cron dies with the session — re-arm (§10).
- **Session-4 end state (2026-09-18 ~00:30Z, weekly provider limit reached — not a failure):** wpv-v3 builds so far on the warm target at c0766697: set A `--tests` DONE (exit 0, 4h39m, `logs/10-norun-setA.log`), lib DONE (exit 0, 887 s, `logs/11-norun-lib.log`), set B IN PROGRESS (`cargo.exe` PID 9300, `logs/12-norun-setB.log`, started 23:30Z, `app-runtime,surreal-test-support,test-utils`); set C not started; no test executed yet (`results.jsonl` absent). Validator's plan is in `00-lane.txt` (LISTS line): execute 72 set-A v2 reds + 126 affected set-A binaries, cite 194 unchanged set-A rows from wpv-v2 by exe/log sha; then lib, set B (all), set C. Before doing anything: (a) `Get-CimInstance Win32_Process` for `cargo.exe`/`rustc.exe`/`link.exe` — if a cargo is STILL RUNNING it belongs to the dead session's validator; do NOT kill it and do NOT start another cargo; wait for it to exit, then check the newest `logs/NN-norun-*.log.exit`; (b) if it is gone and the set-B `.exit` is missing, the build was cut off — the fresh validator re-runs the identical `--no-run` (incremental; only un-linked targets remain); (c) then set C `--no-run`, then execution per the LISTS plan, one binary at a time, `results.jsonl` rows as in wpv-v2. Spawn the fresh validator with the same brief as §14.2 and tell it to RESUME this lane (continue log numbering from 13).

### MT tally (152)
- **114 PASS** (status == validator_verdict == `PASS_Vn`). Today: MT-148 `PASS_V2`.
- **1 FAIL_V2 under validation:** MT-141 — `remediation_v2` recorded (candidate c0766697; groups V2-R1..R6 done; set A/B/lib green on the builder's lane; one open item: `knowledge_code_nav_api_tests::mt045_lc06_500_file_code_nav_index_is_embedded_surrealdb_bounded` 40 s vs 10 s budget on debug profile, attributed to engine/debug cost; `PERF_BUDGET_LC06_MS` override is sanctioned by MT-045 step 8 for slow machines — validator decides). Set-B regressions caused by v2 were fixed in `state_recovery.rs`/`locus_store.rs` (commit c0766697).
- **3 FAIL_V6 (native clippy/build halves executed for real today):** MT-079, MT-098, MT-088. Root: `cargo clippy -p handshake-native --all-targets -D warnings` at ba03f563 → 68 lib errors under toolchain 1.97.1 (37× `float_literal_f32_fallback`, rest ordinary lints; only 2 introduced by MT-098; others blamed to MT-003/012/014/015/017/021/026/054/055/057/059/060/061/076/101/103/113). Full attribution: `../Handshake_Artifacts/WP-KERNEL-012/MT-079/wpv-v6/logs/01_clippy_errors_attributed.json` and each MT's `validation_v6.clippy_classification`. Build halves and palmistry.exe GREEN. MT-088's `#[ignore]` live palmistry/Argus test is unrunnable by design (see §3.3).
- **5 PARTIAL_PENDING_OPERATOR_DECISION:** MT-045 (perf release-build proofs; supervisor script `run_mt045_perf_proof.ps1` is cargo-invoking; also perf manifest must not write the tracked `tests/perf_proof/perf_manifest.json` — builder item), MT-124/125/132 (RED halves: guard must be shown red with the fix reversed — needs exclusive tree), MT-142 (PT-142-7: rerun `HANDSHAKE_SWARM_EXTENDED=1 WORKERS=64 OPERATIONS=50000 DATASET=5000` solo on an IDLE host; expected integrity pass, 0 timeouts).
- **1 NEEDS_REIMPLEMENTATION_V1:** MT-131 (add `tests/test_slash_commands.rs` test: open a SlashPrompt, run `accessibility::assert_no_unnamed_interactive`, assert inspected > 0 and `slash-prompt-surface` present).
- **28 BLOCKED_ON_DEPENDENCY** (native MTs, F403 etc.): 008 023 026 027 033 034 036 046 064 065 066 067 068 070 074 108 111 113 116 117 120 121 122 127 128 130 140 143. Each `validation_vN` names its dependency (F403 / F500 / F401 / F111 / FDIRTY / FTAGS / FCANVAS, see §7).

## 3. Operator decisions (binding; 2026-09-17 sessions 3+4)

1. **MT-141 builder scope waiver** (session 3) stands; `remediation_v2.out_of_scope_edit_waiver` lists 34 files with `granted_by: "OPERATOR (session waiver 2026-09-17, relayed by orchestrator)"`.
2. **F403 — unblock it** (unchanged): native proof support (`src/frontend/handshake_native/tests/backend_proof_support/mod.rs`) sends only `x-hsk-session-token`; `api/authority.rs::authorize_request` returns 403. Builder root-causes (policy vs stale fixtures), fixes the correct side, records reasoning. Outcome: the 28 BLOCKED verdicts' F403 dependency resolves.
3. **Native batch = ONE shared builder session (not per-MT rounds):** (a) crate-wide `handshake-native` clippy sweep to `-D warnings` clean under 1.97.1, attributing fixes to owning MTs; (b) **MT-088-R6-2** test-only env-contract fix in `src/frontend/handshake_native/tests/test_backend_down_responsive.rs`: `HANDSHAKE_PALMISTRY_EXE` becomes authoritative when set (drop the "must equal `../../../../Handshake_Artifacts/handshake-cargo-target/debug/palmistry.exe`" assert, lines ~106-129), provenance takes `HANDSHAKE_PROOF_SOURCE_SHA` (+ optional `HANDSHAKE_PROOF_REPO_ROOT`) and falls back to `git rev-parse` only when unset (lines ~152-165, ~1133-1150); (c) sweep `src/**/tests` for the same pattern (hard-coded `handshake-cargo-target`, `git rev-parse`, `.GOV` reads at test runtime — known sibling `src/backend/handshake_core/src/api/kernel.rs:779`) and fix all once; (d) MT-131's missing test. Palmistry itself is a companion diagnostic app; how it is BUILT does not change — no rule changes.
4. **MT-148 precedent:** "fresh worktree" proofs are satisfied by a cold build from a clean `git archive` export into a new empty target under the MT's artifact folder. Coders/validators never create git branches or worktrees.
5. **No half runs:** NOT_RUN halves are executed for real; no waivers. MT-124/125/132 RED halves run when no builder owns the tree (`git apply -R` of the fix commit's diff in the working tree is a tree rewrite — needs same-turn Operator authorization each time; ask).
6. **Order:** MT-141 v3 verdict → (PASS) native batch §3.3 → MT-124/125/132 RED halves → MT-045 release perf + manifest fix → MT-142 idle-host rerun → F403 builder → 28 BLOCKED native re-verdicts. (MT-141 FAIL → fresh builder with the validator's groups first.)
7. **Crash root cause found (session 4):** the 16 Sep and 17 Sep "crashes" were Windows Update restarts (`MoUsoCoreWorker.exe` at 09:59 local, then `TrustedInstaller` reboots; zero bugchecks/WHEA/minidumps). Operator paused updates for a month. Not cargo.
8. **Host constraints:** one cargo at a time host-wide; C: drive is OFF LIMITS (nearly full); no SSD purchase; everything stays under `D:/.../Handshake_Artifacts/`.
9. **Fap Test / adult scope:** not applicable to this WP.

## 4. Hard location rules (unchanged)

- Work ONLY inside `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-native-editors-v1` (branch `feat/WP-KERNEL-012`). `.GOV/` there is a live junction to `wt-gov-kernel/.GOV` — read/write MT contracts through it.
- `handshake_main` and `wt-gov-kernel` product trees are stale mirrors; never work in `main`. Give sub-agents absolute paths (two agents once resolved relative paths against the shell cwd).
- MT contracts: `wtc-native-editors-v1/.GOV/task_packets/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-NNN.json` (large; extract with `node -e`, never `cat`).
- Protocols agents must read in full and acknowledge: `.GOV/codex/Handshake_Codex_v1.4.md`, `.GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md` (builder), `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md` (validator). Spec via `.GOV/spec/SPEC_CURRENT.md` → active `master-spec-vNN.NNN/INDEX.json`.

## 5. Cargo / artifact / host rules (HARD)

- Everything under `D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-012/MT-NNN/<lane>/…` (`HANDSHAKE_ARTIFACTS_ROOT` absolute). Lanes: builder `kb-vN/`, validator `wpv-vN/`, each with `logs/ tmp/ runtime/ workspace/`; numbered logs `NN-<name>.log` + `.exit` + `.sha256`; a `results.jsonl` row per binary (index,target,features,exe,exe_sha256,exit_code,result_line,passed,failed,ignored,log,log_sha256,wall_s,failing_tests,residue,git_porcelain,session,rerun_alone,started_at_utc,args,finished_at_utc); `00-lane.txt` as the human ledger; `covers:[…]` on shared-batch rows.
- Env per run: `HANDSHAKE_SURREAL_TEST_STORE_ROOT=<lane>/runtime`, `HANDSHAKE_WORKSPACE_ROOT=<lane>/workspace/run-NN-<target>` (fresh per run), `TMP`/`TEMP`/`TMPDIR=<lane>/tmp`; native fixture binaries need `HANDSHAKE_TEST_ARTIFACTS_ROOT=<Handshake_Artifacts>/handshake-test`; `hbr_e2e_smoke_test` needs `HANDSHAKE_TEST_GOV_REPO_ROOT`. Embedded SurrealDB only — no managed PG, no Docker, no SQLite. Feature sets: set A `surreal-test-support,test-utils`; set B `app-runtime,surreal-test-support,test-utils`; set C `inspector,…` (see `MT-141/wpv-v2/logs/exe-map.tsv`); lib `duckdb-flight-recorder,test-utils`.
- **The host is disk-bound, not CPU/RAM-bound**: Ryzen 5950X 16c/32t, 128 GB RAM (never below ~85 GB free), D: is a 16 TB SATA HDD (queue length ~12, 1200 % disk time during links; a full set-A `--tests` relink takes 2+ h; `link.exe` peaks ~13.5 GB RSS). Apply these HDD rules:
  1. **Fewer link rebuilds.** Validators build only the affected targets (`--test a --test b …`), never `--tests` blind, once a shared batch exists at the commit; later MTs cite that batch's rows by `exe_sha256`+`log_sha256` instead of rebuilding. A full relink is a one-time cost per commit.
  2. **Warm-target reuse across lanes.** Sequential lanes at the SAME commit reuse the same target (compatible-build reuse per CX-503I1 / IV-ART-003; record `target_reuse_note` in the verdict). Concurrent owners never share a mutable target. Do NOT create a fourth target.
  3. **Link concurrency.** Use `-j 2` for link-heavy `--no-run` builds on this HDD (4 parallel `link.exe` writing 13 GB PDBs thrash a single spindle); `-j 4` is fine for rustc-heavy phases. Announce the choice in the lane ledger.
  4. **Cargo is exclusive host-wide** (poll to zero `cargo.exe`/`rustc.exe`/`link.exe`, `WAIT` lines each minute, then run) **and test exes run one at a time** (pool size 1, print pool size + PID at phase start). Exec-only validator work MAY overlap another lane's exec phase, but reds observed under any concurrent load are suspect until rerun idle.
  5. **Keep debug-profile incremental compilation on** (default); do not add `CARGO_INCREMENTAL=0` or profile changes; `--locked` always.
- `cargo fmt` crate-wide fails on this host (os error 206, long path): per-file `rustfmt --edition 2021`, in its own `style(MT-NNN): …` commit.
- Check residue after every run (preserved stores under `<lane>/runtime/store-*`); record counts, do not delete.
- FORBIDDEN: repo-local or sibling `target/`, `D:/hsNN` dirs, `subst`, stashes, new worktrees/branches, `git checkout/switch/reset/clean/restore` (Operator authorization per turn only), `git worktree remove`, anything on C:, `cargo clean`, deleting other lanes' output.
- Exit `0xC0000409` = double panic; `3221226091`/`1073807364` = shutdown artifacts, not results.

## 6. Governance stance (Operator)

- No repo-governance paperwork or ceremony: no `just` recipes (they build the governance binary via `cargo run`), no receipts, dossiers, repomem, ACP, task-board polish, gov-check remediation. Repo governance is being abandoned.
- Only two surfaces matter: **state recovery** and **per-MT status**. Every cycle must leave, in `MT-NNN.json` (append-only, preserve keys, valid 2-space JSON, `updated_at_utc` bumped): builder `remediation_vN` and validator `validation_vN` (schema_id `hsk.mt_validation_verdict@1`) + `lifecycle.status`/`validator_verdict` flipped, `completed_by` set on PASS (must ≠ `claimed_by`), `validation_vN_transition` block. Field lists: see the session-3 brief §6 and existing records (MT-141 `remediation_v2`, MT-148 `validation_v2`, MT-079 `validation_v6` are good templates).
- **Status convention:** `lifecycle.status == lifecycle.validator_verdict` (`PASS_Vn` / `FAIL_Vn`); never `COMPLETED`. Non-PASS vocabulary: `FAIL_Vn`, `BLOCKED_ON_DEPENDENCY`, `PARTIAL_PENDING_OPERATOR_DECISION`, `NEEDS_REIMPLEMENTATION_Vn`, `READY_FOR_VALIDATION(_Vn)`.
- Sub-agents write MT-json through the junction but **never git-commit `.GOV`**. YOU snapshot after every remediation record and every verdict: `git -C wt-gov-kernel add .GOV/task_packets/…/MT-NNN.json; git -c core.safecrlf=false commit -m "gov(MT-NNN): record …"; git push origin gov_kernel`. Verify the file parses first. Model-created files: only MT-json edits; create no new `.md` unless the Operator asks (this brief was Operator-requested).
- Product commits by the builder only, on the feature branch, subjects `fix(MT-NNN): …` / `style(MT-NNN): …` — never `feat:`. `git push origin feat/WP-KERNEL-012` after every candidate (Operator waiver).

## 7. Open findings inventory

**MT-141 (validation_v2 → remediation_v2 at c0766697):** V2-F01 harness drain-stall Drop guard → fixed (c0603cbd), the 80-red class collapsed; V2-F02 row-shape/schema drift → writers bind record links matching `schema.surql` (validator static-checked); V2-F03 txn conflicts → bounded `retry_transaction_conflicts` at racing sites; V2-F04 `HbrPillar::Priv` + no `.GOV` runtime read; V2-F05 tests/ drift → per-item decisions in `remediation_v2.decisions`; V2-F06 env docs, node deps fail-closed (app/node_modules installed offline, gitignored); F500 `wsids` fixed in R2. Open: `mt045_lc06` budget (see §2). Builder's recommended validator rerun set is in its report inside `remediation_v2`.

**Native (behind the 28 BLOCKED + the 3 FAIL_V6):** F403 (§3.2); F500 (fixed by MT-141 R2 — re-verify natively); F401 atelier intake 401 (MT-141 R15 turned 400→401 at `stage.rs:198-202` — decide contract); F111 MT-111 401-vs-403 (resolve from Master Spec, record in MT-111.json, route into F403 work); FDIRTY (`test_e7_swarm_edit_proof`, `test_perf_large_rich` write tracked fixtures — MT-128/MT-045 must stop that; keep `TRACKED_FIXTURE_WRITE` as a finding, not a restore); FTAGS/FCANVAS backend causes NOT_INSPECTED; clippy 68 (§2); MT-088 env-contract (§3.3); native 191-binary sweep at `MT-108/wpv-n1` has 21 red binaries, owners listed in MT-079 `validation_v6.full_suite_clause` (WP-boundary item for the Integration Validator, not per-MT).

**MT-135 PASS_V2 non-blocking:** `hbr_obligations` arrays empty (HBR-INT-009 WP-level accounting for you).

**Topology residue (Operator to decide, do not delete):** local branch `tmp/mt148-freshcheck`; four stale registered git worktrees at ba03f563 under `Handshake_Artifacts/WP-KERNEL-012/MT-141/wpv-v2/tmp/hsk-session-worktrees-model-session-scheduler-*/` (created by product tests); `Handshake_Artifacts` root holds 15 noncanonical dirs (`debug`, `managed-pg`, `mt123-direct`, …) flagged by `artifact-root-preflight` — governance settlement debt, not product.

## 8. Lean mode (Operator-approved 2026-09-16, binding for all validators — paste verbatim into every validator brief)

- Full suite is NOT the per-MT gate. MT-141's `wpv-v2/logs/results.jsonl` (fd9060de) is the WP **shared proof batch** until `wpv-v3` (c0766697) completes and supersedes it; every other MT cites rows from it (`shared_proof_batch.owning_mt = "MT-141"`) and runs only its own `proof_targets` + the tests that anchor its remediation groups.
- Static first (grep the defect class across `src/**`), then the SMALLEST confirming binary — never the biggest binary that also covers the site.
- Reds: one solo rerun (`rerun_alone: true`, pool 1); one representative solo rerun per failure class is enough.
- No re-execution of already-green rows to satisfy "executed by this validator"; cite on `exe_sha256` + `log_sha256` + verbatim result line.
- Keep the `--nocapture` SKIP audit for binaries with silent SKIP branches.
- Contention makes evidence: solo reruns must be on an idle host (0 cargo, no other exec pool); reds observed while a builder compiles are suspect until rerun idle.

## 9. Sub-agent rules

- Fresh pair per MT/batch (`general-purpose`, background). **Model choice (Operator, 2026-09-18, token economy):** builders `opus` (product fixes need it); validators `sonnet` (mechanical run/parse/cite work with explicit checklists) — escalate a validator to `opus` only for a contested adversarial ruling. No reasoning-effort knob is exposed to the orchestrator. Ask agents for a ≤20-line structured final report (the MT-json carries the detail). Tick 20 min while a lane is building/linking, 8 min while it executes or a builder is in a red-fix loop. Session ids: `KERNEL_BUILDER-<UTC yyyymmdd-hhmmss>`, `WP_VALIDATOR-<tag>-<UTC>`. Release the previous pair before spawning the next.
- Direct agent↔agent messaging does not resolve — **you are the relay**: verify a builder's candidate (HEAD == SHA, tree clean, `ls-remote` == SHA, 0 cargo procs, contract appended & parses) → snapshot → spawn the validator; on FAIL forward `remediation_required` to a fresh builder; on PASS release both, snapshot, move on.
- Agents may end their turn while a background command runs and are resumed on completion; a driver that isn't a tracked child does NOT resume them. Your tick must detect a lane whose driver finished with no agent activity and `SendMessage` it.
- Verify agent claims against the ledger yourself (count `exit_code!=0 || failed>0`; compare verdict JSON to lane logs) before relaying counts — done every time this session, always matched.
- Require every green to carry its own log + exit + verbatim `test result:` line; `NOT_INSPECTED`/`UNVERIFIED` labels; never weaken/`#[ignore]` assertions; builder logs are triage input for validators, never proof.
- Give agents absolute paths; name the other lanes' targets so they never touch them; `git status --porcelain` after every run and report tracked-file dirt (`TRACKED_FIXTURE_WRITE`) instead of restoring it.
- Validators must read Codex + WP_VALIDATOR_PROTOCOL; builders must read Codex + KERNEL_BUILDER_PROTOCOL; each acknowledges in its report.
- Builder briefs: include the Operator stance, HDD rules §5, the exact contract fields to write, and "commit as you go / push after every candidate". Validator briefs: include lean mode §8 verbatim, the affected-set rule, the adversarial checklist, and the target/lane to use.

## 10. Monitoring

`CronCreate` every 8 min (avoid :00/:30; e.g. `3,11,19,27,35,43,51,59 * * * *`). Each tick, ONE PowerShell command: feature HEAD + dirty paths; gov dirty; cargo/rustc/link/test-exe processes + command lines; free RAM + D: free; per active lane: newest log + mtime, `00-lane.txt -Tail 4`, `results.jsonl` row/red count; MT lifecycle + `*_vN` keys for the active MTs; repo-local `target/` scan and `D:/hs*`. Stall = no artifact/proc change > 25 min → `SendMessage` a status request. Report 2–3 lines per tick. Re-create the cron whenever the lane set changes; delete it when all lanes close. Never kill a process you did not start.

## 11. Mistakes (cumulative; don't repeat)

1. Read `handshake_main/.GOV` copies of the Codex/protocol first — they are stale; read the kernel copies.
2. Ran `just repomem`/`just phase-check` before reading the Operator stance — no `just`.
3. (session 3) Two validators wrote `lifecycle.status = "COMPLETED"`; convention is `status == validator_verdict`.
4. (session 3) Relayed "0 reds" from an agent's word; verify ledgers first.
5. (session 3) Two concurrent cargo builds → 64-min relink; now hard rule: one cargo.
6. (session 3) Validator driver STOP-loop bug orphaned binaries; ask validators to print pool size and PIDs.
7. Assumed the crashes were ours; the event log showed Windows Update — check `Get-WinEvent` (Kernel-Power 41/1074/6008) before theorising.

## 12. Do / Don't

DO: verify remotes with `ls-remote`; snapshot every MT-json change on `gov_kernel` and push; keep `status == validator_verdict`; challenge agent tallies against `results.jsonl`; one cargo, `-j 2` for link-heavy builds; reuse warm targets at the same commit; put lean-mode text verbatim in every validator brief; name the dependency in every BLOCKED verdict; keep TRACKED_FIXTURE_WRITE as a finding.
DON'T: run `just`; touch `handshake_main`/`wt-gov-kernel` product trees; create targets; run the full suite per MT; re-exec green rows; let a validator cite builder logs; let an agent commit `.GOV`; use `feat:` subjects; `git checkout/restore` without same-turn Operator authorization; trust `origin/*` tracking refs; write to C:.

## 13. Artifacts to read first

- MT-141: `MT-141.json#validation_v2`, `#remediation_v2`; `../Handshake_Artifacts/WP-KERNEL-012/MT-141/{wpv-v2,kb-v2,wpv-v3}/{00-lane.txt,results.jsonl}`; `wpv-v3/logs/01-affected-map.tsv`.
- Native: `MT-079.json#validation_v6` (+ `clippy_classification`, `full_suite_clause`), `MT-088.json#validation_v6.remediation_required` (MT-088-R6-2), `../Handshake_Artifacts/WP-KERNEL-012/MT-079/wpv-v6/logs/01_clippy_errors_attributed.json`, `../Handshake_Artifacts/WP-KERNEL-012/MT-108/wpv-n1/logs/{311_pass3_summary.json,results.jsonl}`.
- MT-148: `MT-148.json#validation_v2` (cold-export precedent).
- Session-3 brief for the F403 / N1B detail: `HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-17.md` §3, §7, §13.

## 14. First actions

1. `git -C wtc-native-editors-v1 status/log`, `git -C wt-gov-kernel status/log`, `ls-remote` both, process scan (cargo/rustc/link/test exes), `Get-WinEvent` shutdown check if anything looks cut off — confirm §2.
2. Inspect `MT-141/wpv-v3/00-lane.txt` + `results.jsonl` + `logs/10-norun-setA.log.exit`. If `validation_v3` exists in MT-141.json: verify it against the ledger, snapshot, and go to step 4. Otherwise spawn a fresh `WP_VALIDATOR-B3-<UTC>` that RESUMES the wpv-v3 lane (brief = §5 + §8 + affected-set rule + adversarial checklist: V2-F02 vs `schema.surql`, V2-F03 replay-vs-serialize, V2-F04 no `.GOV` reads, V2-F05 test-vs-product decisions, set-B product changes in `state_recovery.rs`/`locus_store.rs`, lc06 budget ruling; Spec-Realism sub-rules 1–3; verdict fields per §6).
3. Arm the 8-min tick. Relay.
4. On MT-141 PASS: spawn the native batch builder (§3.3 a–d) on the kb-b2 target with `SESSION_MT_BATCH` = {clippy-owning MTs, MT-088, MT-131, env-contract sweep}; per-file rustfmt; `fix(MT-NNN):` commits; push; `remediation_vN` on each touched MT. Then one fresh validator for the batch (clippy `-D warnings` clean from a clean export, MT-088 live proof now runnable, MT-131 test, build halves cited). On MT-141 FAIL: fresh builder with the validator's `remediation_required` groups.
5. Then §3.6 order: MT-124/125/132 RED halves (ask Operator for the tree-rewrite authorization each turn) → MT-045 → MT-142 idle rerun → F403 builder → 28 BLOCKED re-verdicts (batch by dependency class, one validator lane per class, exec-only on the warm target).
