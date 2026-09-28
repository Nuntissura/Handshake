---
file_id: WORKFLOW-WP-KERNEL-012-KB-ORCHESTRATOR-2026-09-24
file_kind: operator_workflow_reference
updated_at: 2026-09-24T17:02:15Z
wp_id: WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1
governance_pin: 896f4e15
authority: reference_only
---

<topic id="use-and-authority" wp="WP-KERNEL-012" updated_at="2026-09-24">

# WP-KERNEL-012 execution template and failure record

This is an Operator-requested, restartable *reference*, not a new workflow, status surface, proof waiver, or authority. Execute the current MT JSON and the WP's governance pin, with explicit later Operator decisions. If a sentence here conflicts with those, stop using that sentence and use the authoritative source. Never turn this document into a second MT status ledger. [packet.json `governance_pin`; Codex CX-021/CX-914/CX-GOV-PIN-001; KB-START-001]

The closure target is every required MT at independent `PASS_Vn`, the packet-declared end-of-WP extra proofs and whole-WP validation, then hand integration to its authorized role. The Kernel Builder orchestrator may coordinate builders and validator, fix within its product role through assigned builders, and commit/push governance on `gov_kernel`; it may **not** issue verdicts, mark MTs complete, merge, or assume integration authority. Product progress means pushed product commits and MT status/verdict changes. [KB-AUTH-002; KB-HANDOFF-002/003; Codex CX-EXEC-006/008]

Source order for *this* WP: (1) explicit current Operator instructions and later recorded decisions, (2) `packet.json` governance pin `896f4e15` for Codex, role protocols, build rules and HBR, (3) LIVE kernel `.GOV` for current task state and non-conflicting operating detail, (4) Master Spec resolved through `.GOV/spec/SPEC_CURRENT.md` for product questions, (5) handoffs/logs as context only. Read root `AGENTS.md` and `CLAUDE.md` in the assigned product worktree; do not use `handshake_main/.GOV` as live authority or import later template-refactor gates into this pinned WP. The later `canary_check=REPLACE_ME` is **not** a WP-012 gate; the Operator still requires IV-OUT-005/CX-984-014 prelaunch checks. [packet.json `governance_pin`; Codex CX-010/011/021/CX-212C/CX-SPEC-TRUTH-001; session7 handoff `operator-rules-and-corrections`]

Canonical paths, relative to `wt-gov-kernel` unless stated otherwise: `.GOV/codex/Handshake_Codex_v1.4.md`; `.GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md`; `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`; `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`; `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`; `.GOV/task_packets/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/packet.json` and exact `MT-###.json`; `.GOV/spec/SPEC_CURRENT.md`; `../wtc-native-editors-v1/AGENTS.md` and `CLAUDE.md`; the latest session7 handoff and session6-snapshot §6–7. Resolve the artifact root from the canonical kernel checkout; the current local lane is `../Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x/`. [KB-AUTH-001/KB-START-001; Codex CX-984-001/012/013]

The Handshake Creation Template skill was consulted only as a shape/check reference. It does not replace this WP's pin, statuses, actor permissions, test entrypoint, or Operator instructions. [packet.json `governance_pin`; Codex CX-EXEC-011]

</topic>

<topic id="restart-inventory" wp="WP-KERNEL-012" updated_at="2026-09-24">

## A. Start or resume without guessing

1. `[W12-A01]` Identify the exact kernel checkout and product worktree; resolve the `.GOV` junction to the kernel. Confirm `gov_kernel` and `feat/WP-KERNEL-012`, both clean/dirty states, exact HEADs, remote feature tip, and Git worktree/branch inventory. Preserve user or other-agent dirt; do not create a branch/worktree, reset, checkout away, or hide work. `git -C <kernel|product> status --short --branch`, `git -C <product> rev-parse HEAD`, `git -C <product> ls-remote origin refs/heads/feat/WP-KERNEL-012`, `git worktree list`. Never infer push from local HEAD. [Codex CX-212C/F, CX-GIT-001/002, CX-107/108; KB-START-002]
2. `[W12-A02]` Read the authority files named above once for the current revision/scope, then the packet and the exact MT JSON(s) being touched. For each MT read `lifecycle`, allowed paths, claim/completer, failure/remediation, `validation.proof_records`, own `proof` checks, top-level `proof_commands.commands`, `unmapped_acs`, feature set, and `extra_build_proofs`. Never use a derived queue/index or a handoff as the acceptance surface. Reopen only changed sections or an unresolved question. [Codex CX-620/CX-EXEC-002/008; WPV-OUT-006; KB-IMPL-001]
3. `[W12-A03]` Enumerate only exact `MT-###.json` files (exclude `MT-136-candidate-boundary.json`), parse each, tally `PASS_Vn`, `READY_FOR_VALIDATION`, `FAIL_Vn`, `BLOCKED`, `NEEDS_NEW_APPROACH`, and list each `blocked_on`. The MT JSON alone owns status. A `BLOCKED` state for a named open fix is not a PASS/FAIL verdict; enforce `status == validator_verdict` when an acceptance verdict is issued. Never invent a status. [Codex CX-STATUS-001; KB-HANDOFF-003; packet MT JSON]
4. `[W12-A04]` Read the latest pushed product and governance commits, validator JUnit/log/proof state, running process identities (PID **plus** start time **plus** command line), and owned lane handles before sending work. A current run's immutable export remains its SHA even if the feature branch advances. Do not attribute an old binary to a new commit. [Codex CX-VAL-001/004; KB-STEER-004]
5. `[W12-A05]` Resolve a product question from `SPEC_CURRENT.md` → active manifest/index → topical Master Spec module and relevant MT/code, preferably with a cheap read-only agent; do not escalate a question already settled in an MT `operator_decision` or current spec. If a genuine scope/contract decision remains, stop only that item, record it in its MT, and continue unrelated work. [Codex CX-011; KB-IMPL-006; session7 handoff `open-failures`]
6. `[W12-A06]` Do not claim the old session's count as live. As of this file's initial read: 119 PASS, 31 READY, 9 BLOCKED; `MT-032` moved from old `PASS_V4` to `BLOCKED` on a named delete regression. The frozen `2bf51103` core finished 222 tests with nine distinct FAILs; native was still running. Recompute, never copy this snapshot into MT state. [MT-032.json; run `round-2bf51103` logs; Codex CX-EXEC-006]

</topic>

<topic id="agents-and-ownership" wp="WP-KERNEL-012" updated_at="2026-09-24">

## B. Assign and steer the existing lanes

1. `[W12-B01]` Use one persistent Astra/medium Kernel Builder context per remediation batch, resumed for follow-up; use a second Astra/medium builder only for disjoint files. Give each exact MT/failure, owned files, recorded remediation, pushed-commit output and 20–30-minute deadline. Do not dispatch one new agent per MT, overlap a shared file while a check is running, or let a builder mutate `.GOV`, start a test, create a worktree/branch, or stop a foreign process. The orchestrator reviews delegated diffs and remains responsible. [Operator instruction 2026-09-24; KB-LANES-001/002, KB-OUT-004/008; Codex CX-SAFE-001]
2. `[W12-B02]` Keep one independent Sol/medium WP_VALIDATOR context. Its scope is clean git-archive export, one union round per candidate, structured test mapping, independent proof, failure classification, and MT verdicts. It is the **only** Cargo test runner. The Kernel Builder orchestrator must not suggest verdict classification as fact, write verdicts, mark complete, or merge. [Operator instruction 2026-09-24; Codex CX-VAL-001/006; KB-AUTH-002; WPV-OUT-001/007]
3. `[W12-B03]` Use cheap read-only agents for bounded spec/code/proof audits. They cannot edit, test, issue verdicts or replace the validator's own inspection. Tell the owning builder about every confirmed shared-file change and every new product FAIL. Do not turn audits into a new broad-research loop during remediation. [KB-LANES-001/KB-CARGO-SHARED-001; WPV read-only lens duty; Codex CX-EXEC-012]
4. `[W12-B04]` Maintain the Operator's 10-minute monitoring tick. At each tick observe agent output, process PID/start/command, CPU/I/O or log/test count, exit and target size; compare with the previous observation. Keep long commands asynchronous and bounded waits below the tick; do not end a turn while a round runs. Log silence or low CPU alone is not a hang. Stay quiet on unchanged observations; relay actionable new FAILs promptly. Do not repeatedly interrupt agents between ticks. [Operator instruction 2026-09-24; KB-STEER-002–007; Codex CX-EXEC-009/010]
5. `[W12-B05]` If a lane has no commit/verdict after 20 minutes, ask for the exact output/blocker; at 30 minutes apply KB-OUT-004 using its ledger, without resetting attempt counts or repeating unchanged tests. A named long running proof is monitored, not falsely called productive. Never stop a process this assistant session did not start without exact `PROCESS_STOP_APPROVED:<comma-separated-PIDs>`. [KB-OUT-004; Codex CX-EXEC-003/010/CX-SAFE-001]

</topic>

<topic id="builder-cycle" wp="WP-KERNEL-012" updated_at="2026-09-24">

## C. From a real failure to a pushed candidate

1. `[W12-C01]` Read the exact failed assertion, stderr/JUnit, candidate SHA, MT record and code path. Classify product versus infrastructure only from observed evidence; the validator owns final classification. A failure outside an MT's named test is mapped by the validator, not guessed from a filename. Count repeated attempts per assertion across sessions. No new diagnostic run for unchanged inputs; after two failed approaches record a distinct cause and change approach or escalate only that blocker. [Codex CX-VAL-006/CX-EXEC-003/003A/004/012]
2. `[W12-C02]` Group all confirmed related fixes before the expensive next union, including a fix needed by an MT already READY. Keep ownership disjoint and review changes against the MT's allowed paths; do not rewrite acceptance criteria to fit code or add unrelated hardening. Shared schema/pin edits need a quiet boundary and one coordinated builder. [Operator 2026-09-24 question on same remediation run; Codex CX-EXEC-004; KB-CARGO-SHARED-001]
3. `[W12-C03]` Builder runs only `cargo check --locked --tests` and applicable clippy/static checks using the warm D: `Handshake_Artifacts/WP-KERNEL-012/MT-154/kb-c5/target`. Set the assigned `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, reduced debug-info profile, `TMP/TEMP` under the D: owner lane, and the exact required features/manifests. One Cargo per physical disk, never builder Cargo on C:, never builder `cargo test`, `--no-run`, or a single-MT test. Check process inventory before launch and poll it. Distinguish a green check from runtime proof; inherited clippy debt must be compared against changed diagnostics, not called green. [Operator hard rules 2026-09-24; KB-OUT-002/008; Codex CX-984-002/014]
4. `[W12-C04]` Verify the builder diff and source hashes after checks. If any input changed during a check, invalidate that check and run one combined check on the stable tree; do not accept a stale green exit. If the changed code compiles, commit **immediately** with `git commit -m '<scope>' -- <explicit-product-paths>` and push explicitly to the existing `feat/WP-KERNEL-012` ref. Verify `ls-remote`, local cleanliness, and no `.GOV` in the feature commit. Do not wait for the whole batch or validator proof to push. [Codex CX-EXEC-007/CX-GIT-002/CX-212F; KB-OUT-001/KB-IMPL-004]
5. `[W12-C05]` A pushed compile is candidate progress only. The implementer may record readiness on the MT under the allowed transition and required implementer proof, but never PASS/FAIL. Later source commits queue for the next round and do not restart or silently change a running export. [Codex CX-STATUS-001/CX-VAL-001; KB-PROOF-001/002; KB-HANDOFF-003]

</topic>

<topic id="validator-preflight-and-round" wp="WP-KERNEL-012" updated_at="2026-09-24">

## D. One union validation round per stable pushed SHA

1. `[W12-D01]` Validator receives the full 40-character pushed feature SHA and the exact READY MT set from canonical JSON. Confirm feature HEAD/remote equal that SHA, tree clean, and no other Cargo on C:. Record the before-run branch/worktree inventory. Do not launch while a known applicable product fix remains unpushed or while a prior round still runs. [Codex CX-VAL-001/005/CX-GIT-001/CX-984-002; runner `run-round.sh` preconditions]
2. `[W12-D02]` Resolve/verify the sole `Handshake_Artifacts` root from the kernel; C: grant covers **only** `C:/.target/WP-KERNEL-012/MT-109/wpv-c3x/target-r52`, and the builder's D: target is separate. Compute target bytes, remaining cap `150000000000 - bytes`, C: free bytes versus floor, and expected incremental growth from the changed compile graph **before** launch. Record both the estimate and measured values. If capacity is uncertain/insufficient, solve it without deleting the warm target or moving it to D:. Runtime stores, TMP/TEMP, logs/evidence stay under the D: owner. [Operator C cap/no-delete instructions; Codex CX-984-001/002/012/014; runner `check_target_cap`]
3. `[W12-D03]` Read `run-round.sh` end to end, plus `nextest-core.toml`, `nextest.toml`, effective environment, feature/target arrays and every invocation consuming any changed config. Check the script has no whole-run timeout, handles nextest `100` as failed tests without aborting the native phase, rejects 0 tests, copies both fresh JUnit files, and leaves bounded per-test slow-timeout. Verify native-only `owned-backend` config is **not** read by the core runner; include the MT-008 code-nav/hover binaries in its group. Confirm the specific MT-153 route-matrix timeout is long enough for its observed ~298-second case under the contract bound. A config change is not ready until every reader invocation is checked. [IV-OUT-005; Codex CX-EXEC-005/CX-VAL-005; WPV-OUT-003/005; session6 postmortem 21]
4. `[W12-D04]` Trace every env var the selected tests/fixtures read against the script's actual exports, including `HANDSHAKE_ARTIFACTS_ROOT`, `HANDSHAKE_TEST_ARTIFACTS_ROOT`, `HANDSHAKE_WORKSPACE_ROOT`, `HSK_TEST_BACKEND_TARGET_ROOT`, `HSK_TEST_BACKEND_BIN`, stage-binding root, Surreal test sync and GPU setting. Confirm the backend binary is built from the **same export** and is in its D: owner child. Check the shortened evidence root plus deepest fixture leaf stays below the Windows 260-character path limit; a root-only length check is insufficient. Confirm all output paths after junction/symlink resolution stay in assigned owner. [IV-OUT-005; WPV-OUT-005; Codex CX-984-008; runner env block; session6 postmortem 13/22]
5. `[W12-D05]` Confirm there is no silently inserted WP-012 canary requirement from the later governance refactor. Do the Operator's concrete IV-OUT-005 prelaunch checks, including host contention/OS state when relevant; an infrastructure preflight failure starts **no** round and yields **no** MT verdict. Never call a hardware issue a code waiver. [packet `governance_pin`; session7 handoff canary correction; Codex CX-VAL-006]
6. `[W12-D06]` Launch the checked-in lane script once with the full SHA from a clean `git archive` export, under a non-interactive background session with live stdout/stderr/JUnit paths and process identity. The current script hardcodes the warm C: `TARGET`; it does **not** read `RUN_ROUND_TARGET` despite the Operator's env wording, so verify the effective target rather than assuming that variable controls it. Do not edit a running script/export or add a per-MT Cargo invocation. The two nextest invocations are one per separate Cargo manifest (core and native), not a per-MT build. [Codex CX-VAL-001/004; runner header, `TARGET`, CORE/NATIVE blocks; Operator warm-target instruction]
7. `[W12-D07]` Monitor at each tick. The runner builds core union, native union, then its backend binary sequentially; it validates native test-group selection before tests, runs core and native with `--no-fail-fast`, and writes separate fresh JUnit. Confirm nonzero nextest exit `100` means actual test failure, while `96`, 0 tests, missing/stale JUnit, build abort, invalid path/config or foreign process interference are infrastructure/invalid proof. Do not kill/stop a foreign process; do not infer completion from log silence. [runner build/run blocks; Codex CX-EXEC-005/CX-VAL-005/006/CX-SAFE-001; KB-STEER-004/006]
8. `[W12-D08]` At round end, hash logs/JUnit, verify executed count >0 and selected test names, source export SHA and binary compile provenance (`Compiling ...export-<sha>...` or equivalent dependency evidence). Compare before/after worktree/branch inventory and external OS resources left by tests. Do not label a complete failing suite as script infrastructure merely because nextest returned `100`. Preserve required evidence and warm reusable target. [Codex CX-VAL-001/004/005/CX-GIT-001/003; WPV-OUT-003/005; VPX-003]

</topic>

<topic id="verdict-and-failure-loop" wp="WP-KERNEL-012" updated_at="2026-09-24">

## E. Independent verdict, relay, repeat

1. `[W12-E01]` The validator reads each exact MT JSON's required `proof` and top-level `proof_commands.commands`, including `unmapped_acs`, plus applicable HBR rows. A derived queue merely selects tests; it never defines acceptance. For each required check, map one **passing** result line in the hashed log to a typed `validation.proof_records[]` record naming command, features, SHA, compiled binary, environment, executed count and coverage. A partial green suite, compiled target, previous implementer test, or substitute representation is not PASS. [Codex CX-EXEC-008/CX-VAL-001; WPV-OUT-006; VPX-001–004; MT-033 `unmapped_acs`]
2. `[W12-E02]` Before every PASS/FAIL commit, independently check JSON parses, `lifecycle.status == lifecycle.validator_verdict`, `completed_by != claimed_by`, full required proof (including unmapped AC disposition), log hash, candidate/binary provenance, clean product tree and explicit scope. The validator issues the verdict and commits/pushes the exact MT JSON path on `gov_kernel`; the builder/orchestrator only audits and relays. A `BLOCKED` state names `blocked_on`; infrastructure changes no MT status. [KB-HANDOFF-003; Codex CX-STATUS-001/CX-VAL-006/CX-GIT-002; VPX-001/003; Operator verdict checklist]
3. `[W12-E03]` Emit each defensible MT verdict as soon as its *own* evidence is complete; do not wait for unrelated MT failures or substitute a blanket FAIL for a union exit. If another MT's code caused a shared failing test, classify and map from the offending commit/code/result. Relay the exact failing assertion, observed value, candidate SHA, file:line and named remediation through the steering role to the assigned builder. [Codex CX-EXEC-008/CX-VAL-006; WPV-OUT-001/007]
4. `[W12-E04]` Builder acts on the failure, combines already-known related fixes (including ones for READY MTs), D: check/clippy, explicit-path commit/push, then validator preflights and runs the **next** one union candidate. Do not repeat the same SHA/failed assertion without changed relevant inputs or a distinct recorded hypothesis. Reuse prior independent proof only after exact relevant-input diff and matching binary/config/resource conditions; name both commits in reused evidence. [Codex CX-EXEC-003/003A/004/CX-VAL-001/002; VPX-004/005]
5. `[W12-E05]` Treat a failing pin as a code/proof issue, never hardware. The LET-bound workspace-delete schema changed compiled `schema.surql`; `EXPECTED_SCHEMA_INFO_SHA256` remains the last measured value. The canonical `mt139_current_schema_info_pin_matches_fresh_mem_catalog` test in the next union must print the new engine-backed fingerprint; only then may a builder repin, check/push, and submit the resulting SHA for independent union proof. Do not guess the hash, delete the pin assertion, or run a single-MT test to obtain it. [MT-157.json re-pin procedure; `schema.rs` canonical test and comment; Codex CX-VAL-001]
6. `[W12-E06]` A hardware-only proof waiver requires measured hardware blocker, why the normal proof cannot execute, exact next-best permitted proof method, its actual result, and why that result supports the conclusion; the validator judges it. No waiver covers syntax, code, auth, test assertion, schema pin, or other coding failure. Do not self-certify PASS on a waiver. [Operator instruction 2026-09-24; Codex CX-503B1; VPX-001]
7. `[W12-E07]` MT-033 is **not** made PASS by its current `PC-033-01`: `unmapped_acs` still lists literal `atelier_embed` persistence, `PUT /canvas`/`atelier_item_id`, and AccessKit `StartDrag`/`Region`. The current hsLink/placed_block_id/Click/GenericContainer tests are evidence for different behavior. Its allowed paths exclude the backend; the pinned AccessKit action enum lacks StartDrag; MT-066 requires that same stage-pane ID as GenericContainer. Master Spec §7 supports same-Loom-entity Canvas placement, not those literal names. The validator must classify it; only an authorized contract reconciliation or literal implementation within approved scope can close it. Keep unrelated MTs moving. [MT-033.json AC-001/002/003/006, allowed paths, `unmapped_acs`; MT-066.json AC; Master Spec `07-user-experience-and-development.md` lines 545–547]
8. `[W12-E08]` Keep supervisor-only ignored proofs and declared non-union/release/RED/idle-host proofs at their contract-appointed boundary; do not add a per-MT Cargo build to the active union. Currently blocked examples: MT-068/098/140 named supervisors; MT-045/124/125/142 end-of-WP; MT-127 separately governed proofs. Read each current `blocked_on` before scheduling. A test runner's skipped/ignored result is not PASS. [MT JSON lifecycle/proof; Codex CX-VAL-005; session7 handoff]

</topic>

<topic id="closure-and-reporting" wp="WP-KERNEL-012" updated_at="2026-09-24">

## F. Close only after proof

1. `[W12-F01]` After all required MTs independently PASS on applicable final inputs, the assigned validator/integration role performs the declared end-of-WP extra-build proofs in their scheduled grouped boundary, then whole-WP HBR/Argus/UserManual/diagnostics, full suite, Master Spec and integration-candidate review. A Kernel Builder does not waive a required proof, issue the WP verdict, sync to main, merge or push main. [Codex CX-503B1/CX-VAL-005; KB-AUTH-002/KB-HANDOFF-002/004]
2. `[W12-F02]` Preserve evidence and warm compiler reuse while needed. For any cleanup, resolve exact owned paths, verify no active process and no future reuse, estimate freed bytes **and** rebuild/time cost, then remove only demonstrably disposable output. Never delete/move/clean the whole warm target to satisfy the cap; never touch other owners' data. After actual WP PASS, the authorized closeout role handles declared cleanup/integration. [Operator target-preservation corrections; Codex CX-984-006/014/CX-107; KB-ART-001–005]
3. `[W12-F03]` Operator reports contain only MTs moved, pushed commits and blockers. If neither moved/pushed, write `no direct progress`; test count, CPU, a compile, a log, or this document is not product progress. A status answer may explain why, but must not imply PASS. [KB-OUT-006; Codex CX-EXEC-006]

</topic>

<topic id="verified-command-cards" wp="WP-KERNEL-012" updated_at="2026-09-24">

## Command cards — re-verify live paths and inputs before use

These are examples for the *existing* Windows/Git-Bash lanes, not permission to launch another round or test. The validator owns the runner commands, the builders own check/clippy, and the orchestrator owns only read-only inspection and its explicit governance commit. Do not run the launch card while a round is active. `C:\Program Files\Git\bin\bash.exe`, the runner and both warm targets existed on 2026-09-24; verify again on resume. [Codex CX-EXEC-009/CX-VAL-001/CX-SAFE-002; KB-OUT-008]

```powershell
# Run from wt-gov-kernel. Read-only roots and branch checks; no guessed cwd fallback.
$worktrees = (Resolve-Path -LiteralPath '..').Path
$product = Join-Path $worktrees 'wtc-native-editors-v1'
$kernel = (Get-Location).Path
$artifacts = Join-Path $worktrees 'Handshake_Artifacts'
$lane = Join-Path $artifacts 'WP-KERNEL-012\MT-109\wpv-c3x'
$target = 'C:\.target\WP-KERNEL-012\MT-109\wpv-c3x\target-r52'
$builderTarget = Join-Path $artifacts 'WP-KERNEL-012\MT-154\kb-c5\target'
@($product,$artifacts,$lane,$target,$builderTarget) | ForEach-Object { Resolve-Path -LiteralPath $_ }
git -C $kernel status --short --branch
git -C $product status --short --branch
git -C $product rev-parse HEAD
git -C $product ls-remote origin refs/heads/feat/WP-KERNEL-012
git -C $product worktree list
Get-CimInstance Win32_Process | Where-Object { $_.Name -match 'cargo|rustc|nextest' } |
    Select-Object ProcessId,Name,ParentProcessId,CreationDate,CommandLine
```

The next two checks are read-only; use exact canonical `MT-###.json`, and measure target bytes **before** asking the validator to launch. Account for a new archive (~0.26 GiB observed for each of `export-2bf51103` and `export-4ffda19b`) plus changed crate/test artifacts; do not claim the unused cap is a guaranteed build budget. [Codex CX-STATUS-001/CX-984-014; runner `check_target_cap`]

```powershell
$packet = Join-Path $kernel '.GOV\task_packets\WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1'
Get-ChildItem -LiteralPath $packet -File |
    Where-Object { $_.Name -match '^MT-\d{3}\.json$' } |
    ForEach-Object { (Get-Content -LiteralPath $_.FullName -Raw | ConvertFrom-Json).lifecycle.status } |
    Group-Object | Sort-Object Name | Select-Object Name,Count
$targetBytes = (Get-ChildItem -LiteralPath $target -Recurse -File | Measure-Object Length -Sum).Sum
$capBytes = 150000000000
[pscustomobject]@{ TargetBytes=$targetBytes; HeadroomBytes=$capBytes-$targetBytes;
                    CFreeBytes=(Get-PSDrive C).Free }
```

The builder check card is a **compile/static** card, not a test command. Select the actual changed crate/feature/test targets from the MT and source diff; serialize all D: Cargo work. The example below checks a named native test target without linking/running it. Run only when the builder owns D:, from the product worktree, with a stable source tree. Do not run it from C: or replace it with `cargo test`. [Operator 2026-09-24; KB-OUT-002/008; Codex CX-984-002]

```powershell
$env:CARGO_TARGET_DIR = $builderTarget
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
$env:CARGO_PROFILE_TEST_DEBUG = 'line-tables-only'
$env:TMP = Join-Path $artifacts 'WP-KERNEL-012\MT-154\kb-c5\tmp'
$env:TEMP = $env:TMP
cargo check --locked --manifest-path (Join-Path $product 'src\frontend\handshake_native\Cargo.toml') `
    --features integration,integration_tests,wgpu_screenshots --test test_author_id_budget
# After the builder inspects the exact diff and compile exit:
# git -C $product commit -m '<MT scope and change>' -- <explicit product paths>
# git -C $product push origin HEAD:refs/heads/feat/WP-KERNEL-012
# git -C $product ls-remote origin refs/heads/feat/WP-KERNEL-012
```

The validator's round card is shown for audit, **not** for the orchestrator/builder to execute. `run-round.sh` currently hardcodes `$target` and ignores `RUN_ROUND_TARGET`; the validator must verify/fix that discrepancy *after the running round* before using the Operator's variable contract. Preflight the script, both configs, every config reader, env/path length and cap first. Launch hidden with redirected logs and retain returned PID/start/command; never put a whole-run timeout around it. [Operator RUN_ROUND_TARGET direction; IV-OUT-005; Codex CX-VAL-001/004; runner header/`TARGET`]

```powershell
$sha = git -C $product rev-parse HEAD
if ($sha -notmatch '^[0-9a-f]{40}$') { throw 'full candidate SHA required' }
$runner = Join-Path $lane 'run-round.sh'
$gitBash = 'C:\Program Files\Git\bin\bash.exe'
Get-Content -LiteralPath $runner -Raw                 # inspect, do not skip
Get-Content -LiteralPath (Join-Path $lane 'nextest-core.toml') -Raw
Get-Content -LiteralPath (Join-Path $lane 'nextest.toml') -Raw
# Only the assigned validator, after completed preflight and when no round is live:
# $job = Start-Process -WindowStyle Hidden -FilePath $gitBash `
#     -ArgumentList ('"{0}" {1}' -f $runner,$sha) -PassThru `
#     -RedirectStandardOutput (Join-Path $lane ('logs\round-{0}.stdout.log' -f $sha.Substring(0,8))) `
#     -RedirectStandardError (Join-Path $lane ('logs\round-{0}.stderr.log' -f $sha.Substring(0,8)))
# $job | Select-Object Id,StartTime,Path
```

During the run, read the current log tail and process state without relaunching Cargo. At completion the validator reads both JUnit files, named MT proof lines and export/binary provenance before any verdict; grep counts alone never issue a verdict. [KB-STEER-004; Codex CX-EXEC-008/CX-VAL-001/004; WPV-OUT-006]

```powershell
$short = $sha.Substring(0,8)
Get-Content -LiteralPath (Join-Path $lane ("logs\round-$short.stderr.log")) -Tail 30
Get-Content -LiteralPath (Join-Path $lane ("logs\round-$short.stdout.log")) -Tail 15
Get-CimInstance Win32_Process | Where-Object { $_.Name -match 'cargo|rustc|nextest' } |
    Select-Object ProcessId,Name,ParentProcessId,CreationDate,CommandLine
# After round completion only:
# Test-Path -LiteralPath (Join-Path $lane ("junit-$sha-core.xml"))
# Test-Path -LiteralPath (Join-Path $lane ("junit-$sha-native.xml"))
# Get-FileHash -Algorithm SHA256 -LiteralPath <exact log or JUnit path>
```

</topic>

<topic id="failure-log" wp="WP-KERNEL-012" updated_at="2026-09-24">

## G. Failures to avoid — observed, not a replacement rulebook

Historical items 1–10 are from `HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-23_session6.md` §0/§4b; 11–25 are from `...session6-snapshot.md` §6. Their older proposed methods/statuses are **not** carried forward where the current pin or Operator superseded them.

1. `[W12-FL01]` I/preceding orchestration reported healthy lanes for hours with zero commits/verdicts. Correction: only pushed commits and MT moves count. [session6 §0 item 1; Codex CX-EXEC-006]
2. `[W12-FL02]` An earlier validator used the wrong breadth/filter: 0 selected tests, an out-of-scope hang and delayed per-MT verdicts. Historical suggestion of 3–5-MT queues is now superseded by the Operator's **one union of every READY MT**. [session6 §0 item 2; Codex CX-VAL-001/CX-EXEC-008]
3. `[W12-FL03]` Compiled work was held uncommitted (109 files for hours). Correction: explicit-path commit/push as soon as it compiles. [session6 §0 item 3; Codex CX-EXEC-007]
4. `[W12-FL04]` Builders ran costly Cargo tests. Correction: D: check/clippy only, validator only test runner. [session6 §0 item 4; Operator 2026-09-24]
5. `[W12-FL05]` Agents sat in long foreground commands and missed steering. Correction: asynchronous process/log handles and bounded monitoring. [session6 §0 item 5; KB-STEER-003]
6. `[W12-FL06]` I changed runtime disk and sync settings on theory before inspecting process dumps; they did not fix the hang. Worse, `SURREAL_DATASTORE_SYNC=never` was later proven unread by the embedded engine, invalidating my earlier inference. Correction: inspect actual reader and owned process evidence before changing env. [session6 §0 item 6, §4b]
7. `[W12-FL07]` I treated stale shared-target binaries as proof for a different export; archive timestamps can make the build cache misleading. Correction: require exact export/binary compile provenance. [session6 §4 item 4; Codex CX-VAL-001]
8. `[W12-FL08]` A validator wrote PASS on a failed run; only later independent audit caught it. Correction: parse required log results, status/verdict, actors and binary provenance **before** commit. [session6 §4 item 7; KB-HANDOFF-003]
9. `[W12-FL09]` The first hang diagnosis called the index-builder retry the cause while the supposed sync override was inactive. Correction: re-check any conclusion whose environmental premise failed; do not alter production based on a nonexistent setting. [session6 §4/§4b]
10. `[W12-FL10]` Earlier run planning proposed multiple special/per-MT proof builds before the union contract was settled. Correction: contract-declared extra builds at end-of-WP only, not opportunistic MT builds. [session6 §8; Codex CX-VAL-005; Operator one-union instruction]
11. `[W12-FL11]` Run 50 launched without disk/host preflight: target reached 193 GB over a 150 GB cap, C: approached the stop line and another load competed. Correction: measure target/free/growth/host before launch. [session6-snapshot §6 item 11]
12. `[W12-FL12]` Test OS state leaked 100 vault credentials, causing 15 failures. Correction: before/after OS-resource inventory and owned cleanup; do not clear another test's state without authority. [session6-snapshot §6 item 12; Codex CX-GIT-003]
13. `[W12-FL13]` Run 50 omitted `HANDSHAKE_WORKSPACE_ROOT` because I guessed the env list. Correction: trace each selected test's env readers to actual script exports. [session6-snapshot §6 item 13; WPV-OUT-005]
14. `[W12-FL14]` I proposed per-MT extra builds for MT-045/124/125. Correction: honor single-union plus contract-declared end-of-WP builds. [session6-snapshot §6 item 14; Codex CX-VAL-005]
15. `[W12-FL15]` I kept an invented/retired `PARTIAL_PENDING_OPERATOR_DECISION` and asked questions already answered. Correction: exact CX-STATUS-001 vocabulary, inspect MT `operator_decision`. [session6-snapshot §6 item 15]
16. `[W12-FL16]` I told a builder to hold pushes. Correction: push each compile-green commit immediately. [session6-snapshot §6 item 16; Codex CX-EXEC-007]
17. `[W12-FL17]` I tried to direct the validator's classification. Correction: relay evidence; independent validator classifies and issues verdict. [session6-snapshot §6 item 17; Codex CX-VAL-006]
18. `[W12-FL18]` I deleted the entire 193 GB warm target before run 52 and caused ~95 minutes of cold build/link. Correction: retain expensive compiled dependencies; no whole-target deletion. [session6-snapshot §6 item 18; Operator 2026-09-24]
19. `[W12-FL19]` I proposed moving the active target to D:, which would have forced another full rebuild on HDD. Correction: warm C: target under cap; calculate before suggesting migration. [session6-snapshot §6 item 19]
20. `[W12-FL20]` I recommended C: as a general WP build disk despite measured random-I/O/link cost. Correction: do not generalize from the special C: validator grant; builders stay on D:. [session6-snapshot §6 item 20; KB-OUT-008]
21. `[W12-FL21]` I changed shared nextest config without checking the core reader; native-only `owned-backend` group made core exit 96 with 0 tests. Correction: test every effective config consumer before launch. [session6-snapshot §6 item 21; IV-OUT-005]
22. `[W12-FL22]` I missed Windows 260-character deepest fixture path; 35 native failures were infrastructure. Correction: budget full generated leaf path, not only the evidence root. [session6-snapshot §6 item 22]
23. `[W12-FL23]` I suggested a rerun before reading `backend.stdout.log` already on disk, and misread the JSON `proof_commands` path. Correction: inspect exact artifact and contract first. [session6-snapshot §6 item 23]
24. `[W12-FL24]` I invented a justfile-deletion WP-end step from a passing Operator remark. Correction: cite an actual rule/contract before adding workflow. [session6-snapshot §6 item 24; Codex CX-EXEC-011]
25. `[W12-FL25]` I let the validator agent end its turn during run 52 and left it unwatched. Correction: persistent 10-minute tick and live validator through completion. [session6-snapshot §6 item 25; KB-STEER-002–005]
26. `[W12-FL26]` In session7 I treated a **post-pin** `canary_check=REPLACE_ME` as a WP-012 blocker, despite the pin and later Operator exclusion. I withdrew it after checking `git show 896f4e15` and the recorded exception. Correction: distinguish pinned law from later refactor. [session7 handoff `operator-rules-and-corrections`; packet `governance_pin`]
27. `[W12-FL27]` I repeated stale MT-033 and C1-FDELETE questions. C1-FDELETE was already decided in MT-109; MT-033's proof-command question was closed but its *literal unmapped AC* conflict remains. Correction: separate answered question from remaining acceptance gap. [MT-109.json `operator_decision`; MT-033.json `proof_commands`/`unmapped_acs`; session7 handoff]
28. `[W12-FL28]` I let an obsolete candidate round consume time while newer known fixes queued. The earlier `4ff` launch was invalid; the current `2bf51103` export is valid but immutable and necessarily cannot show later `31b4da3f`, `31e5d799`, `9d8bcff7`, `6f3cad14`, `ae7d1258` fixes. Correction: preflight and bundle all known stable fixes before **next** union; never claim current PASS for later code. [round logs; Codex CX-VAL-001]
29. `[W12-FL29]` I allowed a shared source edit during a D: check; that check exited green against changed inputs and had to be invalidated/repeated. Correction: source-hash/tree lock for check duration, shared-file quiet boundary. [session7 check30/check31 logs; KB-CARGO-SHARED-001]
30. `[W12-FL30]` I misread MT-032 `BLOCKED` plus null current `validator_verdict` as a status/verdict mismatch and interrupted the validator. CX-STATUS-001 permits BLOCKED for a named fix; equality is for an issued PASS/FAIL verdict. I corrected the message. Correction: inspect transition semantics before raising an alarm. [MT-032.json `lifecycle`; Codex CX-STATUS-001; KB-HANDOFF-003]
31. `[W12-FL31]` I did not prevent a later delete-guard change (`9c13b642`, included in frozen `2bf51103`) from breaking previously passed MT-032 runtime behavior. Four MT-032 delete tests and route6 failed `Specify a database to use` at statement 1; the validator moved old PASS_V4 to BLOCKED and the PASS count fell 120→119. The `9d8bcff7` context repair is pushed but unvalidated. Correction: old PASS can be invalidated by changed inputs; compile is not transaction proof; include it in the next union. [MT-032.json; `round-2bf51103.stderr.log`; product commits `9c13b642`/`9d8bcff7`]
32. `[W12-FL32]` I reported activity and partial green tests while no MT advanced to PASS. Correction: explicitly say `no direct progress`; distinguish test count from per-MT complete proof and verdict. [Codex CX-EXEC-006; KB-OUT-006]
33. `[W12-FL33]` I did not surface MT-033's remaining literal-contract conflict early enough. Read-only spec/code audit found four `unmapped_acs`; a passing binary cannot close them, and literal implementation crosses its allowed backend scope/AccessKit surface and conflicts with MT-066 stage role. Correction: the validator must adjudicate independently and cannot PASS on substitute proof; keep other MTs moving while the Operator-owned contract decision remains open. [MT-033.json AC/allowed paths/`unmapped_acs`; MT-066.json; Master Spec §7]
34. `[W12-FL34]` My frequent ad-hoc messages interrupted agents between required ticks. Correction: leave disjoint agents to work; send only the 10-minute observation or a new actionable failure, and let the validator own classification. [Operator 2026-09-24 `let you agents work`; KB-STEER-004]

These are accountability records, not claims that the fixes passed. Do not mark them resolved until the independent proof/status surface advances. [Codex CX-EXEC-006/008; KB-AUTH-002]

</topic>
