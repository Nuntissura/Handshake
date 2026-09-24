---
file_id: HANDOFF-WP-KERNEL-012-KB-ORCHESTRATOR-2026-09-24-SESSION7
file_kind: operator_handoff
updated_at: 2026-09-24
---

<topic id="live-state" wp="WP-KERNEL-012" updated_at="2026-09-24">

# WP-KERNEL-012 — KB orchestration handoff, session 7

This handoff supersedes the live-state and open-decision claims in `HANDOFF_WP-KERNEL-012_IV-orchestrator_2026-09-23_session6-snapshot.md`; its postmortem remains relevant. It is context, not authority. Read the LIVE kernel Codex, Kernel Builder and WP Validator protocols, build rules, root `handshake_main/AGENTS.md` and `CLAUDE.md`, the applicable MT JSON, and the WP pin (`896f4e15`). The packet's `governance_pin.rule` says this WP closes under Codex, role protocols and HBR **as of `896f4e15`**, except explicit later Operator decisions; later template/governance refactors do not re-judge it. Never use `handshake_main/.GOV` as the live governance copy.

At this handoff's creation, live MT JSON: 120 PASS, 29 READY_FOR_VALIDATION, 6 FAIL (153/154 `FAIL_V1`, 157/159 `FAIL_V2`, 155/158 `FAIL_V3`), 4 BLOCKED (045/124/125/142 for named end-of-WP proofs). No MT status was moved in this session yet. The older run-52 verdicts are against `1097ef1c` (MT-157's latest verdict is from run 50); newer product fixes have not received independent verdicts.

Product `feat/WP-KERNEL-012`: `d6d43a87` locally and at origin, clean on inspection. Since run 52: `582c97b2` MT-155 fmt, `3496f18e` MT-153 Loom row-set test, `b41a0d71` MT-159 executor fixture serialization, `f6b53481` MT-158 document-delete diagnostic only, `d6d43a87` MT-154 rich-document indexing permission and MT-154/157 owned Canvas cascade guard. These commits are candidates, not PASS proof. Governance `gov_kernel`: `2d141768` at origin before this handoff, with a pre-existing dirty edit to the session6 snapshot; preserve and commit it by explicit path as requested. The product tree had no dirt to commit.

The validator's warm target is `C:/.target/WP-KERNEL-012/MT-109/wpv-c3x/target-r52` (measured 91,776,844,224 bytes, about 85.5 GiB); C: had 854,086,643,712 bytes free. The grant caps this WP's C: build output at 150 GB; calculate target growth before every round. Never wipe a warm target to satisfy the cap. The builder uses only `Handshake_Artifacts/WP-KERNEL-012/MT-154/kb-c5/target` on D: for `cargo check`/clippy. One Cargo process per physical disk. No single-MT cargo tests.

Current sub-agents (new IDs because the session6 IDs cannot be resumed through this agent tool): builder `01a0d343-0c88-7c22-9e0c-24c2e8626a02` on Astra/medium; validator `01a0d343-3252-78b0-bccb-c4f5c84ebd32` on Sol/medium. Builder owns product fixes; validator owns union test execution, classification and verdicts. Parent owns neither verdict nor merge authority. On creation, no cargo/rustc/link/nextest process was seen. Poll agents and process/CPU/log observations every 10 minutes; keep the validator watching a long round, never end a turn while a round is running. Demand output at 20 minutes; replace an unproductive agent after 30 minutes per KB-OUT-004 without re-running unchanged tests.

</topic>

<topic id="goal-and-loop" wp="WP-KERNEL-012" updated_at="2026-09-24">

## Goal and cycle

Drive every outstanding MT to a verified independent `PASS_Vn`, then the declared end-of-WP extra proofs and WP-boundary validation; hand integration to its authorized role. Product progress means pushed fixes and validator status changes, not reports or builds. The MT JSON alone owns status. Allowed statuses: `READY_FOR_VALIDATION`, `PASS_Vn`, `FAIL_Vn`, `BLOCKED` with exact `blocked_on`, `NEEDS_NEW_APPROACH`. Infrastructure failures change no status. A newer implementer commit may move a failed MT to READY, but only the independent validator issues PASS/FAIL.

Loop: inspect the exact recorded failure and code; builder makes the smallest in-scope product fix, uses D: warm `cargo check --locked --tests`/clippy only, `git commit -m ... -- <explicit product paths>` and pushes immediately after compile; validator preflights the script, env, disk cap and all runner/config consumers per the Operator's IV-OUT-005/CX-984-014 direction; validator freezes one pushed SHA, exports it cleanly, and runs ONE union `run-round.sh <40-char-SHA>` covering every READY MT, with `RUN_ROUND_TARGET` set to the warm C: target; validator independently maps structured results to each MT's own proof checks and unmapped ACs, classifies product versus infrastructure, writes only defensible verdicts; relay concrete failures to builder and repeat on changed inputs. No per-MT builds/tests, no rerun of unchanged failures, no unmonitored long run. Reuse valid compiler artifacts and proof only under the recorded input-diff rule.

Existing `run-round.sh` has shortened evidence root, separate core/native nextest configs, and JUnit copy repair. Validate these and every actual runner invocation before launch. The run-52 native long-path failures are infrastructure; prefer the next candidate's union round over rebuilding/rerunning obsolete 1097ef1c binaries. The git-archive export has no `.git`, causing worktree-creating tests to fail as infrastructure; fix or prove a non-destructive export-compatible harness before judging them. The 27 READY MTs with top-level `proof_commands.commands` lacked at least one named run-52 target. MT-033 now has `PC-033-01` (`test_ckc_embed`) plus four explicit unmapped ACs; the old 'no proof commands' question is closed. MT-128 still awaits native proof.

</topic>

<topic id="open-failures" wp="WP-KERNEL-012" updated_at="2026-09-24">

## Recorded failures and direct next work

- MT-153: route matrix single-result panic (candidate fix `3496f18e`), owner workspace-delete 403 (candidate `d6d43a87`), transclusion 403 (not proven fixed).
- MT-154: knowledge upsert returned no record (candidate `d6d43a87`); other document 403s; predecessor-pin check matched a comment, not a changed pin value (validator classification pending).
- MT-155: `test_embeds.rs` fmt failure (candidate `582c97b2`; static proof pending).
- MT-157: owned workspace-delete 403 (candidate `d6d43a87`; proof pending).
- MT-158: route6 soft-delete 403, document remains; `f6b53481` is a diagnostic, not a fix. Act on its condition-specific result, then patch the cause.
- MT-159: `.handshake/gov-test-<pid>` fixture path versus expected `.handshake/gov` (candidate `b41a0d71`; proof pending).
- Other run-52 core failures are relayed in session6 §0b. Validator must map them to MT ownership from evidence; builder may repair an evidenced common root without waiting for a per-MT build.
- MT-045/124/125/142: stay BLOCKED on their recorded end-of-WP proof runs. No substitute PASS.

The session6 handoff's C1-FDELETE 'open Operator decision' is stale: `MT-109.json.operator_decision` (2026-09-22) chooses authorized owner/admin cascade of rich documents, versions and Canvas boards with audit. The new request says the Operator will allow a hardware-only proof waiver if an explicit hardware blocker is measured and recorded with why the normal proof cannot run, the next-best proof/validation method and its actual result, and the reason that result justifies the conclusion. This waiver NEVER covers code, syntax, test assertion, authorization or other coding failures. Do not self-certify PASS: the validator still decides acceptance and records the evidence.

</topic>

<topic id="operator-rules-and-corrections" wp="WP-KERNEL-012" updated_at="2026-09-24">

## Session7 Operator directives and mistakes to avoid

The Operator is AFK for a long time; do not wait for feedback or commands that prompt for input. Resolve repo/spec/product questions with cheap read-only agents and Master Spec before escalating. No new worktrees/branches, no destructive git, no unapproved foreign-process stop. Keep the product/governance split; product commits on `feat/WP-KERNEL-012`, `.GOV` commits on `gov_kernel`, every commit with `git commit -- <explicit paths>`, push without asking. Keep one builder context per batch and one independent validator. Preserve disjoint file ownership. Builders use D: for check/clippy only; validator is the only test runner and uses C: warm target with one union round per candidate. Inspect diff, generated outputs, proof logs and binary provenance before any verdict. Never claim a status merely because a check was launched or a fix was pushed.

My session6/7 errors: I treated the undefined *later* HBR `canary_check` (`REPLACE_ME`) as a WP-012 blocker without checking its pin. Direct inspection of `git show 896f4e15:.GOV/codex/Handshake_Codex_v1.4.md` and the pinned HBR shows **no canary rule or field** at that pin. More decisively, post-pin gov commit `f6bbcaac` records the Operator's authority-fix instruction as **"apply all except host profile/canary"**; the session6 snapshot §0a records the same exclusion. The Operator's WP-specific instructions still require IV-OUT-005/CX-984-014 prelaunch checks. Do not invent a canary gate for WP-012 or wait for an Operator response. I also presented stale handoff questions (MT-033 and C1-FDELETE) as needing new decisions despite live resolutions. The session6 mistakes that cost time were deleting a warm 193 GB target, moving/recommending build disks without cost evidence, launching with a config valid for only one runner, missing Windows path length, rereading evidence after guessing, and leaving a long run unwatched. Do not repeat them. The Operator's instruction to solve operational problems independently does not authorize lowering product proof or waiving code errors.

The Operator now allows cleanup of target content only when demonstrably no longer needed and when it will not slow future work/tests. Before any cleanup, verify exact resolved owned paths, active process use, what compiler artifacts can be reused, expected freed bytes versus rebuild cost, and retain required evidence. Do not delete, move or clean the warm target merely to make a metric green; no broad deletion. A hardware waiver requires measured hardware evidence and the validator's next-best proof/result, not assertion or convenience.

Report only MTs moved, pushed commits and blockers; if none moved/pushed, say `no direct progress`. Continue direct product and validator work after this handoff; documentation is not completion.

</topic>
