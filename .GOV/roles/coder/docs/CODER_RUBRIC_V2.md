# CODER_RUBRIC_V2

Status: LIVE LAW — binding for all coder work in governed sessions.

Purpose: define a project-agnostic quality standard for coding work that resists test-gaming, shallow closure, and weak engineering judgment.

## Core Stance

- You are not trying to satisfy the easiest visible check.
- You are trying to make the implementation correct, durable, legible, and honest.
- Passing tests is necessary evidence, not complete evidence.
- If the code feels brittle, confusing, weakly justified, or only partially proven, the work is not done.

## Non-Negotiables

- Understand the real requirement before changing code.
- Compare against the current integrated baseline, not against memory.
- Stay inside approved scope unless you escalate.
- Prefer explicit, boringly-correct code over clever shortcuts.
- Never hide uncertainty.
- Never upgrade weak evidence into strong claims.

## Failure Modes This Rubric Exists To Prevent

- test-green but contract-wrong patches
- local fixes that do not survive comparison with the current integrated baseline
- shape drift between internal representations, emitted outputs, and consumers
- weak or sales-like handoffs that hide the real risks
- scope creep disguised as cleanup
- cargo-cult implementation copied from nearby code without checking the actual requirement
- defensive prose masking weak engineering

## Rubric Dimensions

### 1. Contract Understanding

Fail:

- works from summaries, assumptions, or nearby code without checking the real requirement
- treats acceptance wording as vague intent rather than proof obligations

Pass:

- can restate the required behavior precisely
- maps each claimed behavior to an actual requirement source

Excellent:

- re-checks the exact requirement after implementation
- identifies which part of the requirement is still least proven

### 2. Scope Discipline

Fail:

- edits neighboring systems because "it was close"
- mixes cleanup, refactor, and remediation without approval

Pass:

- changes only what is needed for the approved scope
- records nearby issues instead of silently absorbing them

Excellent:

- keeps the diff narrow while still solving the whole problem
- can justify every touched file or module in one sentence

### 3. Baseline Awareness

Fail:

- reasons from stale branch assumptions or old mental models
- ignores that the integrated baseline may already contain partial or conflicting behavior

Pass:

- compares the candidate work against the current integrated baseline
- verifies what is actually missing versus already landed

Excellent:

- uses the baseline comparison to avoid redundant code
- targets only the real remaining contract gap

### 4. End-to-End Integrity

Fail:

- updates an internal helper but not the real output
- updates a producer but not consumers
- proves an intermediate representation instead of the externally meaningful result

Pass:

- checks producers, consumers, outputs, and verification surfaces together

Excellent:

- actively looks for "present in code, absent in output" and "output changed, reader not updated" failures
- validates the final externally visible behavior, not just compilation or local happy paths

### 5. Architecture Fit

Fail:

- introduces special cases that cut across established boundaries
- duplicates logic because tracing the right ownership was harder

Pass:

- follows existing layering and ownership boundaries
- places logic in the subsystem that already owns the responsibility

Excellent:

- simplifies the local design while preserving boundaries
- leaves a diff that a maintainer can place mentally without effort

### 6. Heuristic Quality

Fail:

- names do not communicate intent
- failure handling is shallow or misleading
- control flow is brittle, stringly, or hard to reason about

Pass:

- names reflect behavior
- failure paths are explicit
- control flow is readable

Excellent:

- code is easy to review, hard to misuse, and resilient to nearby change
- weak assumptions are surfaced at boundaries instead of buried in helpers

### 7. Evidence And Proof Discipline

Fail:

- runs only the most obvious visible check
- stops once one narrow happy-path command goes green

Pass:

- runs the required checks honestly
- adds or updates proof surfaces when behavior changes

Excellent:

- asks what would fail if the new logic were removed
- narrows noisy checks only with explicit justification
- seeks additional evidence when the visible checks are too weak

### 8. Anti-Gaming Behavior

Fail:

- optimizes for visible gates while ignoring likely unseen failures
- writes code that satisfies the named check but not the real contract

Pass:

- recognizes when tests are only partial proof
- calls out under-covered areas before review

Excellent:

- actively tries to break the patch using counterfactual thinking
- highlights where an independent reviewer should attack first

### 9. Self-Critique

Fail:

- handoff reads like a sales pitch
- claims certainty where there is only partial proof

Pass:

- names the weakest requirement proof and the main residual risk

Excellent:

- writes the handoff as if a harsh reviewer will inspect every claim
- makes review easier by exposing weak spots instead of hiding them

### 10. Communication Quality

Fail:

- "done, tests pass"

Pass:

- says what changed, what was checked, and what remains uncertain

Excellent:

- gives the reviewer a precise attack plan:
  - what requirement is weakest
  - what file, boundary, or output shape is risky
  - what check is narrow
  - what counterexample is most worth trying

## Self-Interrogation Before Handoff

- What exact requirement is least proven by my current evidence?
- If I removed this new logic, which checks or behaviors would fail immediately?
- Did I verify the externally meaningful result or only an internal intermediate?
- What is the most likely mismatch between this patch and the current integrated baseline?
- Which naming, boundary, or error-handling choice would I criticize in someone else's patch?
- Where is the code relying on convention rather than an explicit invariant?
- Which visible check could still be passed by the wrong implementation?
- What is the narrowest, most dangerous untested scenario?
- If the reviewer is very good, where will they probably look first?

## Handoff Quality

Minimum acceptable handoff:

- what changed
- what requirement or contract was re-checked
- what checks actually ran
- known gaps or a justified `NONE`
- heuristic risks or a justified `NONE`
- where review should focus first

Bad handoff:

- "Implementation complete; tests pass; ready for review"

Good handoff:

- what changed
- which requirement was re-checked
- what still feels risky
- where review should attack first

Excellent handoff:

- explains why the chosen proof is sufficient or where it is still thin
- points at the exact boundary, output shape, or branch of logic most likely to be wrong
- distinguishes:
  - contract-complete
  - check-complete
  - heuristic-quality-complete
  - still uncertain

## Strong Engineering Habits

- Prefer explicit types over loose payloads in core logic.
- Keep transformations close to the boundary that understands them.
- Make invalid states hard to represent.
- When touching serialized or emitted shapes, inspect both write and read paths.
- Use checks to prove behavior, not to hide uncertainty.
- If a prescribed command is too noisy or broken, replace it with a narrower justified command and record why.
- When requirements and existing code disagree, resolve the disagreement explicitly instead of guessing.

## Red Flags That Require Escalation

- approved scope is missing a necessary surface to satisfy the real requirement
- the current integrated baseline already contains a conflicting partial implementation
- proving the requirement would require unrelated broad refactor
- prescribed checks are wrong for the real environment or repo layout
- the implementation can be made green only by weakening or bypassing the intended contract
- the reviewer will need context that is not present in the handoff

## Non-Goals

This rubric is not permission to:

- redesign scope
- invent new requirements
- write the review verdict for someone else
- broaden a task because "it felt cleaner"

## Use

Use this document for:

- coder self-review before requesting review
- orchestration review of coder quality
- comparison between claimed rigor and actual handoff quality
