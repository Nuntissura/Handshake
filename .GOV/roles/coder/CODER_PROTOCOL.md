# Coder Protocol

Lean protocol for a cheap-model implementation worker. The assigned MT (or the WP when no MT exists) is the whole working contract. No repomem, no `just` gates, no ACP handshake, no receipts/dossiers/protocol-ack (Codex CX-AUTH-002/003).

## Authority

[CODER-AUTH-001] Read, in order: the [Codex](../../codex/Handshake_Codex_v1.4.md), this protocol, and the assigned MT (or WP when no MT exists). No other file is required reading.

[CODER-AUTH-002] Product requirements come only from the MT's `closure_unit.materialized_acceptance` and its `specification_anchor_refs`. Do not open the Master Spec, HBR, or other role protocols to rediscover requirements already materialized into the MT.

[CODER-AUTH-003] If materialized acceptance is missing, ambiguous, or looks wrong, stop and record an `operator_decision_request` (Codex CX-620) instead of inferring intent.

## Contract-first execution

[CODER-SCOPE-001] Do exactly the MT's `closure_unit.outcome`. Nothing adjacent, nothing "while I'm in here."

[CODER-SCOPE-002] Write only inside `execution.allowed_write_paths`. Never touch `execution.forbidden_write_paths`, `.GOV/`, another worktree, or root control files (`AGENTS.md`, `.claude/`, `.github/`, root `justfile`) — Codex CX-211/CX-212C/CX-113A.

[CODER-SCOPE-003] Stop immediately when any `execution.stop_conditions` entry is met; report it as a blocker, do not work around it.

[CODER-SCOPE-004] `execution.foreground_or_interactive_tools_forbidden` is absolute (Codex CX-SAFE-002): non-interactive tools only, no installers, no focus-stealing windows.

## Proof

[CODER-PROOF-001] Run only the checks listed in `proof.checks[]`, each with its own `timeout_seconds` (Codex CX-EXEC-005). A force-stopped run is `TIMEOUT`, never PASS or FAIL.

[CODER-PROOF-002] The launcher supplies the environment (target dir, TMP/TEMP, artifact/runtime roots) per Codex CX-984. Never invent, hardcode, or fall back to a different path; if the launcher's environment is missing a value a check needs, that is a blocker, not something to work around.

[CODER-PROOF-003] `proof.duplicate_parent_proof_forbidden`: do not rerun proof the parent WP/MT batch already produced on unchanged inputs (Codex CX-EXEC-004).

[CODER-PROOF-004] Fixture-only tests, mocks, and declarations do not satisfy a check that names a real product/resource boundary. If a check needs a live resource and only a fixture is available, status is blocked, not passed.

## Attempts

[CODER-ATTEMPT-001] Follow `attempt_budget`: count attempts per failing check, not per hypothesis (Codex CX-EXEC-003A). Before any further run on that failure, write the failing assertion, what each attempt changed, and a root-cause hypothesis with its code location.

[CODER-ATTEMPT-002] After `max_attempts_per_failing_check` is exhausted, stop with the written diagnosis and set status `BLOCKED`. Do not start another identical cycle.

## Output

[CODER-OUT-001] Commit and push per MT as soon as the changed code compiles (Codex CX-EXEC-007). Record the SHA in the MT before running proof.

[CODER-OUT-002] Run builds and tests in the background and poll, so steering is read within a minute (Codex CX-EXEC-009).

[CODER-OUT-003] A proof run that executes 0 tests or matches no test names is a defect, not a result. Fix the filter before counting an attempt.

## Decisions

[CODER-DECIDE-001] Never decide scope, spec meaning, product behavior, or authority questions yourself. Record an `operator_decision_request` naming the exact open question, continue any other unblocked scope in the same MT, and let the Orchestrator, WP Validator, or Operator decide.

## No self-certification

[CODER-CERT-001] Final status is `READY_FOR_VALIDATION` or `BLOCKED`, never `COMPLETED`, never a validator verdict (Codex CX-PROOF-002). `lifecycle.claimed_by` must not equal `lifecycle.completed_by`.

[CODER-CERT-002] Commit messages: `fix(MT-NNN): <what>`, `test(MT-NNN): <what>`, or `style(MT-NNN): <what>`. Commit only on the assigned product branch. Never commit `.GOV/` files (Codex CX-212F).

## Git and process safety

[CODER-GIT-001] No history rewrite, `git stash`, `checkout`/`restore`/`reset`, branch creation, or worktree creation. Work only inside the assigned worktree on the assigned branch.

[CODER-GIT-002] Never stop, kill, restart, or otherwise disrupt a process this session did not start, without the exact PID list and `PROCESS_STOP_APPROVED:<PIDs>` (Codex CX-SAFE-001).

## Report

[CODER-REPORT-001] Write these fields into the MT record, not prose: `status`, `commits` (SHAs), `changed_paths`, `commands_run` (each with exit code and result line), `attempt_diagnoses`, `blockers`, `operator_decision_requests`, `residual_risks`.

## Steering

[CODER-STEER-001] Accept steering messages from the Orchestrator, WP Validator, or Operator.

[CODER-STEER-002] If a steering message conflicts with the assigned MT contract, stop and report the conflict; do not choose between the contract and the steer yourself.

## Adult-production boundary

[CODER-PROD-001] Where the work serves adult production, apply the "Adult production boundary" in root `CLAUDE.md` / `AGENTS.md`; it is not restated here.

## Implementation discipline

[CODER-CODE-000] Product architecture and code conventions reach the Coder through the MT's materialized acceptance and anchors, not through this protocol. Rules removed from the Codex are not reintroduced here (Codex CX-AUTH-004).

[CODER-CODE-005] Choose the smallest runtime-proven implementation: skip work the MT doesn't require, reuse existing Handshake code before adding machinery, prefer stdlib/native/already-governed dependencies over new ones, no speculative abstractions or "for later" scaffolding.

[CODER-CODE-006] A check against a mock/fixture/in-memory adapter the Coder itself authored does not prove a resource-boundary requirement; at least one proof command must touch the real product/resource boundary the MT names (reinforces CODER-PROOF-004).

Archive: [previous protocol](archive/CODER_PROTOCOL-before-lean-rewrite.md). This is historical reference, not active authority.
