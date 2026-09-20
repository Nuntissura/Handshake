# HANDOFF — WP-KERNEL-012 validation/remediation orchestration (2026-09-17, session 3)

You are the INTEGRATION VALIDATOR acting as orchestrator for `WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1`. You spawn and relay between sub-agents (KERNEL_BUILDER, WP_VALIDATOR). You do not write product code yourself. Read in full before acting: `.GOV/codex/Handshake_Codex_v1.4.md`, `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`. Operator instruction overrides protocol ceremony (§6 below).

## 1. Goal (unchanged)

Bring every MT of WP-KERNEL-012 to a validator-proven PASS. One MT (or one shared batch) at a time with a fresh KERNEL_BUILDER + WP_VALIDATOR pair. You orchestrate, relay, monitor with a repeating tick; you never sit idle.

## 2. Verified state at handoff (2026-09-17 ~00:50Z)

- Product branch `feat/WP-KERNEL-012` @ **`ba03f5638e3f6b1903cb2829c1e846958d0dbf42`** (= "C-prime"), worktree `wtc-native-editors-v1` **clean**, pushed (GitHub tip verified by `ls-remote`). NOTE: the worktree's local tracking ref `refs/remotes/origin/feat/WP-KERNEL-012` is stale at `6d5d705f` (no fetch since 2026-08-28); always verify the remote with `git ls-remote origin refs/heads/feat/WP-KERNEL-012`, never trust `origin/...`.
- Governance `gov_kernel` @ **`84136b3411e39a8e4d9e5a49cd939c6dcb5fb569`**, `wt-gov-kernel` clean, pushed.
- `handshake_main` local `main` is 1 Operator docs commit ahead of `origin/main` — not yours to push.
- 0 cargo/rustc processes. No cron alive. No agents alive.
- Two cargo targets exist and are Operator-approved: warm shared validator target `../Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-v15/target` (holds set A `surreal-test-support,test-utils`, set B `app-runtime,…`, set C `inspector,…`, lib `duckdb-flight-recorder,test-utils`, native `handshake-native` tests, and `handshake_core.exe` — all at **ba03f563** except the set A/B/C/lib test exes which are at **fd9060de**, functionally identical: C-prime changed only trait signatures + a test module + a native test rename) and the builder target `../Handshake_Artifacts/WP-KERNEL-012/MT-135/kb-b2/target` (55 GB, everything at ba03f563). Do NOT create a third target.
- The W: subst alias is gone (reboot); do not recreate it. The Operator deletes `Handshake_Artifacts` at WP end.

### MT tally (152)
- **107 PASS** (status == validator_verdict == `PASS_Vn`).
- **9 READY_FOR_VALIDATION (non-native backend MTs, DO FIRST):** MT-112, MT-118, MT-120, MT-139, MT-142, MT-145, MT-146 (all `_V4`), MT-148, MT-149.
- **1 FAIL_V2:** MT-141 (`B14-…Port-The-Backend-Test-Suite-Off-sqlx-Onto-The-Embedded-SurrealDB-Store`) — builder round v2 approved (§3).
- **27 BLOCKED_ON_DEPENDENCY** (native MTs, verdicts at ba03f563 by N1B): MT-008 023 026 027 033 034 036 046 064 065 066 067 068 070 074 108 111 113 116 117 121 122 127 128 130 140 143. Each `validation_vN` names its dependency: F403 / F500 / F401 / F111 / FDIRTY / FTAGS / FCANVAS (see §7).
- **7 PARTIAL_PENDING_OPERATOR_DECISION:** MT-045 (supervisor-only perf proofs), MT-079/MT-098 (clippy/release-build halves), MT-088 (palmistry LB-2), MT-124/MT-125/MT-132 (red halves). Operator decision: **NO half runs — the NOT_RUN halves must be executed for real**, no waivers.
- **1 NEEDS_REIMPLEMENTATION_V1:** MT-131 (PT-131-2 "gate on the prompt-modal frame" has no executable test; product fix present).

## 3. Operator decisions (2026-09-17, binding)

1. **MT-141 builder round v2: scope waived and approved.** Builder may edit `src/storage/tests.rs` (the src test harness), the atelier/knowledge/loom/event-ledger product sites named in `validation_v2.reds`, and MT-141's own `tests/`. Record every out-of-allowed-path file in `remediation_v2.out_of_scope_edit_waiver` with `granted_by: "OPERATOR (session waiver 2026-09-17, relayed by orchestrator)"`.
2. **F403 — unblock it.** Native Flight-Recorder/EventLedger reads get `403 HSK-403-PROTECTED-RESOURCE` from `src/backend/handshake_core/src/api/authority.rs::authorize_request` (MT-109/MT-141 authority layer) because native proof support (`src/frontend/handshake_native/tests/backend_proof_support/mod.rs`) only sends `x-hsk-session-token` and never does the `/authority/session` exchange. The builder must make the native proofs pass: root-cause first (is the product policy correct and the fixtures stale, or is the policy over-strict for the FR session token?), then fix on the correct side and record the reasoning. Outcome required: the 27 BLOCKED verdicts' F403 dependency resolves.
3. **MT-111 401-vs-403:** the Operator does not have context on this. Resolve it yourself from the Master Spec (`.GOV/spec/SPEC_CURRENT.md` → `master-spec-v02.206/INDEX.json`) and the MT-111 contract: MT-111 asserts an unauthenticated request yields 401; the backend authority middleware returns 403. Decide which is the contract (spec wins; if the spec is silent, keep the MT-111 contract and fix the middleware order/status), record the ruling in MT-111.json, and route the fix into the F403 work.
4. **Fixture restore authorized and DONE** (`tests/fixtures/swarm_edit_proof_log.txt` restored to HEAD at handoff). The root cause — `test_e7_swarm_edit_proof` (and `test_perf_large_rich`) write into tracked fixture files — belongs to MT-128 (and MT-045): tests must not write tracked fixtures. Until fixed, every native run re-dirties the tree; the MT-066 dirty-tree gate then trips. Ask the Operator before any further `git checkout --` (CX-108 needs same-turn authorization each time).
5. **7 PARTIAL MTs: no half runs.** Run the missing halves (clippy, release builds, palmistry LB-2, supervisor-only perf) for real and write full verdicts.
6. **Order: the 9 non-native READY MTs first**, then MT-141 v2, then F403-unblocking + the 27 BLOCKED re-verdicts, then the 7 PARTIAL, then MT-131.

## 4. Hard location rules

- Work ONLY inside `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-native-editors-v1` (branch `feat/WP-KERNEL-012`). `.GOV/` there is a live junction to `wt-gov-kernel/.GOV` — read/write MT contracts through it.
- `handshake_main` and `wt-gov-kernel` product trees are stale mirrors; do not work in `main`. A sub-agent that resolves a relative path against the shell cwd can accidentally read `handshake_main` — two agents did (read-only, harmless); tell them to use absolute paths.
- MT contracts: `wtc-native-editors-v1/.GOV/task_packets/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-NNN.json` (large; extract with `node -e`, never `cat`).
- Protocols agents must read in full and acknowledge: `.GOV/codex/Handshake_Codex_v1.4.md`, `.GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md` (builder), `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md` (validator). Spec via `.GOV/spec/SPEC_CURRENT.md` → `master-spec-v02.206/INDEX.json`.

## 5. Cargo / artifact rules (HARD)

- Everything under `D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-012/MT-NNN/<lane>/…` (`HANDSHAKE_ARTIFACTS_ROOT` absolute). Lanes: builder `kb-vN/`, validator `wpv-vN/`, each with `logs/ tmp/ runtime/ workspace/`; numbered logs `NN-<name>.log` + `.exit` + `.sha256`; a `results.jsonl` row per binary (target, exe, exe_sha256, exit_code, result_line, passed/failed/ignored, log, log_sha256, session, wall_s, rerun_alone, residue counts); `00-lane.txt` as the human-readable ledger.
- Env per run: `HANDSHAKE_SURREAL_TEST_STORE_ROOT=<lane>/runtime` (pre-exists), `HANDSHAKE_WORKSPACE_ROOT=<lane>/workspace/run-NN-<target>` (fresh per run), `TMP`/`TEMP`/`TMPDIR=<lane>/tmp`; native fixture binaries additionally need `HANDSHAKE_TEST_ARTIFACTS_ROOT=<Handshake_Artifacts>/handshake-test` (the fixture rejects any other evidence root — see `MT-108/wpv-n1/logs/013_env_invalid_rows.json`). Embedded SurrealDB only — no managed PG, no Docker, no SQLite.
- Targets: validators execute PRE-BUILT exes from the warm target (exec-only, no cargo). The builder builds on the kb-b2 target. **Cargo on a target is exclusive**: never two cargo invocations on the same target; a cargo build on the warm target while validators execute from it is only allowed for a single `--bin handshake_core` relink (it does not touch `deps/*.exe`) and must be announced. Two cargo processes on DIFFERENT targets are allowed but starve each other on this host (a 20-min build became 64 min) — sequence them when you can.
- After a builder commits a new candidate, a validator must rebuild what it cites: `cargo test --no-run` for the affected feature set(s), then exec. Reuse builds; bundle (one `--no-run` build, then per-target exec); rerun only affected targets; never the whole crate blind.
- `cargo fmt` crate-wide fails on this host (os error 206, long path): per-file `rustfmt --edition 2021`, in its own `style(MT-NNN): …` commit so the validator can prove fmt-only.
- Check residue after every run. The `src/storage/tests.rs:177` Drop guard preserves stores under `<lane>/runtime/store-*` on drain-stall (`HANDSHAKE_TEST_STORE_CLEANUP_FAILURE`); until V2-F01 is fixed, expect hundreds — record counts, do not delete.
- FORBIDDEN: repo-local or sibling `target/`, `D:/hsNN` short dirs, `subst`, stashes, new worktrees/branches, `git checkout/switch/reset/clean/restore` (Operator authorization per turn only), `git worktree remove`.
- Host facts: D: is a SATA HDD; embedded-store tests are disk-bound (core_data 2.5 h, event_ledger 80 min alone); `0xC0000409` (fail-fast abort) = a double panic (test assertion + Drop-guard panic during unwinding) — the process dies before its result line; `3221226091`/`1073807364` exits are shutdown/debug-terminate artifacts, not test results.

## 6. Governance stance (Operator)

- No repo-governance paperwork or ceremony: no `just` recipes (they build the `handshake` binary via `cargo run` — `just validator-startup` burned a cargo build before I killed it), no receipts, dossiers, repomem, ACP, task-board polish, gov-check remediation. Repo governance is being abandoned.
- Only two surfaces matter: **state recovery** and **per-MT status**. Every cycle must leave, in `MT-NNN.json` (append-only, preserve keys, valid 2-space JSON, `updated_at_utc` bumped): builder `remediation_vN` (session, base_sha, candidate_sha, fix_shas, style_sha, files_changed, commits, proof_environment, groups, proofs {log/exit/sha256/result_line/counts/failing_tests}, out_of_scope_edit_waiver, not_run, not_inspected, residue) and validator `validation_vN` (schema_id `hsk.mt_validation_verdict@1`, verdict, validated_by/at, validated_commit, basis, shared_proof_batch, proofs, resolved_prior_findings, independent_findings, spec_realism_gate, remediation_required with test-anchored steps, residual_uncertainty, residue) + `lifecycle.status`/`validator_verdict` flipped, `completed_by` set on PASS (must ≠ `claimed_by`), `validation_vN_transition` block.
- **Status convention:** `lifecycle.status == lifecycle.validator_verdict` (`PASS_Vn` / `FAIL_Vn`); do not let validators write `COMPLETED` (two of them did; I aligned 15 files with a `status_alignment_*` note). Non-PASS lifecycle vocabulary in use: `FAIL_Vn`, `BLOCKED_ON_DEPENDENCY`, `PARTIAL_PENDING_OPERATOR_DECISION`, `NEEDS_REIMPLEMENTATION_Vn`, `READY_FOR_VALIDATION(_Vn)`.
- Sub-agents write MT-json through the junction but **never git-commit `.GOV`**. YOU snapshot: `git -C wt-gov-kernel add .GOV/task_packets/…; git -c core.safecrlf=false commit -m "gov(MT-NNN): record …"; git push origin gov_kernel` after every remediation record and every verdict. Verify the file parses before committing.
- Product commits by the builder only, on the feature branch, subjects `fix(MT-NNN): …` / `style(MT-NNN): …` — never `feat:` (triggers a governance auto-relay hook). `git push origin feat/WP-KERNEL-012` after every candidate (Operator waiver).

## 7. Open findings inventory (root causes to drive the next rounds)

**MT-141 `validation_v2` (fd9060de, 413 binaries: 321 green / 80 red, every red reproduced solo):**
- **V2-F01 BLOCKING HARNESS** — `src/storage/tests.rs:177` `TestStoreCleanupGuard` Drop: embedded shutdown "still draining after 30000 ms" → panic; when it fires during unwinding of another panic → `0xC0000409` abort that kills the binary and destroys the evidence of the underlying failure. 27 set-A binaries fail ONLY for this, 5 mixed, set B ×3, 43/63 lib panics, 14 set-A aborts + lib abort. remediation_v1 knowingly left the src harness unfixed ("src harness unchanged, not in waiver"). Known-good pattern from MT-150: explicit `close_and_remove().await`, not Drop; on Windows the LOCK is held longer than the retry window (F06). **This is R-1; fix it first, then re-measure — most of the 80 collapse.**
- **V2-F02 BLOCKING PRODUCT** — SurrealDB row-shape/schema drift outside the R1–R16 sites: visual_diff `baseline_id` typed `record<kernel_visual_diff_baseline>` but uuid written (055/056); `applied_update_id` missing (418); `PassageIdRow.created_at` NONE (**introduced by candidate commit a0d0a726**, 229); `pin_order -1` (421); quick_switcher coercions (427); `kernel_event_ledger` test schema `wsids`/`authority_action` (lib; same root as F500); loom_blocks `source_rich_document_id` (419).
- **V2-F03 BLOCKING PRODUCT** — unretried `Transaction conflict: Resource busy` under concurrent intake (053 swarm_ingestion, 284, 362).
- **V2-F04 HIGH** — `HbrPillar` lacks `PRIV` vs live `HANDSHAKE_BUILD_RULES.json` v1.11.0 (103/105); the test reads `.GOV` at runtime through the junction (CX-211 boundary — product tests must not read `/.GOV/`).
- **V2-F05/F07 HIGH** — ~30 expectation-drift items in MT-141's own `tests/`: `state_authority "surreal"` vs product `"surrealdb"` (051), docker argv fixtures (126), sandbox trait shape, `app/src-tauri/Cargo.toml` `runtime-full` assertion (123), UserManual freshness `dangling_anchor` (384/386 — 3 http_route anchors not in the WP-009 registry), `mt224` manual patch, `mt026`… ; two superseding proofs the disposition table cites are red (`mt002` core-data migrations, `mt045_lc06`).
- **V2-F06 ENV** — undocumented `HANDSHAKE_TEST_STAGE_BINDING_ROOT`; node `yaml` module missing (349); CDP breakpoint (087). **V2-F09** — 223 silent `eprintln!("SKIP …")` sites (PostgreSQL-era); only 078 ×10, 384 ×1, lib ×1 actually skip. `crash_recovery_e2e` solo 13/13 → contention only.
- MT-150 carried debt still present: `mt138_canonical_atelier_catalog_fingerprint` 9 vs 10 `fn::mt109_*`; 3 schemaless in-source `loom_store` tests red.

**N1B native pass 3 (ba03f563) findings behind the 27 BLOCKED:**
- **F403** — see §3.2. Unblocks most of the 27.
- **F500** — `GET /knowledge/code/symbols` → 500 `quiet_receipt_failed`: `record_quiet_background_work` writes `kernel_event_ledger` rows without the SCHEMAFULL `wsids` array (`schema.surql:5758`). Backend product defect → MT-141 v2 scope.
- **F401** — atelier intake POST returns 401 (route contract vs test auth; MT-141 R15 turned 400→401 on `stage.rs:198-202`).
- **F111** — see §3.3.
- **FDIRTY** — see §3.4.
- **FTAGS / FCANVAS** — `test_tags_panel` (`tags_tag_hub_live_surrealdb_self_seeds…`) and `test_canvas_board_argus` (`mt026_mounted_canvas_canonical_argus_semantic_and_visual_edges`, `test_canvas_board_argus.rs:204`) reproducible alone; backend cause NOT_INSPECTED.
- MT-134 CONCERN: `test_swarm_concurrency::test_lease_serializes_same_widget…` flakes under load; MT-072 spec-namespace CONCERN; MT-126 overlay CONCERN.
- Withdrawn: `frame_timing`, `project_tabs`, `model_session_launch` "reproducible reds" were host-load flakes (green on idle host); `test_word_wrap` red was a runner GPU-heuristic miss.

**MT-135 PASS_V2 non-blocking:** `hbr_obligations` arrays empty (WP-level HBR-INT-009 accounting for you as IV); PASS_V2 is not a green backend set-A.

## 8. Lean mode (Operator-approved 2026-09-16, binding for all validators)

- Full suite is NOT the per-MT gate. MT-141's `wpv-v2/logs/results.jsonl` (fd9060de) is the WP **shared proof batch**; every other MT cites rows from it (`shared_proof_batch.owning_mt = "MT-141"`) and runs only its own `proof_targets` + the tests that anchor its remediation groups.
- Static first (grep the defect class across `src/**`), then the SMALLEST confirming binary — never the biggest binary that also covers the site.
- Reds: one solo rerun (`rerun_alone: true`, pool 1); one representative solo rerun per failure class is enough (do not solo-rerun 40 drain-stalls).
- No re-execution of already-green rows to satisfy "executed by this validator"; cite on `exe_sha256` + `log_sha256` + verbatim result line.
- Keep the `--nocapture` SKIP audit for binaries with silent SKIP branches.
- For MT-141 v2: the validator re-runs only binaries affected by the fix (harness-affected set + F02/F03/F05 sites + set B/C/lib as they are minutes), not 413 blind. Full-suite proof of AC-141-4 was already done at fd9060de; the v2 verdict cites it plus the delta.
- Contention makes evidence: solo reruns must be on an idle host (0 cargo, no other exec pool); reds observed while a builder compiles are suspect until rerun idle.

## 9. Sub-agent rules

- Fresh pair per MT/batch (`general-purpose`, background, model opus). Session ids: `KERNEL_BUILDER-<UTC yyyymmdd-hhmmss>`, `WP_VALIDATOR-<tag>-<UTC>`. Release the previous pair before spawning the next.
- Direct agent↔agent messaging does not resolve in this harness — **you are the relay**: forward validator Phase-A findings to the builder immediately; on candidate SHA verify (HEAD == SHA, tree clean, `ls-remote` == SHA, 0 cargo procs, contract appended & parses) → snapshot → `SendMessage` the validator; on FAIL forward `remediation_required` to a fresh builder; on PASS release both, snapshot, move on.
- Agents may end their turn while a long background command runs; they are resumed on completion — **but a driver script that isn't a tracked child does not resume them**. Your tick must detect a lane whose driver finished/halted with no agent activity and `SendMessage` it to continue (happened twice).
- Verify agent claims against the ledger: one validator reported "0 reds so far" while `results.jsonl` had 14 red rows — challenge with the exact row list; it retracted. Count `exit_code!=0 || failed>0` yourself.
- Require every green to carry its own log + exit + verbatim `test result:` line; `NOT_INSPECTED`/`UNVERIFIED` labels; never weaken/`#[ignore]` assertions; builder logs are triage input for validators, never proof.
- The pre-crash coordinator pattern worked: builder on its own target, two validators exec-only on the warm target, three lanes concurrent. Keep it.
- Give agents absolute paths; tell them the other lanes' targets so they never touch them; tell them to check `git status --porcelain` after every run and report any tracked-file dirt (`TRACKED_FIXTURE_WRITE`) instead of restoring it.
- wp validator sub agents must read codex.md and wp validator porotocol file and follow it's rules and instructions
- kernel buillder sub agents must read codex.md and kernel builder porotocol file and follow it's rules and instructions

## 10. Monitoring

`CronCreate` every 8 min (avoid :00/:30). Each tick, ONE PowerShell command: feature HEAD + dirty paths; gov dirty; MT lifecycle + `*_vN` keys for the active MTs; cargo/rustc count + command lines; per lane: newest file + time, newest `.log` tail matching `^test result|panicked at|FAILED|^running`, `00-lane.txt -Tail 3`; repo-local `target/` scan and `D:/hs*`. Stall = no artifact/proc change > 25 min → `SendMessage` a status request (agents park; the tick is what un-parks them). Report 2–3 lines per tick. Cancel/re-create the cron whenever the lane set changes. Delete the cron when all lanes close.

## 11. Mistakes this session (don't repeat)

1. Ran `just validator-startup` per protocol before reading the Operator stance → it launched a `cargo run` product build into `handshake-cargo-target`; had to kill my own process tree. Do not run `just` recipes.
2. Told agents to "use their own tick" — Operator said no ("meant this for you"); the tick is the orchestrator's. Retracted.
3. Two validators wrote `lifecycle.status = "COMPLETED"`; convention is `status == validator_verdict`. I aligned 15 files afterwards; put the convention in every validator brief.
4. Reported "0 reds" from an agent's word once; verify ledgers before relaying counts.
5. Let two cargo builds run concurrently on two targets → 64-min relink. Sequence cargo when the host is the bottleneck.
6. The MT-141 validator's driver had a STOP-loop bug (orphaned two long binaries, derived their exit from the result line) and a stale POOL file (three solo reruns ran concurrently, re-queued). Ask validators to print pool size and PIDs at phase start.

## 12. Do / Don't

DO: verify remotes with `ls-remote`; snapshot every MT-json change on `gov_kernel` and push; keep `status == validator_verdict`; challenge agent tallies against `results.jsonl`; sequence cargo; put lean-mode text verbatim in every validator brief; name the dependency in every BLOCKED verdict; keep TRACKED_FIXTURE_WRITE as a finding, not a restore.
DON'T: run `just`; touch `handshake_main`/`wt-gov-kernel` product trees; create targets; run the full suite per MT; re-exec green rows; let a validator cite builder logs; let an agent commit `.GOV`; use `feat:` subjects; `git checkout/restore` without same-turn Operator authorization; trust `origin/*` tracking refs.

## 13. Artifacts to read first

- MT-141: `../Handshake_Artifacts/WP-KERNEL-012/MT-141/wpv-v2/logs/{00-lane.txt,results.jsonl,exe-map.tsv,016-session-takeover.json}` and `MT-141.json#validation_v2` (reds, remediation_required.groups, residue).
- N1B native: `../Handshake_Artifacts/WP-KERNEL-012/MT-108/wpv-n1/logs/{311_pass3_summary.json,019_pass2_summary.json,016_session_takeover.json,015_stale_backend_gate_rows.json,013_env_invalid_rows.json,native_mt_coverage_matrix.json,results.jsonl}`.
- MT-135: `../Handshake_Artifacts/WP-KERNEL-012/MT-135/{kb-b3,wpv-v2}/logs/` and `MT-135.json#validation_v2`.
- Previous handoff (2026-09-15) lessons still apply: `build_loom_mutation_event` idempotency design is accepted (don't "fix"); schema DDL changes require re-pinning `EXPECTED_CATALOG_SHA256` in `schema.rs` + lineage upgrade tests; embedded-DB teardown via `close_and_remove().await`; on a 429 snapshot state and stop.

## 14. First actions

1. `git -C wtc-native-editors-v1 status/log`, `git -C wt-gov-kernel status/log`, `ls-remote` both, `tasklist | findstr cargo` — confirm §2.
2. Read `MT-141.json#validation_v2` and `311_pass3_summary.json` (node extraction).
3. Batch the 9 non-native READY MTs (MT-112 118 120 139 142 145 146 148 149): read their contracts, group by shared proof surfaces, spawn one WP_VALIDATOR batch lane (lean: own proof targets + shared-batch citation; warm target exec-only; a `--no-run` rebuild is allowed for targets whose exes are absent) — no builder unless a verdict FAILs.
4. In parallel, spawn the MT-141 v2 KERNEL_BUILDER on the kb-b2 target with R-1 = V2-F01 harness fix, then F02/F03 (+F500 `wsids`), F05/F07 test drift, F04 `PRIV`, F06 env docs; scope waiver per §3.1; per-file rustfmt; `fix(MT-141):` commits; push; `remediation_v2` append. Then a fresh validator (`wpv-v3`) with lean rules (§8).
5. Arm the 8-min tick. Relay.
