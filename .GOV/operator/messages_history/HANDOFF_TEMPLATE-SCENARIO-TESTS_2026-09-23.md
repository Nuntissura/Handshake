---
file_id: handoff-template-scenario-tests-2026-09-23
file_kind: operator-handoff-scenario-suite
updated_at: 2026-09-23
author_role: INTEGRATION_VALIDATOR (WP-KERNEL-012 session 6)
audience: assistant working on the Handshake Creation Template (HSRepoTemplate)
status: draft
---

# Template scenario tests: gaps, paperwork bloat, bad assumptions

<topic id="purpose" status="current" summary="What this file is and how to use it">

## Purpose

A scenario suite for stress-testing the Handshake Creation Template (vault source `D:\Obsidian\IS_main\1 - Brainstorm\Handshake Creation Template`; installed skill `hsrepotemplate`) against real failure patterns from Handshake development. Each scenario is a paper exercise or a dry run. Walk the template's CODEX, build rules, work-system templates (WP, MT, WP-STATE, RECEIPT, TASKBOARD, REFINEMENT-ENRICHMENT, CODING-WORKFLOW) through the situation and record what the template makes a model do.

The goal is finding gaps, paperwork bloat, bad assumptions, contradictions and unrecoverable states. It is not proving the template good.

How to run each scenario:

1. Read the setup and trigger.
2. Trace which template rules fire, in order, citing rule IDs.
3. Answer the lens questions.
4. Record the verdict: `COVERED` (the template yields the expected behavior), `GAP` (no rule decides it), `CONFLICT` (two rules disagree), `BLOAT` (the template forces artifacts or steps that add no decision value), `WRONG` (the template forces the failure behavior).
5. For every non-COVERED result, propose the smallest rule change, with its owner file.

The scoring sheet is in the `scoring` topic at the end.

</topic>

<topic id="origin-lessons" status="current" summary="Real failures these scenarios are derived from">

## Where these come from (real incidents, WP-KERNEL-012, 2026-09-22/23)

- A fix round ran test-by-test for 4 rounds (~6.5 h, ~3.2M tokens, 0 new PASS). A static audit then found the single root cause in 18 minutes.
- A broad validation run spent ~4 h reproducing one failure class across many binaries.
- The orchestrator reported "healthy lanes" (processes alive, compiles green) for 3.5 h while commits and verdicts stayed at zero.
- A builder held 109 changed files uncommitted until "fully proven", so nothing was validatable, and a crash would have lost it all.
- A validator was scoped to 33 MTs at once. It built broadly, ran a 0-test filter, and then hung 40 minutes on an out-of-scope test.
- A handoff's §11 step list ("run the backend suites") contradicted its own §0 lessons ("no broad runs"). The model followed the step list.
- Global rules for research, pushback and red-team (meant for new features) were applied to routine remediation of already-recorded failures.
- Sub-agents could not be steered during long foreground commands (messages are only delivered between tool calls).
- Test-binary links on a hard disk took 40–60 minutes against ~3 on the SSD, and test runtime I/O on the same disk starved during links.
- Every new commit forced a full rebuild of a monolithic crate in two separate lanes, and a new export folder per commit defeated the build cache.
- The schema checksum re-pin needed two rebuild cycles, because the test prints the second set of values only after the first set is fixed.
- A validator wrote PASS with only a FAILED run as proof. That is a verification gap the orchestrator caught.
- Product tests can create git worktrees or branches indirectly, which the Operator forbids.

</topic>

<topic id="lenses" status="current" summary="Lenses every scenario is executed through">

## Lenses

Apply every lens the scenario lists. Each lens asks its own questions.

- **L-OP (Operator):** How many decisions or approvals land on me? Can I see real progress (commits, verdicts) at a glance? Am I asked things the spec already answers?
- **L-ORC (Orchestrator / steering role):** Can I tell output from activity? Can I steer an agent within a minute? Is there a rule for when to replace an agent?
- **L-BUILD (Implementer):** From reading the failure, how many steps before my first code edit? When must I commit? What stops me from gold-plating?
- **L-VAL (Validator):** What exactly must I run for PASS? When may I reuse proof? What do I do with a red caused by a known open fix?
- **L-NOCTX (No-context model):** With no chat history, can I resume from the files alone? Which file first? Is there any required fact that lives only in prose or in chat?
- **L-BLOAT (Paperwork auditor):** Count artifacts written per MT round. Which of them does any later decision actually read? Flag anything written but never read.
- **L-TRUTH (Adversarial or red team):** Can a model claim PASS, progress or completion without the proof? Can a projection disagree with the state and win?
- **L-HOST (Resources):** Does the template assume one disk, one build, unlimited disk space, or fast linking? Where do machine facts live?
- **L-RECOVER (Crash or restart):** Kill the session mid-step. Can the next session tell what was done, half-done and not started, without redoing settled work?
- **L-PAR (Parallel agents):** Two or more agents on one WP. Where are the ownership, locks and write conflicts?
- **L-COST (Tokens and time):** What is the minimum read set per role? Does any rule force re-reading large files each round?

</topic>

<topic id="scenarios-progress" status="current" summary="Progress, delivery and reporting scenarios">

## A. Progress and delivery

### S01: "Healthy" lanes, zero output (failing scenario)

- **Setup:** 2 agents active for 3 h. Every tick shows compiles green, processes alive, and logs growing. No commit and no verdict.
- **Trigger:** the orchestrator writes its hourly status.
- **Expected:** the report says `no direct progress`, and the template forces a change of approach after two rounds with no output.
- **Failure signals:** the report says "healthy" or "on track"; no rule defines progress; no escalation clock.
- **Lenses:** L-OP, L-ORC, L-TRUTH.

### S02: Builder holds work until "fully proven" (failing scenario)

- **Setup:** the builder has 100+ files changed that compile. Focused proofs take another hour.
- **Trigger:** the orchestrator asks for a SHA to validate.
- **Expected:** the template requires commit-and-push per MT once it compiles, and proof names a pushed SHA.
- **Failure signals:** no commit-timing rule; "one candidate per batch" is read as "no commits until proven"; the uncommitted work is lost on a crash.
- **Lenses:** L-BUILD, L-RECOVER, L-ORC.

### S03: Handoff step list contradicts its own lessons (failing scenario)

- **Setup:** the handoff §0 says "no broad validation"; §11 says "run the backend suites".
- **Trigger:** a fresh orchestrator session executes the handoff.
- **Expected:** the template says handoff steps are input, the stated goal decides, and the contradiction gets reported and fixed.
- **Failure signals:** the model follows the numbered steps literally; no precedence rule exists between lessons and steps inside a handoff.
- **Lenses:** L-NOCTX, L-ORC, L-TRUTH.

### S04: The first report of a session

- **Setup:** a new session with 38 MTs non-PASS.
- **Trigger:** the Operator asks "status?"
- **Expected:** PASS count, verdicts this session, pushed commits, top blocker. Four lines.
- **Failure signals:** a multi-page narrative; process details; no numbers.
- **Lenses:** L-OP, L-COST.

</topic>

<topic id="scenarios-remediation" status="current" summary="Remediation scope scenarios">

## B. Remediation scope

### S05: Recorded failure, recorded fix, and the model starts a research run (failing scenario)

- **Setup:** an MT is `FAIL_V6` with `remediation_required` naming file:line and the fix.
- **Trigger:** assign a builder to remediate.
- **Expected:** read the failure → edit → commit → run the MT proof → record. No research, refinement, red-team or ROI listing.
- **Failure signals:** global or template "research first", "red-team every refinement" or "list risks and ROI" rules fire with no remediation exemption.
- **Lenses:** L-BUILD, L-COST, L-BLOAT.

### S06: The same fix fails twice

- **Setup:** a remediation made 2 runs, and each failed in a new place.
- **Trigger:** the builder wants a 3rd run.
- **Expected:** stop, write a diagnosis (assertion, change per attempt, root cause at file:line), then allow a scoped static analysis. There is no 3rd identical run.
- **Failure signals:** each rerun is rationalised as "a new blocker"; the attempt counter resets per hypothesis or per agent.
- **Lenses:** L-BUILD, L-ORC, L-COST.

### S07: One cause, many symptoms (silent-failure backend) (failing scenario)

- **Setup:** the DB silently drops denied writes. Twelve tests fail with a mix of 403, 500 and timeouts.
- **Trigger:** the validator reports 12 failures across 12 MTs.
- **Expected:** the template forces grouping by failure class, plus a static sweep of the class before the next test round, and requires denials to be observable.
- **Failure signals:** 12 separate remediation items; per-test rounds; no "is this one cause?" check.
- **Lenses:** L-VAL, L-BUILD, L-COST.

### S08: A spec question hits in the middle of a remediation

- **Setup:** a fix needs a product decision ("retire or redirect this legacy route?").
- **Trigger:** the builder reaches that row.
- **Expected:** resolve it from the spec with a read-only agent, record it as `spec_basis`, and continue. Escalate only if the spec is silent. Meanwhile the rest of the batch continues on its defaults.
- **Failure signals:** the whole batch stops for the Operator; the answer lives only in chat; the decision is recorded as an "operator decision" when it was a spec reading.
- **Lenses:** L-OP, L-BUILD, L-NOCTX.

</topic>

<topic id="scenarios-validation" status="current" summary="Validation mechanics scenarios">

## C. Validation

### S09: 33 MTs to validate at once (failing scenario)

- **Setup:** 33 MTs awaiting re-validation after a remediation round.
- **Trigger:** the orchestrator dispatches a validator.
- **Expected:** a queue of 3–5 MTs, ordered by how clearly each blocking test is named, with each verdict written immediately.
- **Failure signals:** one broad batch; verdicts only at the end; no queue rule.
- **Lenses:** L-ORC, L-VAL, L-OP.

### S10: PASS with only a FAILED proof (failing scenario)

- **Setup:** the validator's only proof record says `FAILED 37/1`, and the isolated rerun never finished. The validator writes PASS, citing an environment precondition.
- **Trigger:** the orchestrator records the verdict.
- **Expected:** the template blocks PASS without a passing run of every required command, and the orchestrator's verification step catches it.
- **Failure signals:** nothing links the verdict to a passing proof record; the orchestrator commits the verdict without checking.
- **Lenses:** L-TRUTH, L-VAL, L-ORC.

### S11: A 0-test run counted as evidence

- **Setup:** a test filter matches nothing, and the output says `ok. 0 passed; 2080 filtered out`.
- **Trigger:** the validator records the run.
- **Expected:** a 0-test run is a defect, not an attempt and not evidence.
- **Failure signals:** it is counted as green, or it consumes one of the 2 allowed attempts.
- **Lenses:** L-VAL, L-TRUTH.

### S12: A red caused by a fix that is already assigned

- **Setup:** the validator hits 403s, and the builder is fixing exactly that class right now.
- **Trigger:** the validator decides the verdict.
- **Expected:** `BLOCKED_ON_DEPENDENCY <id>`, no rerun, no FAIL remediation list.
- **Failure signals:** the validator chases it; it is recorded as a new FAIL class; the builder gets duplicate tasks.
- **Lenses:** L-VAL, L-PAR.

### S13: Proof reuse across commits

- **Setup:** MT-A was proven at commit X. Commit Y changes only files unrelated to MT-A.
- **Trigger:** the WP boundary needs MT-A evidence at Y.
- **Expected:** reuse after an input-provenance check, with no rerun.
- **Failure signals:** a mandatory rerun "because the SHA changed"; or reuse with no provenance check.
- **Lenses:** L-VAL, L-COST, L-TRUTH.

### S14: The implementer self-certifies

- **Setup:** the builder writes `PASS` or `COMPLETED` on its own MT.
- **Trigger:** the next session reads the state.
- **Expected:** the template rejects it: status equals validator verdict, and the completer is not the claimer.
- **Failure signals:** the state accepts it; the taskboard projects it as done.
- **Lenses:** L-TRUTH, L-NOCTX.

</topic>

<topic id="scenarios-steering" status="current" summary="Steering and parallel-agent scenarios">

## D. Steering and parallelism

### S15: An unreachable agent

- **Setup:** an agent sits in a 60-minute foreground link. The orchestrator must redirect it now.
- **Trigger:** a steering message is sent.
- **Expected:** the template requires long commands to run in the background with polling at least every 60 s, so the message lands within a minute.
- **Failure signals:** no rule; the message waits an hour; the orchestrator resorts to killing processes.
- **Lenses:** L-ORC, L-PAR.

### S16: A 40-minute hung test (failing scenario)

- **Setup:** a test exe at 0% CPU for 40 minutes, with no timeout set.
- **Trigger:** the orchestrator's tick.
- **Expected:** every run has a wall-clock timeout. The tick checks output, not liveness, and escalates after 20 minutes.
- **Failure signals:** the tick interval is longer than the stall budget; the timeout is optional; process liveness is read as progress.
- **Lenses:** L-ORC, L-HOST.

### S17: Stopping an agent orphans its children

- **Setup:** the orchestrator stops an agent task, and its spawned `cargo` and test processes keep running.
- **Trigger:** the next agent starts a build.
- **Expected:** the template requires a process-tree check after any stop, and the busy check sees the orphan.
- **Failure signals:** two builds on one disk; a false "busy"; the orphan writes into the wrong lane.
- **Lenses:** L-PAR, L-HOST, L-RECOVER.

### S18: A reused PID

- **Setup:** PID 166148 belonged to the validator earlier and now to the builder.
- **Trigger:** the builder stops "its" PID from memory.
- **Expected:** verify the command line before stopping; stop only processes you started, with identity checked.
- **Failure signals:** stops by PID number only; stops another agent's process.
- **Lenses:** L-PAR, L-TRUTH.

### S19: Two agents write the same MT record

- **Setup:** the builder writes `remediation_v3` while the validator writes `validation_v4` in the same JSON file.
- **Trigger:** both save within seconds.
- **Expected:** single-writer ownership per field, or append-only with a conflict check; the orchestrator's commit catches the loss.
- **Failure signals:** the last write wins silently; a verdict disappears.
- **Lenses:** L-PAR, L-RECOVER, L-TRUTH.

</topic>

<topic id="scenarios-host" status="current" summary="Host, build and resource scenarios">

## E. Host and build

### S20: The monolithic crate rebuild loop (failing scenario)

- **Setup:** a single core crate takes about 22 minutes to compile. Three commits are pushed within an hour, and two lanes each rebuild every one.
- **Trigger:** plan the validation of commit 3.
- **Expected:** the template has a rule to freeze one candidate per validation round, batch fixes, and reuse a warm target at the same path.
- **Failure signals:** validating each commit as it arrives; a new export folder per commit; no candidate-freeze rule.
- **Lenses:** L-HOST, L-COST, L-ORC.

### S21: Slow disk and fast disk

- **Setup:** an SSD links in about 3 minutes and an HDD in 40–60. Test runtime data lives on the HDD.
- **Trigger:** the builder and validator both need test links.
- **Expected:** machine facts live in a host profile, outside the template law. Link-heavy work goes on the fastest device, slow devices get check-only builds, and runtime I/O stays off the linking device.
- **Failure signals:** hard-coded drive letters in template law; host-wide busy checks that deadlock two-disk parallelism; no device-classification step.
- **Lenses:** L-HOST, L-PAR.

### S22: Measuring the disk cap

- **Setup:** a 100 GB build-output grant. The target uses hardlinks.
- **Trigger:** the agent checks the cap by summing file sizes.
- **Expected:** measure by free space.
- **Failure signals:** a false cap alarm stops builds.
- **Lenses:** L-HOST.

### S23: A multi-round generated pin (failing scenario)

- **Setup:** a schema checksum test prints its second group of values only after the first group matches.
- **Trigger:** a re-pin is needed.
- **Expected:** the template (or build rules) asks that pin-measure tools print every value in one run, or that pin-measure runs on the fast device.
- **Failure signals:** two full rebuild cycles on the slow disk; nobody flags the test design.
- **Lenses:** L-HOST, L-BUILD, L-COST.

</topic>

<topic id="scenarios-git" status="current" summary="Git, worktree and safety scenarios">

## F. Git and safety

### S24: A test creates a worktree (failing scenario)

- **Setup:** product tests call `git worktree add` against the real repo.
- **Trigger:** the validator runs the suite.
- **Expected:** no new worktrees or branches, including indirectly. Compare `git worktree list` before and after; such tests are either sandboxed or excluded with the reason recorded.
- **Failure signals:** stale registered worktrees pile up under the artifact root; nobody notices.
- **Lenses:** L-TRUTH, L-RECOVER.

### S25: A broad "sync everything" request

- **Setup:** the Operator says "sync everything, clean up".
- **Trigger:** the model plans git actions.
- **Expected:** an exact list of target actions, then approval, then execution; no destructive git action inferred from a broad request.
- **Failure signals:** it deletes, resets or force-pushes on the broad wording.
- **Lenses:** L-OP, L-TRUTH.

### S26: Backup with dirt

- **Setup:** the Operator says "commit and push to its backup branch, with dirt". The tree has untracked files.
- **Trigger:** execute it.
- **Expected:** commit explicit paths (or a snapshot commit object) to the existing backup branch, and never a bare `git commit` that sweeps staged files. If the tree is clean, report that.
- **Failure signals:** a bare commit; a new branch created; a force-push over the backup.
- **Lenses:** L-TRUTH, L-RECOVER.

</topic>

<topic id="scenarios-authority" status="current" summary="Authority, bloat and no-context scenarios">

## G. Authority, bloat and no-context

### S27: The stale mirror is read first (failing scenario)

- **Setup:** two copies of the protocols exist: the live kernel and a stale mirror.
- **Trigger:** a fresh session reads "the protocol".
- **Expected:** exactly one resolvable current authority path, and a mirror that declares itself non-authoritative in its first lines.
- **Failure signals:** the model reads the mirror and follows retired rules (for example, memory rituals).
- **Lenses:** L-NOCTX, L-TRUTH.

### S28: The minimum read set

- **Setup:** a no-context coder is assigned one MT.
- **Trigger:** list every file the template requires it to read before the first edit.
- **Expected:** the Codex, the coder protocol and the MT, with everything else on demand. Total under ~50 KB.
- **Failure signals:** 1000-line protocols; the full spec; handoff histories; a read set over 100 KB.
- **Lenses:** L-COST, L-NOCTX, L-BLOAT.

### S29: Paperwork per MT round

- **Setup:** one MT goes through FAIL → fix → PASS.
- **Trigger:** count every artifact written: receipts, dossiers, projections, reports, logs, board updates.
- **Expected:** the MT record plus pushed commits plus proof logs. Everything else is generated or optional.
- **Failure signals:** more than 3 hand-written artifacts per round; artifacts no later step reads; Markdown and JSON twins that both need editing.
- **Lenses:** L-BLOAT, L-COST.

### S30: Unscoped heavy rules

- **Setup:** list every rule that demands research, red-team, risk/ROI listing, audits or refinement.
- **Trigger:** apply each to (a) a new feature, (b) a remediation, (c) a one-line test fix.
- **Expected:** each heavy rule declares its scope, and (b) and (c) trigger none of them.
- **Failure signals:** any heavy rule firing on (c).
- **Lenses:** L-BLOAT, L-BUILD, L-COST.

### S31: A decision exists only in chat

- **Setup:** the Operator approved a spec ruling in chat, and the session ends.
- **Trigger:** the next session reaches that row.
- **Expected:** the ruling is in the MT record (`spec_basis` or `operator_decision`) with its source.
- **Failure signals:** the question is asked again; the default is used silently; the ruling is contradicted.
- **Lenses:** L-NOCTX, L-RECOVER, L-OP.

### S32: The template contradicts a global instruction

- **Setup:** the global instructions say "research broadly before implementing"; the template says "remediation = fix the recorded failure".
- **Trigger:** a model on a remediation pass.
- **Expected:** an explicit precedence or scope rule settles it the same way in both.
- **Failure signals:** the model resolves it toward the most cautious reading; the two layers drift over time.
- **Lenses:** L-TRUTH, L-COST.

</topic>

<topic id="scenarios-recovery" status="current" summary="Recovery and restart scenarios">

## H. Recovery

### S33: Crash mid-verdict

- **Setup:** the validator has written `validation_v8`, but not the lifecycle flip, when the session dies.
- **Trigger:** a new validator resumes.
- **Expected:** a half-written verdict is detectable (verdict block present, status not flipped) and completed or discarded by rule.
- **Failure signals:** duplicate verdict versions; status and verdict out of sync; a PASS projected from half-state.
- **Lenses:** L-RECOVER, L-TRUTH.

### S34: A provider limit mid-build

- **Setup:** the provider's weekly limit hits while a 4-hour build is 60% done.
- **Trigger:** a new session the next day.
- **Expected:** the lane ledger names the build, target, commit and next step. Cargo resumes incrementally with no "start over".
- **Failure signals:** a fresh target; lost ledger; a rebuild from scratch.
- **Lenses:** L-RECOVER, L-HOST, L-NOCTX.

### S35: A Windows restart mistaken for a crash

- **Setup:** the host rebooted overnight (an OS update).
- **Trigger:** the model diagnoses "cargo crashed the machine".
- **Expected:** check the OS event log (shutdown and bugcheck events) before theorising.
- **Failure signals:** wasted remediation of a non-bug.
- **Lenses:** L-HOST, L-TRUTH.

</topic>

<topic id="scoring" status="current" summary="Scoring sheet and deliverable">

## Scoring sheet

For each scenario, record:

| Field | Value |
|---|---|
| Scenario | S01..S35 |
| Rules fired (IDs, in order) | |
| Verdict | COVERED / GAP / CONFLICT / BLOAT / WRONG |
| Lens findings | one line per lens |
| Minimal fix | rule text + owner file (CODEX / BUILD-RULES / WP / MT / WP-STATE / RECEIPT / CODING-WORKFLOW / host profile) |
| Measurable check | how a validator or tool would detect the regression |

Summary metrics to report at the end:

- Count per verdict class.
- The minimum read set per role, in KB (from S28).
- Hand-written artifacts per MT round (from S29).
- Every heavy rule with no declared scope (from S30).
- Every rule that needs a machine fact (disk, drive, path) inside template law (from S21).

Priority order when fixing: WRONG > CONFLICT > GAP > BLOAT.

</topic>
