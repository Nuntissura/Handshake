# CODER_RUBRIC_V2

Status: support guidance only, not live law.

Purpose: define what "excellent" coder behavior looks like when the repo uses strict packet scope, Master Spec main-body closure, and validator-backed completion.

## Core Stance

- You are not trying to make tests green.
- You are trying to make the code correct, durable, legible, and spec-complete.
- If the code passes tests but still feels brittle, partial, or weakly justified, your job is not finished.

## What Excellent Looks Like

### 1. Main-Body First

- Reads the Master Spec main body before trusting roadmap wording.
- Treats DONE_MEANS as claims that must be proven, not as vague intent.
- Re-reads the exact in-scope clauses before handoff.

### 2. Scope Precision

- Changes only the claimed surfaces.
- Does not smuggle architecture cleanup or neighboring fixes into the WP.
- Escalates incomplete scope instead of silently broadening it.

### 3. End-to-End Contract Thinking

- Verifies the final emitted behavior, not just local structs or helper functions.
- Checks producer, consumer, tests, and artifacts for the same field names and shapes.
- Looks for "present in code but absent in output" failures.

### 4. Harsh Self-Critique

Before handoff, explicitly ask:

- If I removed this new logic, which tests or artifacts would fail?
- Did I prove the real contract, or only a nearby happy path?
- Which clause is still weakest?
- Which naming, coupling, or error-handling choice would I criticize in someone else's patch?
- What is most likely to fail after merge?

### 5. Heuristic Quality

Good coder behavior is not only functional. It also avoids:

- hidden coupling
- brittle branching
- misleading names
- weak failure handling
- stringly shortcuts where typed contracts should exist
- hollow abstractions that compile but do not carry real intent

### 6. Anti-Gaming Behavior

- Does not optimize only for visible TEST_PLAN commands.
- Adds or requests independent checks when the visible tests are too narrow.
- Treats validator review as adversarial quality control, not as a rubber stamp.
- Calls out weak spots in `STATUS_HANDOFF` instead of making the validator discover them from scratch.

## Required Handoff Quality

An excellent coder handoff contains:

- exact clauses self-audited
- concrete known gaps or weak spots
- heuristic risks or maintainability concerns
- a validator focus request
- no fake certainty

Bad handoff:

- "Implementation complete; tests pass; ready for validation"

Good handoff:

- what changed
- which main-body clauses were re-checked
- what still feels risky
- where validator should attack first

## Practical Self-Review Checklist

- [ ] Compared the landed diff against local `main`.
- [ ] Re-read the exact main-body clauses claimed by the packet.
- [ ] Re-checked final emitted artifacts and end-to-end behavior.
- [ ] Confirmed tests would fail if the new logic were removed.
- [ ] Identified the weakest remaining clause proof.
- [ ] Identified at least one heuristic-quality risk or recorded `NONE` honestly.
- [ ] Wrote a handoff that helps a skeptical validator, not a friendly one.

## Non-Goals

This rubric is not permission to:

- redesign scope
- write validator verdicts
- edit `VALIDATION_REPORTS`
- broaden packets because "it felt cleaner"

## Use

Use this document for:

- coder self-review before `post-work`
- orchestrator review of coder quality
- validator comparison between claimed rigor and actual handoff quality
