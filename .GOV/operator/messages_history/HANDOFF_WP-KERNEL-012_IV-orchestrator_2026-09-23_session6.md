# HANDOFF — WP-KERNEL-012 IV orchestration (2026-09-23, session 6)

You are the INTEGRATION VALIDATOR acting as orchestrator for `WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1`. You steer sub-agents (KERNEL_BUILDER, WP_VALIDATOR, read-only audit agents); you do not write product code. This file supersedes the session-5 handoff (`HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-23_session5.md`), which stays as history. Its §5 host rules and the session-6 notes appended there are consolidated here.

**Governance pin:** WP-KERNEL-012 closes under governance as of `gov_kernel` commit `896f4e15` (`packet.json.governance_pin`). The Operator is starting a repo-governance refactor in `wt-gov-kernel` with another agent right after this session. That refactor applies from WP-KERNEL-012-bis onward. Don't re-judge settled 012 verdicts under the new rules. Operator decisions recorded after the pin (MT JSON, this file) still apply.

Read, from the pinned governance (commit `896f4e15`; the live files may be mid-refactor, so use `git show 896f4e15:<path>` if they have changed): Codex `.GOV/codex/Handshake_Codex_v1.4.md`, `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`, root `CLAUDE.md`/`AGENTS.md`. Global instruction `[GLOBAL-REMEDIATE]` and Codex `[CX-EXEC-012]` apply: this is remediation, so no research runs, audits, or ROI listing unless the same fix failed twice.

## 0. Read first: what cost the most this session, and the rules against it

1. **Activity reported as progress.** For about 3.5 h the orchestrator reported "healthy lanes" while commits and verdicts stayed at 0. Rule (Codex CX-EXEC-006, ORC-OUT-005): report only PASS count, new verdicts, new pushed SHAs. If none changed, the report says `no direct progress`.
2. **Broad validation instead of per-MT verdicts.** The validator got 33 MTs at once, then built broadly, ran a 0-test filter, and hung 40 min on an out-of-scope test. Rule (CX-EXEC-008, ORC-OUT-002, WPV-OUT-001..004): a queue of 3–5 MTs, each verdict written immediately, full suite only at the WP boundary.
3. **Compiled work held uncommitted** (109 files for hours). Rule (CX-EXEC-007, KB-OUT-001): commit and push per MT when it compiles.
4. **Builders ran expensive tests** (Operator decision 16:10). Builders run `cargo check`/clippy plus at most ONE cheap focused test. The validator is the single test runner.
5. **Unsteerable agents.** Agents in long foreground commands can't read messages (CX-EXEC-009): background + poll ≤60 s.
6. **Environment guesses before evidence.** I moved runtime to C: and toggled sync settings on theory; both caused or failed to fix hangs. The real causes came from **process dumps** (§4). Rule: on a zero-CPU hang, ProcDump the process and stackwalk it BEFORE changing the environment.

## 1. Goal

Every MT at validator-proven `PASS_Vn` → whole-WP IV verdict (full suite + HBR/Argus/UserManual/diagnostics closure) → merge to `main` (backup push first; `sync-gov-to-main`; push `origin/main`). No new WPs for 012 remediation; extra MTs go inside 012 (Operator).

## 2. Verified state (2026-09-23 ~18:20 local)

- Product: `feat/WP-KERNEL-012` = **`72498260`** (pushed, `ls-remote` verified), worktree `wtc-native-editors-v1` clean at push. Chain since session start: `0cfbff64` → `bea9496d` (C3 residuals + MT-153..157, 104 files) → `9a2a8d7e`/`88822974`/`e9973e8c` (re-pin + MT-088 test fix) → `c21a5be0`/`51d9f40f` (MT-154 test lock order + clippy) → `5f31aa07`/`7e73eb03`/`72498260` (schema batch incl. CRDT delete permission, MT-158 Locus ownership, index DDL in its own bootstrap transaction, ONE re-pin; CRDT delete cascade; MT-158 Locus job path).
- Backup branch `backup/WP-KERNEL-012-wtc-dirt-20260905` = `bea9496d` (fast-forwarded on Operator instruction; update again at next checkpoint).
- Governance: `gov_kernel` pushed, clean (pin commit `c9bc29e8`).
- **MT tally: 158 MTs, 118 PASS, 40 open.** This session: MT-131 PASS_V3, MT-088 PASS_V8, MT-108 PASS_V7 (all at verified candidates, binaries rebuilt from the named export).
  - FAIL (27): 008 023 027 033 034 036 046 064 065 066 067 068 074 079 098 113 116 117 120 121 122 127 128 130 140 141 143 — remediations are recorded in code, mostly the native-live class (NOT-RUN-MASKED, fixture auth/workspace-create classes fixed in C1/C2/C3). They need the validator run, not new code.
  - BLOCKED (3): 026 070 111. PARTIAL (4, scheduled one-offs): 045 (release perf), 124/125 (RED-half reversal, standing Operator authorization), 142 (idle-host swarm stress).
  - READY_FOR_VALIDATION (5): 153 154 155 156 157. IN_PROGRESS (1): **MT-158** (new this session, Operator option A: Locus job path via `POST /jobs` under the account session + owner permissions on mt_iterations/dependencies/Locus storage_graph_anchors).
- **Open builder item at handoff:** change the bootstrap so each `DEFINE INDEX` runs outside an explicit transaction (one index build commits at a time; `schema_applied` receipt in a final short tx). Commit `5f31aa07` still wraps all 816 index DDLs in ONE transaction, which may not remove the hang trigger (§4.2). Then sets A/C + clippy, push, MT-158 → READY_FOR_VALIDATION.
- Deferred inside MT-154: rev-158/159 upgrade tests + `bootstrap_resumes_exact_current_schema_applied_state` (DEFERRED-SLOW-IO); `bootstrap_is_concurrent_restart_safe…`, `bootstrap_rejects_lower_or_divergent_lineage` (HANG-INTERMITTENT, expected fixed by §4.2). `mt138_full_schema_atelier_noop_matches_bounded_projection`: failure text never captured; builder found no static divergence (UNVERIFIED hypothesis: its own 300 s timeout under load).

## 3. Operator decisions this session (binding)

1. Output-first rules → Codex CX-EXEC-003B/006..011 + ORC-OUT/IV-OUT/CODER-OUT/WPV-OUT/KB-OUT/AM-OUT (gov `98523902`); remediation scope → Codex CX-EXEC-012 + global `[GLOBAL-REMEDIATE]` (gov `cbe5dbc9`).
2. D-154-1..3 and the C3 silent-deny rule are resolved from the spec (MT-154 `spec_resolution`, gov `b9d011cd`). Silent-drop writes → constant-shape 403. Legacy `/workspaces/:ws/documents` retired. DuckDB FR reads stay behind fr.read. atelier_*/work_packets/micro_tasks/global prefs are owner-stamped, private by default.
3. MT-158 inside 012 (option A), not deferred.
4. Builders run no expensive tests; the validator is the single test runner.
5. No new git worktrees or branches, including indirectly through tests (check `git worktree list` before/after runs).
6. Commits without approval, always `git commit -- <explicit paths>`; push after each.
7. Governance pin at `896f4e15` (gov `c9bc29e8`).
8. Order after 012: merge 012 → repo-governance refactor (other agent; authoritative) → WP-KERNEL-012-bis (implements it) → WP-1, WP-CKC rework → WP-KERNEL-017.

## 4. Findings (root causes; evidence paths under `Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x/logs/`)

1. **Hang A, default sync:** stackwalk `hang-run15.stacks.txt`, `hang-run18.stacks.txt`: `surrealdb_core::kvs::rocksdb::commit_coordinator::CommitBatcher::run` (commit_coordinator.rs:362) → rocksdb `SyncWAL` (db_impl.cc:1610) → `WinWritableFile::Sync` (io_win.cc:986), blocked in the OS file sync under disk load; every other thread waits on it. The coordinator exists only in `SyncMode::Every`. Test env fix: `SURREAL_DATASTORE_SYNC=never` (surrealdb-core-3.2.0 `kvs/rocksdb/cnf.rs:598/759/863-867`), except durability tests.
2. **Hang B, sync=never:** `hang-run19.stacks.txt`: the only active thread is SurrealDB's index builder (`kvs/index/builder.rs` run_acquired:1353 → mark_durable_online:719 → update_owned_build_state:589), retrying a "transient conflict" forever with 100 ms sleeps (:585/:822-832). The static audit found that all 816 `DEFINE INDEX OVERWRITE` statements run inside the single bootstrap BEGIN..COMMIT (schema.surql:25/6569, schema.rs:4513-4528). Each DEFINE INDEX spawns a blocking builder (define/index.rs:228-235) with no deadline. RocksDB `Busy`/`TryAgain` map to retryable conflict (kvs/err.rs:131-132). Fix in progress: indexes built outside the long transaction (§2). Product relevance: a fresh install could hang the same way.
3. **C: runtime** (test stores on the QLC SSD) also hung. Reverted: test runtime roots stay on D:.
4. **Stale-artifact trap:** exports sharing one target with the same package ids can link stale code (git archive stamps files with commit time). Confirm every cited binary's log shows `Compiling <crate> (…export-<sha>…)`.
5. **Test-order precondition:** `external_artifact_root()` didn't create `<root>/wp-kernel-012`; fixed in `88822974`.
6. **Host load:** Steam downloads (C:) and the Operator's Obsidian vault agents (D:) saturated the disks; the Operator's vault work has priority on D:.
7. **Validator verification gap caught:** a PASS was written on a FAILED run; the orchestrator verification caught it. Always verify: status == verdict, completed_by ≠ claimed_by, a passing proof record for every required command, binary provenance.

## 4b. CORRECTION (2026-09-23 20:40): the embedded engine NEVER reads `SURREAL_DATASTORE_SYNC`

Verified by audit in the SDK source: `ConfigMap::from_env()` has no caller in the embedded path (surrealdb-3.2.0 `engine/local/native.rs:131` → `kvs/ds/builder.rs:82` starts from `ConfigMap::empty()`); the only route is a `?sync=never` query string on the store path (`kvs/ds.rs:582-590`), which Handshake does not set (`src/storage/surreal.rs:1020`). So EVERY test run today used fsync-per-commit, and the "sync=never" results (including the index-builder hang B diagnosis from run 19) must be re-checked. Hang A (OS `NtFlushBuffersFile` never returning, kernel/filter side) reproduces on the HDD AND the SSD: hang-run15/18/21/22/24/25. Fix in flight: a builder-added TEST-ONLY switch, env `HANDSHAKE_TEST_SURREAL_SYNC=never` → store path `?sync=never` in the test-support openers only (production keeps `Every`), plus a 900 s bootstrap watchdog (typed `BootstrapStalled`). The rules below that name `SURREAL_DATASTORE_SYNC` mean this switch. Operator action pending: as admin, run `fltmc filters` and `(Get-MpPreference).ExclusionPath` to find the filter stalling the flush.

## 5. Build, disk and test-env rules

- **Operator decision 2026-09-23 23:20: C: grant raised from 100 GB to 150 GB.** Stop cargo below **192 GB** free on C: (baseline 341.9 GB minus 150 GB), still measured by free space. This supersedes the 248 GB stop line below. Purpose: one full union build + nextest run of all open MTs without chunking. (HARD; supersede session-5 §5)

- One cargo per physical disk. C: grant `C:\.target\WP-KERNEL-012\` for build targets (100 GB cap, stop below 248 GB free, measured by free space). D: HDD: builder target `Handshake_Artifacts/WP-KERNEL-012/MT-109/kb-c3/target` (warm). The builder links no test binaries on D: except its single pin-measure.
- Test runtime roots on D: under the lane; `HANDSHAKE_ARTIFACTS_ROOT = D:/…/Handshake_Artifacts`; `SURREAL_DATASTORE_SYNC=never` (durability tests: default); `--test-threads=2`; every run under a wall-clock timeout.
- Zero-CPU >120 s: `procdump64 -accepteula -nobanner -ma <own pid> <lane>/logs/hang-<run>.dmp`, stop your own process, record HANG, move on. Stackwalk: `dump_syms --store <lane>/symstore <exe>.pdb` then `minidump-stackwalk --symbols-path <lane>/symstore <dmp>`.
- Validator exports: re-export each new SHA over the same folder path to keep the target warm; delete superseded exports.
- Every cargo call: `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `--locked`. No python. No `just`. Non-interactive only.

## 6. Tools (`../gov_runtime/tools/`, manifest `TOOLS.json`)

cargo-nextest 0.9.146, sccache 0.18.0, ast-grep 0.45.3, Sysinternals ProcDump/Handle (`-accepteula -nobanner` mandatory), minidump-stackwalk 0.27.0, dump_syms 2.3.9 (the last two added after the manifest was written; add them to `TOOLS.json`). rust-lld ships with toolchain 1.97.1. All installed, none adopted into lane scripts yet. The proposed authority placement (Codex pointer to the manifest; nextest in the WPV protocol; ast-grep in KB/Coder; ProcDump/Handle in ORC/IV; machine facts in a host profile) goes to the governance refactor. Suggested adoption: nextest (automatic hang termination), then rust-lld (link time), then ast-grep, then sccache.

## 7. Lanes at handoff

- **KERNEL_BUILDER-C4** (session agent; gone with this session): finishing the per-index-transaction change + gates. Ledger `Handshake_Artifacts/WP-KERNEL-012/MT-154/kb-c4/00-lane.txt`. If you're a new session: a fresh builder resumes from the ledger and §2.
- **WP_VALIDATOR-C3X** ended (ledger `…/MT-109/wpv-c3x/00-lane.txt`; `queue-proof-commands.txt` holds the exact proof commands for MT-153..158; `failure_groups.json`). Spawn a FRESH validator on the next pushed SHA.

## 8. Next actions (in order)

1. Builder pushes the per-index-transaction SHA; MT-158 → READY_FOR_VALIDATION. Verify ls-remote + clean tree + MT JSON; snapshot MT records.
2. Fresh validator (Sonnet; brief: Codex + WPV protocol + CLAUDE.md + §5 + lean mode + adversarial checklist) on that SHA: re-export over the same path, build the test crates the queue needs, confirm the bootstrap tests no longer hang, then verdict MT-155/156/157/154/153/158, then 141/008/120/033, then 026/070/111.
3. One grouped native live run for the 27 NOT-RUN-MASKED native MTs (clean export, C: target).
4. One-offs: MT-124/125 RED halves (patches in `…/MT-124/kb-c1/red-half.patch`, `…/MT-125/kb-c1/red-half.patch`; re-check `git apply --check` at the new HEAD), MT-045 release perf, MT-142 idle-host stress, the deferred rev-158/159/bootstrap_resumes tests.
5. WP boundary: full suite on the final SHA + HBR/Argus/UserManual/diagnostics closure → IV verdict → artifact hygiene + cleanup of `C:\.target\WP-KERNEL-012` → merge.
6. Output tick every 10 min (cron at off-minutes): PASS count, new verdicts, new SHAs only. 20 min without output → demand; 30 min → replace from the ledger.

## 9. Do / Don't

DO: commit and push per MT as it compiles; verify every verdict against its log and binary provenance before committing; keep status == validator_verdict; name the dependency in every BLOCKED verdict; dump before stopping a hung process; commit governance only by explicit path (other agents work in `wt-gov-kernel`); relay findings between lanes (agents can't message each other).
DON'T: run broad suites before the WP boundary; let builders run expensive tests; change the test environment on theory; accept an agent plan that delays commits or verdicts; create worktrees or branches (including via tests); use `git add -A`/bare `git commit` in `wt-gov-kernel`; touch another agent's processes without verifying the command line; re-judge settled verdicts under post-pin governance.

## 10. Refactor plans (Operator-owned; for the next sessions)

**Repo-governance refactor (authoritative).** Done by another agent from the Handshake Creation Template (HSRepoTemplate): machine-readable authority, modular/component rules, tools manifest pattern, host profile, output-first and remediation-scope rules carried forward. Scenario suite for testing it: `.GOV/operator/messages_history/HANDOFF_TEMPLATE-SCENARIO-TESTS_2026-09-23.md`. WP-KERNEL-012-bis implements whatever that refactor makes authoritative. Where this draft conflicts with it, the refactor wins.

**WP-KERNEL-012-bis Refactor: DRAFT scope proposal (not a contract; the stub is created after the governance refactor lands).**
- Base scope:
  1. Modular product code: a small shared core crate (typed state, the SurrealDB/EventLedger authority layer, pillar interfaces) plus module crates (Notes/editor, code index/IDE, knowledge, model runtime, sandbox/process, Argus, UserManual; Studio later) that depend only on core, never on each other directly. Palmistry and `diag_ring` stay separate crates.
  2. Remove redundant and dead code from a tool report, not by guesswork. Candidates to verify: `app/src-tauri` (legacy Tauri shell vs Codex CX-008-VIS), `src/frontend/toolkit_spike`, `storage/postgres` leftovers, duplicate helpers, unused features and dependencies.
  3. Everything on SurrealDB/EventLedger (the declared DuckDB diagnostic projection only).
  4. Every module wired to, and declaring its use of, the Flight Recorder, internal_diagnostics and the Palmistry watcher.
- Additions: tests move with their module, split into fast unit and slow DB/live tiers; one shared test-support crate with timeouts; fix the bootstrap index/transaction design properly if not closed in 012; clear the ~396 pre-existing core clippy errors as code moves; an enforced dependency-direction rule; nextest/rust-lld/ast-grep as the default toolchain; product/governance boundary audit (`hbr`, `role_mailbox_v1`, `mt_executor`, `spec_router` vs Codex CX-211); generated module topology from Cargo metadata.
- Execution: one module per MT in dependency order (core first), move-only MTs separate from cleanup MTs, the suite green after every MT, core + storage first so WP-1/WP-CKC can rebase early.
- Inventory at `72498260` (git): `handshake_core` 726 src + 428 test .rs files (`kernel/` 160); `handshake_native` 226 src + 201 test .rs files; Cargo crates: handshake_core, handshake_native, diag_ring, palmistry, app/src-tauri, toolkit_spike.

## 11. Authority and record changes this session (all on `gov_kernel`, pushed)

`b9d011cd` MT-154 spec resolutions · `1e8cd0c6` MT-131 PASS · `98523902` output-first rules (Codex + 6 protocols + session-5 §11 fix) · `cbe5dbc9` CX-EXEC-012 · `4536558c`/`7a0a1223`/`3ed57211` builder MT records · `9eabcb3e` template scenario suite · `74406c99`/`3a2cf4b4`/`9e08f7b0`/`71f71a89`/`896f4e15` session-5 handoff amendments · `4a3cb4e5` MT-088/108 PASS · `b61a37c6` MT-158 + packet · `c9bc29e8` governance pin · this file. Outside git: global `~/.claude/CLAUDE.md` + `~/.codex/AGENTS.md` got `[GLOBAL-REMEDIATE-001..005]`; tools installed under `../gov_runtime/tools/`.
