# VALIDATOR_ANTI_GAMING_RUBRIC

Status: support guidance only, not live law.

Purpose: reduce the chance that a coder or model can win by overfitting to visible tests, visible prompts, or visible checklist language while still missing the real contract.

## Core Stance

- Do not trust passing tests alone.
- Do not trust coder summaries alone.
- Do not trust packet wording if the Master Spec main body says more.
- A good validator is an independent evaluator, not a test re-runner.

## Anti-Gaming Principles

### 1. Build Your Own Review Target

Derive your review plan from:

- packet scope
- exact spec clauses
- diff against local `main`
- final emitted behavior

Do not let the coder's summary define the whole validation surface.

### 2. Separate Visible Checks From Independent Checks

Visible checks:

- TEST_PLAN
- post-work gate
- packet evidence

Independent checks:

- your own file reads
- your own clause map
- your own heuristic review
- your own spot checks
- your own counterexamples

### 3. Use Counterfactual Thinking

Ask:

- If I reverted this new branch or field, what would actually fail?
- If the coder only optimized for the named tests, what would still be broken?
- Which requirement is easiest to fake?
- Which artifact could look correct while still violating the contract?

### 4. Attack Semantic Weak Spots

Prioritize:

- emitted artifact shape drift
- serialization/deserialization mismatches
- producer/consumer name mismatches
- fields present in types but absent in output
- happy-path-only logic
- "validated locally" claims that do not survive diff-vs-main review

### 5. Judge Heuristics Separately

Tests can pass while the code is still poor.

Maintain a separate judgment for:

- naming clarity
- maintainability
- coupling
- error handling
- boundary discipline
- architectural fit

If those are weak, downgrade heuristic review even when tests pass.

## Independent Validator Techniques

### A. Mutation / Counterfactual Checks

- Mentally or mechanically remove a new condition, field, or branch.
- Ask whether any claimed proof would actually fail.
- If nothing meaningful breaks, the proof surface is weak.

### B. Differential Checks

- Compare behavior before/after against local `main`.
- Compare claimed scope against actual touched surfaces.
- Compare packet claims against emitted artifacts.

### C. Adversarial Examples

- Create at least one edge case the coder did not hand you.
- Prefer boundary and contract-shape failures over duplicate happy-path checks.

### D. Independent Test Generation

When necessary, create validator-owned spot checks that were not copied from the coder handoff.

Good targets:

- omitted required fields
- wrong enum/string values
- missing failure handling
- over-broad path changes
- regression in nearby shared surfaces

## Verdict Discipline

### PASS is strict

PASS means:

- clause proof is real
- code review is acceptable
- heuristic quality is acceptable
- residual risks are either `NONE` or explicitly downgraded into the verdicts

### PASS is not:

- "tests green"
- "looks mostly right"
- "probably intended"
- "good enough for now"

## What To Write Down

A strong validator report should capture:

- exact clauses reviewed
- exact unresolved clause gaps
- exact heuristic quality risks
- exact files read
- exact tests run or not run
- why tests were insufficient if they were insufficient

## Use

Use this rubric:

- during WP validation
- during integration validation
- when designing validator-owned independent checks
- when explaining why a test-green patch still does not deserve PASS
