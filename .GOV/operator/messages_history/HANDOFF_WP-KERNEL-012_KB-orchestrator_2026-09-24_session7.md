---
file_id: HANDOFF-WP-KERNEL-012-KB-ORCHESTRATOR-2026-09-24-SESSION7
file_kind: operator_handoff
updated_at: 2026-09-25
---

<topic id="live-state" wp="WP-KERNEL-012" updated_at="2026-09-25">

# WP-KERNEL-012 — restart handoff: one acceptance priority, shared repairs, union validation

This handoff is restart context, not a second authority or status surface. Read the LIVE kernel `.GOV/codex/Handshake_Codex_v1.4.md`, Kernel Builder and WP Validator protocols, build rules, assigned product worktree root `AGENTS.md`/`CLAUDE.md`, packet and selected MT JSON. The packet remains pinned to `896f4e15`, with explicit later Operator instructions taking precedence; this handoff does not re-pin it. Never use `handshake_main/.GOV` as live governance. The session6 snapshot and `WORKFLOW_WP-KERNEL-012_KB-orchestrator_2026-09-24.md` retain detailed failure history; their dated counts, live-agent claims and continuation instructions are not current state.

Verified 2026-09-25 from exact `MT-###.json` files: 119 PASS, 31 READY_FOR_VALIDATION, 8 BLOCKED, 1 FAIL (`MT-065 FAIL_V7`). Recount on restart; ignore `MT-136-candidate-boundary.json`. The latest completed clean-export union used frozen `1cdfde383ed4e30895f94552ce1b5f4873200ae8` (13:01–15:29 UTC): core 222 cases, native 2917 cases. MT-032 remains `READY_FOR_VALIDATION`, with null `validator_verdict` and `completed_by`; its independent `validation_v9` is inconclusive, not PASS or FAIL. No MT moved to PASS in the latest cycle. The older `2bf51103` union and MT-065 FAIL_V7 remain historical evidence.

Product worktree: `../wtc-native-editors-v1`, branch `feat/WP-KERNEL-012`, clean on inspection, HEAD `1cdfde383ed4e30895f94552ce1b5f4873200ae8` (live remote tip checked in the preceding cycle). Its pushed candidate includes `3b22f415` MT-032 collision/auth diagnostics, `584858e6` in-workspace Loom delete authorization/OS-vault marker, and `1cdfde38` Surreal schema pin. Governance `gov_kernel` HEAD is pushed `c78df7b0`, recording MT-032 `validation_v9`. Recheck remote and trees on restart.

The latest Operator-directed acceptance priority was MT-032. The `1cdfde38` round reached no new PASS/FAIL for that MT; the validator classified its live-proof failure as infrastructure and retained READY. This handoff does not itself authorize a new round, credential deletion, waiver, or changed acceptance method. Follow live Operator instructions and current authority.

Old builder/validator handles are not live assignments. The latest cycle used a persistent coding agent on Astra/medium and an independent WP_VALIDATOR on Sol/medium after the Operator-directed Terra/high-to-Sol/medium escalation; follow any newer live model assignment. Cheap read-only agents handle bounded evidence/spec questions when needed. The parent is KERNEL_BUILDER, never the acceptance or integration authority. A fresh agent inherits source state, caches, failures and attempt counts; it does not reset them. [Operator model/agent instructions; KB-LANES-001/002; KB-AUTH-002; CX-EXEC-003]

</topic>

<topic id="mt032-vault-blocker-and-gameplan-failure" wp="WP-KERNEL-012" updated_at="2026-09-25">

## MT-032: no acceptance movement; expensive preflight miss

The latest union consumed about 2.5 hours and moved **zero MTs** to PASS. MT-032 `validation_v9` is independently classified `inconclusive`/`infrastructure`, with no status effect. Both required live cases reached an owned backend but session exchange returned 403 before backlink, content-hash, restart or UI assertions. Each backend log identifies the real OS keychain write substep failing with Windows error 8; the ultimate host cause is not proven. No signed hardware waiver or substitute independent live proof exists.

A separate disposable, non-interactive `cmdkey /generic:<unique-probe> /user:<probe> /pass:<random>` returned exit 1, `Not enough memory resources are available to process this command`; it created no credential. Credential Manager listed 319 entries, including 100 matching Handshake session targets across 89 installation hashes. Those counts suggest store pressure but do not prove the Windows limit or establish which entries are disposable. Do not delete existing credentials from this handoff: they may represent real sessions, and neither an owned cleanup set nor approval has been established. An alternate OS-bound storage design is a product/security change, not a validation shortcut. Preserve the warm target and all proof.

We invoked `$gameplan` but used it mechanically. Orchestrator `before_round` passed 7/7, validator preflight 11/11, and `before_verdict` 2/2, yet no active check tested whether the Windows vault could accept a new credential after the prior native 403/error-8 evidence. The passing checklist therefore did not establish the environmental prerequisite for the costly live proof. This was my prelaunch failure, not validator proof of MT-032. Before another costly round, the existing gameplan and IV-OUT-005/CX-984-014 prelaunch evaluation must cover a safe, non-interactive, self-cleaning vault-write prerequisite (or independently verified equivalent); if writeability still fails, do not repeat the unchanged union. Record the observed cause as a gameplan step under [GLOBAL-GAMEPLAN-003], without treating that step or this handoff as an MT verdict. Any next-best proof or hardware-only waiver still needs the Operator's stated evidence and independent validator judgment; it cannot cover a code error.

</topic>

<topic id="goal-and-loop" wp="WP-KERNEL-012" updated_at="2026-09-25">

## Restart workflow

Drive every outstanding MT to a verified independent `PASS_Vn`, then the declared end-of-WP extra proofs and WP-boundary validation; hand integration to its authorized role. Product progress means pushed fixes and validator status changes, not reports or builds. The MT JSON alone owns status. Allowed statuses: `READY_FOR_VALIDATION`, `PASS_Vn`, `FAIL_Vn`, `BLOCKED` with exact `blocked_on`, `NEEDS_NEW_APPROACH`. Infrastructure failures change no status. A newer implementer commit may move a failed MT to READY, but only the independent validator issues PASS/FAIL.

1. `[W12-R01]` On authorized restart, choose one MT as the acceptance priority from current MT state and the live assignment; keep the remaining WP scope intact. One priority is not permission to narrow required validation. [Operator restart direction 2026-09-25; KB-IMPL-001; CX-VAL-001]
2. `[W12-R02]` Let the smallest evidenced shared defect define the repair scope, within its owning MT contracts. Include known related fixes needed by READY MTs in the same remediation batch; do not patch around a backend defect inside a proof-only MT. MT-065's test-file-only scope and backend workspace-list failure are the concrete example, not an assignment. [KB-IMPL-001/003; Operator same-remediation-run instruction; MT-065 `scope.allowed_paths`/`validation_v7`]
3. `[W12-R03]` Read the latest failure, exact code and already-pushed repairs before editing. Unknown cause -> inspect the relevant existing artifact; known code cause -> repair; infrastructure cause -> repair the observed environment/harness condition. No repeated run on unchanged relevant inputs without the permitted evidence-based reason; preserve attempt counts across sessions. [CX-EXEC-001/003/003A/004/012]
4. `[W12-R04]` Keep one owner for shared compile-graph files; batch schema/pin changes before checking. Relevant source/configuration must remain unchanged during the check; changed inputs invalidate that check. Trace runtime query/authorization context and affected callers, because Rust compilation does not prove embedded-query semantics. [KB-CARGO-SHARED-001; KB-PROOF-003; session7 check30/check31 and MT-032 findings]
5. `[W12-R05]` Builder runs only assigned D: warm-target check/clippy commands, with the required features and established profile. Commit explicit product paths and push immediately after the stable compile succeeds; record candidate SHA in existing MT state. No builder tests, per-MT test builds or `.GOV` commits on the product branch. [KB-OUT-001/002/008; CX-EXEC-007; CX-212F; Operator Cargo rules]
6. `[W12-R06]` Before dispatch or a costly action, resolve the current repo gameplan location and current skill instructions; use the existing WP gameplan and applicable role/moment checks. Show resume/next-moment steps on resume. Satisfy failed steps without bypass; preparation checks do not replace MT proof or change the governance pin. [GLOBAL-GAMEPLAN-001–006; CX-GP-001]
7. `[W12-R07]` Validator receives one full pushed candidate SHA after the known applicable repairs are ready. Freeze a clean git-archive export and run ONE union round covering every READY_FOR_VALIDATION MT. Later commits queue for the next round; a new agent, new priority MT or later push never restarts an active round. [CX-VAL-001/005; CX-EXEC-004/008]
8. `[W12-R08]` Before launch, verify effective script, required environment, every changed config reader, actual test selection, JUnit paths, deepest generated path, resource ownership and disk budget. No whole-round timeout. The last inspected runner hardcoded its C: target and did not consume `RUN_ROUND_TARGET`; verify the effective target rather than assuming an exported variable controls it. [IV-OUT-005; CX-984-008/014; WPV-OUT-003/005]
9. `[W12-R09]` Bind executed binaries and hashed results to the frozen export and compatible build inputs. An existing executable, hashed filename or export-marker file alone is insufficient provenance. Compiler-cache reuse and acceptance-proof reuse are separate decisions; proof reuse follows the recorded relevant-input diff rule. [CX-VAL-001/002; prior stale-binary incident]
10. `[W12-R10]` Validator maps results to each MT's own `proof`, top-level `proof_commands`, `unmapped_acs` and declared extra proofs. Record each defensible verdict immediately when that MT's complete evidence is available. Before committing it, check parsing, issued `status == validator_verdict`, independent actors and a passing record for every required check credited toward PASS. Infrastructure failures change no MT status. [CX-EXEC-008; CX-VAL-005/006; WPV-OUT-001/006; KB-HANDOFF-003]
11. `[W12-R11]` Keep the validator observing the run through completion; arm the Operator's 10-minute tick before launch. Use bounded asynchronous observations; relay new actionable failures promptly and keep unchanged ticks quiet. Do not interrupt agents repeatedly or treat silence/low CPU as proof of a hang. [KB-STEER-002–007; KB-OUT-004; Operator monitoring instruction]
12. `[W12-R12]` Feed the next observed failure back to its owning builder; measure acceptance progress by MT verdicts and distinguish pushed candidate fixes from PASS. Preserve all stop/authority boundaries. Use existing MT state and evidence; do not create another status ledger, checklist receipt or workflow report. [CX-EXEC-006/011; CX-914; KB-AUTH-002; GLOBAL-CLOSURE]

Per-test success does not establish complete MT acceptance. The earlier 27-MT run-52 proof gap and MT-128 question are historical prompts to inspect current contracts/results, not an instruction to rerun old binaries. MT-033 has `PC-033-01` (`test_ckc_embed`) and four explicit unmapped ACs; do not revive the closed 'no proof commands' question or replace the remaining literal acceptance obligations with a passing test count. Preserve the rule forbidding new branches/worktrees, including from test harnesses in an archive without `.git`.

</topic>

<topic id="build-reuse-and-capacity" wp="WP-KERNEL-012" updated_at="2026-09-25">

## Preserve warm builds

- `[W12-CACHE-01]` Reuse the existing C: validator target `C:/.target/WP-KERNEL-012/MT-109/wpv-c3x/target-r52` and D: builder target `../Handshake_Artifacts/WP-KERNEL-012/MT-154/kb-c5/target` (relative to this kernel checkout). A new agent or MT does not justify a new target or full rebuild. Preserve compatible dependencies; changed application/test inputs may still require recompilation/linking. [Operator target/restart instructions; KB-CAD-VPX-001]
- `[W12-CACHE-02]` Keep compiler/toolchain, feature, profile and relevant configuration compatibility explicit. The two targets are separate caches; D: check output is not automatically a C: test-build cache hit. Existing artifacts alone do not establish correct current-source binaries. [KB-CARGO-SHARED-001; CX-VAL-001]
- `[W12-CACHE-03]` Measure current target bytes, projected growth and disk availability before launch; cap C: output at 150,000,000,000 bytes and keep one Cargo per physical disk. Runtime stores, temporary files and evidence retain their assigned D: ownership. Preserve `CARGO_INCREMENTAL=0` and line-table debug profiles under the C: grant. [CX-984-001/002/014; Operator build rules]
- `[W12-CACHE-04]` Never wipe either warm target for a fresh agent, a clean slate or an unsupported corruption theory. Any selective cleanup requires exact ownership, no active use, demonstrated disposability, retained proof and a rebuild-cost justification within the Operator's cleanup authority. [Operator cache-preservation instructions; CX-984-006; CX-SAFE-001]

Measured after the `1cdfde38` round on 2026-09-25: C: target 143,178,636,234 logical bytes, leaving 6,821,363,766 bytes under the 150,000,000,000-byte cap. The earlier D: builder measurement was 33,376,324,116 bytes; remeasure before any new check. These are dated observations, not a growth forecast or clearance for another launch. The C: warm target was retained; no cache corruption requiring wholesale deletion was demonstrated.

</topic>

<topic id="open-failures" wp="WP-KERNEL-012" updated_at="2026-09-25">

## Current failure pointers and historical repair references

Current first-read pointer: MT-032 `validation_v9` records the independent `1cdfde38` round. All 10 named MT-032 core cases passed, including atomic and identity-collision delete. In native `test_loom_address`, 22/24 passed; the two live cases failed before feature assertions at session exchange 403. Their test-specific backend logs record `local_account_session_vault_write_failed phase="session_exchange_vault_write" keychain_backend=true windows_error_code=Some(8)`. Exact proof paths, hashes and candidate provenance are in `validation_v9`. This does not establish live MT-032 acceptance or a product-code failure. MT-065 `validation_v7` remains the separate FAIL_V7 pointer; inspect later product commits before repeating its repair.

The following is the historical run-52 repair list at session7 opening; current MT JSON and later candidate diffs supersede its open/fixed claims:

- MT-153: route matrix single-result panic (candidate fix `3496f18e`), owner workspace-delete 403 (candidate `d6d43a87`), transclusion 403 (not proven fixed).
- MT-154: knowledge upsert returned no record (candidate `d6d43a87`); other document 403s; predecessor-pin check matched a comment, not a changed pin value (validator classification pending).
- MT-155: `test_embeds.rs` fmt failure (candidate `582c97b2`; static proof pending).
- MT-157: owned workspace-delete 403 (candidate `d6d43a87`; proof pending).
- MT-158: route6 soft-delete 403, document remains; `f6b53481` is a diagnostic, not a fix. Act on its condition-specific result, then patch the cause.
- MT-159: `.handshake/gov-test-<pid>` fixture path versus expected `.handshake/gov` (candidate `b41a0d71`; proof pending).
- Other run-52 core failures are relayed in session6 §0b. Validator must map them to MT ownership from evidence; builder may repair an evidenced common root without waiting for a per-MT build.
- MT-045/124/125/142: recorded end-of-WP proof dependencies; read their current `blocked_on`. No substitute PASS.

The session6 handoff's C1-FDELETE 'open Operator decision' is stale: `MT-109.json.operator_decision` (2026-09-22) chooses authorized owner/admin cascade of rich documents, versions and Canvas boards with audit. The new request says the Operator will allow a hardware-only proof waiver if an explicit hardware blocker is measured and recorded with why the normal proof cannot run, the next-best proof/validation method and its actual result, and the reason that result justifies the conclusion. This waiver NEVER covers code, syntax, test assertion, authorization or other coding failures. Do not self-certify PASS: the validator still decides acceptance and records the evidence.

</topic>

<topic id="operator-rules-and-corrections" wp="WP-KERNEL-012" updated_at="2026-09-25">

## Session7 Operator directives and mistakes to avoid

During authorized execution, solve operational problems independently and use non-interactive commands. Resolve repo/spec/product questions with cheap read-only agents and Master Spec before escalating. No new worktrees/branches, no destructive git, no unapproved foreign-process stop. Keep the product/governance split; product commits on `feat/WP-KERNEL-012`, `.GOV` commits on `gov_kernel`, every commit with `git commit -- <explicit paths>`, push without asking. Keep one builder context per batch and one independent validator. Preserve disjoint file ownership. Builders use D: for check/clippy only; validator is the only test runner and uses C: warm target with one union round per candidate. Inspect diff, generated outputs, proof logs and binary provenance before any verdict. Never claim a status merely because a check was launched or a fix was pushed.

My session6/7 errors: I treated the undefined *later* HBR `canary_check` (`REPLACE_ME`) as a WP-012 blocker without checking its pin. Direct inspection of `git show 896f4e15:.GOV/codex/Handshake_Codex_v1.4.md` and the pinned HBR shows **no canary rule or field** at that pin. More decisively, post-pin gov commit `f6bbcaac` records the Operator's authority-fix instruction as **"apply all except host profile/canary"**; the session6 snapshot §0a records the same exclusion. The Operator's WP-specific instructions still require IV-OUT-005/CX-984-014 prelaunch checks. Do not invent a canary gate for WP-012 or wait for an Operator response. I also presented stale handoff questions (MT-033 and C1-FDELETE) as needing new decisions despite live resolutions. The session6 mistakes that cost time were deleting a warm 193 GB target, moving/recommending build disks without cost evidence, launching with a config valid for only one runner, missing Windows path length, rereading evidence after guessing, and leaving a long run unwatched. Do not repeat them. The Operator's instruction to solve operational problems independently does not authorize lowering product proof or waiving code errors.

The Operator now allows cleanup of target content only when demonstrably no longer needed and when it will not slow future work/tests. Before any cleanup, verify exact resolved owned paths, active process use, what compiler artifacts can be reused, expected freed bytes versus rebuild cost, and retain required evidence. Do not delete, move or clean the warm target merely to make a metric green; no broad deletion. A hardware waiver requires measured hardware evidence and the validator's next-best proof/result, not assertion or convenience.

Additional session7 lessons: shared schema edits invalidated compile check30 and required check31; the delete remediation failed previously passing MT-032 behavior; the `4ffda19b` round ended with missing/stale JUnit and native exit 127; frequent steering messages interrupted agents. Correct ownership, inspect full runtime boundaries, preflight actual consumers and allow bounded uninterrupted work. The final MT-065 FAIL_V7 was supported; the requested stop after that verdict was not a validator failure. Later commits during an immutable round are normal and do not retroactively change its proof.

Report only MTs moved, pushed commits and blockers; if none moved/pushed, say `no direct progress`. Documentation and advisory reviews do not complete the WP or authorize restarting stopped execution.

</topic>
