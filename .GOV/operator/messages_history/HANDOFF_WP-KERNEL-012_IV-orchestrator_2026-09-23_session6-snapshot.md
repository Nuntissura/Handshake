# SNAPSHOT + POSTMORTEM — WP-KERNEL-012 IV orchestration (2026-09-23, session 6, taken 23:25 local)

Purpose: a recovery snapshot in case this session is cut off mid-run (the Operator's weekly usage limit is close). It is not a planned handoff. Read it together with `HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-23_session6.md` (full rules, §4b correction). This file adds the live run state, the postmortem, and the workflow/authority changes since session 5.

Role: INTEGRATION VALIDATOR orchestrating sub-agents (KERNEL_BUILDER, WP_VALIDATOR, read-only audit/extraction agents). No product code by the IV. WP-KERNEL-012 is pinned to governance `896f4e15` (`packet.json.governance_pin`); the Operator's repo-governance refactor (commits `b4c3727d`, `895f4eda`, `520c0c8a`, `e89fb886`, `61b79f01`, …) applies from WP-KERNEL-012-bis on. Read the pinned authority with `git -C wt-gov-kernel show 896f4e15:<path>`.

## 1. Live state at snapshot

- Product: `feat/WP-KERNEL-012` = **`8f60313b`** (pushed, `ls-remote` verified, tree clean). Backup `backup/WP-KERNEL-012-wtc-dirt-20260905` = `bea9496d` (stale; fast-forward it at the next checkpoint).
- **PASS 118 of 159.** This session: MT-131 PASS_V3, MT-088 PASS_V8, MT-108 PASS_V7. That is all.
- Open 41: FAIL 27 (008 023 027 033 034 036 046 064 065 066 067 068 074 079 098 113 116 117 120 121 122 127 128 130 140 141 143), BLOCKED 3 (026 070 111), PARTIAL 4 (045 124 125 142: special runs), READY_FOR_VALIDATION 7 (153 154 155 156 157 158 159). Verified at 23:12: every open MT has a remediation newer than its last validation, so no open MT is waiting on code; all wait on validation.
- MT-153..158 each carry `validation_v1_partial` (CHECK-TESTS, CLIPPY-CHANGED 0 on changed lines, FMT, DIFF-CHECK at `0caf1c10`, diff base `0cfbff64`).
- Governance: `gov_kernel` at `c463388f` or later, pushed. Other agents commit there: commit ONLY explicit paths.

### Lanes running at snapshot (session agents die with the session; resume from the ledgers)
- **C: validator lane** `Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x/` (ledger `00-lane.txt`, logs 35–38+). Export `C:/.target/WP-KERNEL-012/MT-109/wpv-c3x/export-e9973e8c/` holds **8f60313b** (legacy name). Target `C:/.target/WP-KERNEL-012/MT-109/wpv-c3x/target`. It was finishing MT-156 checks (BUNDLE-AUTH, WORKFLOW-EXISTING), then under orders to do the **full union run** (§3).
- **D: validator lane** `Handshake_Artifacts/WP-KERNEL-012/MT-158/wpv-d1/` (ledger, logs 00–02). Export `…/wpv-d1/export` (8f60313b, 7143 files). Target `Handshake_Artifacts/WP-KERNEL-012/MT-109/kb-c3/target` (sole owner). A batched `cargo test --no-run -j 2` for MT-158/159/141/033/026/070/111 targets was linking.
- **Remediation builder (standing)** lane `Handshake_Artifacts/WP-KERNEL-012/MT-154/kb-c5/` (idle; mt138 is not a real failure at 8f60313b). Its target, when needed: `C:/.target/WP-KERNEL-012/MT-154/kb-c5/target`.
- **Retired:** builder C4 (`MT-154/kb-c4` ledger, ~620k tokens), validators C3X/C4V/C5V.
- Union target list for all open MTs: `Handshake_Artifacts/WP-KERNEL-012/MT-109/proof-matrix/union.json` → 43 core test targets + `--lib`, 35 native test targets. (`open_mt_proof_matrix.json` from the Haiku agent is INCOMPLETE; don't use it.)

## 2. How to resume if stranded

1. Verify: `git ls-remote origin refs/heads/feat/WP-KERNEL-012` (expect 8f60313b or a later builder push), clean tree; process scan (`cargo|rustc|link|*-*.exe` with `wpv-c3x|wpv-d1|kb-c5` in the command line). Orphans from a dead session are safe to stop only after checking the command line (they were started by this session's agents).
2. Read both validator ledgers and every MT JSON `validation_v*` newer than 22:00: commit any verdict not yet on `gov_kernel` (after verifying it).
3. Spawn ONE fresh Sonnet validator for the **full union run** (§3), and keep ONE Opus remediation builder on standby for failures.
4. Tick every 10 min (script `tick.ps1` pattern: PASS count, verdicts, SHAs, real CPU gain per process).

## 3. The workflow now (changed this session; supersedes session 5 §6/§11)

- **One candidate per round** (currently 8f60313b). The builder doesn't push while a validation round is building (it breaks warm targets).
- **One union build, then one run:** build every test binary all open MTs need in ONE `cargo test --no-run` per crate (union.json), then ONE `cargo-nextest` run (`../gov_runtime/tools/cargo-nextest/0.9.146/cargo-nextest.exe nextest run … --config-file <lane>/nextest.toml`, with `slow-timeout = { period = "60s", terminate-after = 5 }`, JUnit on), then write all verdicts from the JUnit in one pass. No per-MT builds, no reruns, no diagnostics, no hang checks (Operator 22:30).
- **Every FAIL goes to the remediation builder immediately** (test, assertion, file:line, MT); fixes batch into one push per round.
- **Builders never run expensive tests** (Operator 16:10): `cargo check`/clippy only, plus at most one focused test.
- **Disk:** the C: grant was raised to **150 GB** (Operator 23:20): stop cargo below **192 GB free**, measured by free space. D: HDD links are 40–60 min, so prefer C: for builds. Test runtime roots on D:.
- **Test env:** `HANDSHAKE_TEST_SURREAL_SYNC=never` (store path `?sync=never`; the only switch the embedded engine honours) for non-durability tests; durability tests leave it unset. `SURREAL_DATASTORE_SYNC` does NOTHING.
- **Monitoring:** check real CPU per process, not just log timestamps (a compile stalled at 0 CPU unnoticed for ~17 min).

## 4. Authority changes this session vs session 5 (and why)

| Change | Where | Why |
|---|---|---|
| CX-EXEC-003B/006–011 output-first rules + ORC-OUT, IV-OUT, CODER-OUT, WPV-OUT, KB-OUT, AM-OUT | gov `98523902` | hours of activity reported as progress; broad batches; held commits |
| CX-EXEC-012 remediation scope + global `[GLOBAL-REMEDIATE-001..005]` (outside git) | gov `cbe5dbc9` | research/red-team machinery was firing on routine remediation |
| MT-154 spec resolutions (D-154-1..3, silent-deny → 403) | gov `b9d011cd` | decisions resolved from the spec instead of escalating |
| MT-158, MT-159 added inside 012 (Operator A, "include in this WP") | gov `b61a37c6`, `9e0e901e` | authority gaps (Locus job path as root; /jobs routes; id existence leak) |
| MT-154 out-of-file-list waiver (42 files) | gov `9af54d43` | Operator waiver |
| Governance pin at 896f4e15 | gov `c9bc29e8` | the governance refactor runs live in the same worktree |
| Session-6 handoff + §4b correction + 150 GB grant | gov `bb982a5d`, `2296ac1a`, `c463388f` | recovery; a wrong env rule corrected |
| Tools in `../gov_runtime/tools/` + `TOOLS.json` (nextest, sccache, ast-grep, ProcDump, Handle, minidump-stackwalk, dump_syms; rust-lld in the toolchain) | outside git | hang diagnosis; faster runs |
| Template scenario suite for the template agent | gov `9eabcb3e` | Operator request |

Product (feat, 0cfbff64..8f60313b, 18 commits): C3 residuals + MT-153..157 (`625893e1`..`bea9496d`), re-pins (`9a2a8d7e`, `e9973e8c`, `52873427`), MT-088 test fix (`88822974`), test lock order + clippy (`c21a5be0`, `51d9f40f`), CRDT delete (`5f31aa07`, `7e73eb03`), MT-158 (`72498260`), bootstrap index split (`0caf1c10`), MT-159 + watchdog (`9552ca2b`, `a4f5a46f`), test sync switch (`8f60313b`).

## 5. Postmortem (what went wrong, cost, cause) — all times local

Result: 3 MTs reached PASS in ~12 h. Most time and tokens went to orchestration mistakes, not product work.

1. **Broad validation first (10:05–13:45).** 33 MTs given to one validator with a broad setup; a 0-test filter; a 40-min hang on an out-of-scope test. Cause: the IV followed the session-5 §11 step list instead of the goal.
2. **Held commits (12:00–13:43).** The builder held 109 changed files uncommitted to "prove first"; the IV accepted it. Nothing could be validated.
3. **Wrong environment fix (≈16:00–21:58, the largest cost).** The IV told the lanes to set `SURREAL_DATASTORE_SYNC=never` without verifying that the embedded engine reads it. It doesn't (only the store path `?sync=never` does). Every test from then on still ran fsync-per-commit, and the IV diagnosed "hangs" caused by its own unverified fix, including a C:-runtime detour. The real fix (`8f60313b`) came at 21:58, and its first test passed in 3.86 s where it had hung every time.
4. **Hang misdiagnoses.** Hang A (OS `NtFlushBuffersFile` never returning; kernel/filter-side, reproduced on the HDD and the SSD) was real; "hang B" (index builder) was measured under the wrong mode and remains UNVERIFIED. Several claims were relayed before verification (MT-156 "passed" from a truncated log; findings credited to tests that came from code reading). The Operator lost trust.
5. **Per-MT builds (until 22:40)** instead of one batched build per lane; mid-round pushes invalidated warm targets; new export folders forced rebuilds; the D: lane extracted its export twice.
6. **Silent stalls.** A validator compile sat at 0 CPU for ~17 min before the IV noticed (it checked logs, not CPU). A validator's own `timeout 600` killed builds mid-link and truncated test runs.
7. **Usage-limit stop (≈19:25–20:20)** stopped both agents mid-work.

Genuine findings this session (and their source): the hang root cause (dumps and stackwalk); the SURREAL_DATASTORE_SYNC no-op (audit of the SDK source); MT-158/159 authority gaps (builder code reading); the bootstrap watchdog; the per-index DDL. Product fixes are all pushed (8f60313b).

## 6. Do / Don't (cumulative with session-6 §9)

DO: verify one fact before telling agents to act on it (read the code or SDK path); measure real CPU per process each tick; one frozen candidate per round; one union build + one nextest run; send every FAIL straight to the builder; commit verdicts only after checking status==verdict, completer≠claimer, a passing proof record per required check, and binary provenance; keep reports to PASS count / verdicts / SHAs.
DON'T: relay an agent claim unverified; build per MT; wrap build+test in one short timeout; change the test environment on theory; run hang checks, reruns or diagnostics once a fix is proven; let builders run tests; push mid-round; stage or commit files the governance-refactor agent owns.

## 7. Next actions (unchanged goal: every MT PASS → WP verdict → merge)

1. The full union run at 8f60313b (C: lane) + the D: batch → verdicts for ~37 MTs; FAILs → the builder → one push → one union re-run of the failed MTs only.
2. Special runs: MT-124/125 RED halves (patches in `…/MT-124/kb-c1/red-half.patch`, `…/MT-125/kb-c1/red-half.patch`; standing Operator authorization), MT-045 release perf, MT-142 idle-host stress; the deferred durability tests (rev-158/159, bootstrap_resumes) once hang A is understood (Operator: `fltmc filters`, `(Get-MpPreference).ExclusionPath` as admin).
3. WP boundary: full suite on the final SHA + HBR/Argus/UserManual/diagnostics closure → IV verdict → cleanup `C:\.target\WP-KERNEL-012` → merge to main (backup push first, `sync-gov-to-main`, push `origin/main`).
