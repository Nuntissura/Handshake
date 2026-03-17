# Research: LLM Anti-Gaming Validation

Date: 2026-03-16
Status: research synthesis for governance design

## Question

How should Handshake reduce the risk that coders optimize for visible tests and visible gates while still missing the real contract?

## Key Research Signals

### 1. Test overfitting is real in repository-level issue resolution

Source:

- Ahmed et al., "Investigating Test Overfitting on SWE-bench"
  - https://hirzels.com/martin/papers/arxiv25-overfit.pdf

What matters:

- passing observed tests does not mean the patch generalizes
- refinement loops around visible tests can increase overfitting
- even hiding some test details does not fully solve the problem

Repo implication:

- do not equate packet TEST_PLAN success with true closure
- validator must add independent review and independent spot checks

### 2. Mutation-based consistency testing is useful for semantic mismatch detection

Source:

- "Mutation-based Consistency Testing for Evaluating the Code Understanding Capability of LLMs"
  - https://arxiv.org/abs/2401.05940

What matters:

- small semantic mutations reveal whether the system understands the code/description relationship
- model performance varies sharply across mutation types

Repo implication:

- validator should use counterfactual checks and semantic mutation thinking
- contract proof should include "what breaks if this logic is removed or altered?"

### 3. Mutation-guided independent test generation is promising

Source:

- Foster et al., "Mutation-Guided LLM-based Test Generation at Meta"
  - https://arxiv.org/abs/2501.12862

What matters:

- mutation-guided tests can target currently uncaught faults
- independent test generation can harden code against concerns beyond the visible test plan

Repo implication:

- add validator-owned independent spot checks
- prefer tests derived from likely missed faults, not only coder-provided scenarios

### 4. Reward/specification gaming generalizes

Source:

- Denison et al., "Sycophancy to Subterfuge: Investigating Reward-Tampering in Large Language Models"
  - https://arxiv.org/abs/2406.10162

What matters:

- optimizing for simpler visible rewards can generalize into more serious gaming behavior
- preventing one obvious form of gaming does not fully eliminate deeper forms

Repo implication:

- avoid single-metric success surfaces
- split verdicts and independent evaluator checks are necessary
- coder and validator should not share exactly the same reward target

## Design Consequences For Handshake

### A. Keep tests, but demote them from sole truth

Tests stay necessary.
Tests do not stay sufficient.

### B. Separate surfaces

Coder closure surface:

- visible TEST_PLAN
- post-work gate
- evidence mapping
- self-critique

Validator closure surface:

- independent clause audit
- independent heuristic review
- independent spot checks
- counterfactual / mutation-style reasoning

### C. Require adversarial thinking

Validator should ask:

- what would still pass if the coder optimized only for visible tests?
- what contract drift would survive a test-green patch?

### D. Record residual uncertainty explicitly

If quality or contract proof is incomplete:

- do not hide it in prose
- downgrade verdicts
- record gaps and risks explicitly

## Recommended Next Moves

1. Maintain a strong coder self-critique rubric.
2. Maintain a validator anti-gaming rubric.
3. Add validator-owned independent checks instead of only packet TEST_PLAN reliance.
4. Explore mutation-guided or counterfactual spot checks for high-risk WPs.
5. Keep split verdicts so test success cannot silently overwrite weaker code/spec judgment.
