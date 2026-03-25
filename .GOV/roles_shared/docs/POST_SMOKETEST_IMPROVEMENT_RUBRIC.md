# Post-Smoketest Improvement Rubric

Use this rubric after every live smoketest, workflow-proof run, recovery pass, closeout review, and workflow comparison audit.

Purpose:

- measure whether the workflow is getting smoother after each run
- measure whether the real Master Spec gap surface is actually shrinking
- measure whether operator and token cost are actually going down

This rubric is mandatory for smoketest reviews created from `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`.

## Required Targets

Every qualifying review must score all three targets:

1. Workflow Smoothness
2. Master Spec Gap Reduction
3. Token Cost Pressure

Do not skip a target even when there was no improvement.

## Mandatory Probe Families

Every qualifying review must also scan for these four failure families:

1. Silent failures and false greens
2. Systematic wrong tool or command calls
3. Task, path, or worktree ambiguity
4. Read amplification and governance-document churn

If any of these appeared in the run, they must be named explicitly in the review rather than being hidden inside vague "workflow friction" language.

## Required Fields Per Target

Each target must include:

- `TREND: IMPROVED | FLAT | REGRESSED`
- `CURRENT_STATE: LOW | MEDIUM | HIGH`
- `Evidence:`
- `What improved:`
- `What still hurts:`
- `Next structural fix:`

At least one of the mandatory probe families above must be mentioned whenever it materially contributed to the target's current state.

Interpretation:

- `TREND` compares the current run to the most relevant predecessor smoketest or audit baseline.
- `CURRENT_STATE` measures how much unresolved friction, debt, or cost is still present right now.

## Scoring Guidance

### Workflow Smoothness

Evaluate:

- operator interruptions needed to keep the run valid
- extra Orchestrator steering beyond routing and hard-gate enforcement
- runtime, topology, scope, or status repairs needed after technical work was already correct
- declared role/worktree topology versus what actually got used
- whether closeout was atomic or repair-heavy
- silent failures where a surface looked green before truth was actually settled
- wrong command families or wrong role/tool surfaces being used and then corrected later

Use `CURRENT_STATE: HIGH` when:

- the operator had to restate core lane rules
- the Orchestrator had to keep repairing the process in flight
- status or runtime truth lagged actual technical truth

### Master Spec Gap Reduction

Evaluate:

- which concrete product/spec gaps were actually closed
- whether the remaining gap list is smaller, narrower, and more explicit than before
- whether the run surfaced new adjacent debt or only repeated old uncertainty
- whether validation produced real negative proof rather than shallow PASS language
- whether task wording, scope wording, or file/path ownership was ambiguous enough to weaken implementation or review depth

Use `CURRENT_STATE: HIGH` when:

- the main gap surface is still broad
- reviews still rely on weak evidence or incomplete code inspection

Use `CURRENT_STATE: LOW` when:

- the signed scope is closed
- remaining debt is narrow, explicit, and clearly outside the packet

### Token Cost Pressure

Evaluate:

- repeated operator clarifications
- repeated Orchestrator steering prompts
- repeated status-sync or runtime-repair actions
- unnecessary session churn, topology churn, or duplicate reviews
- places where better gates, templates, or runtime truth would shorten future runs
- repeated rereading of large governance documents
- repeated command-surface inspection or `just --list`-style rediscovery
- repeated worktree/path/source-of-truth checks after startup when context had not materially changed

Use `CURRENT_STATE: HIGH` when:

- humans are still paying for procedural confusion
- the same steering pattern repeats within one run or across consecutive runs
- multiple roles keep rereading governance docs or rediscovering commands because the live task/path/tool surface is not crisp enough

Use `CURRENT_STATE: LOW` when:

- the lane is mostly one launch, one review loop, one closeout

## Output Shape

Use this exact section title in smoketest reviews:

`## Post-Smoketest Improvement Rubric`

Then use these exact subsection titles:

- `### Workflow Smoothness`
- `### Master Spec Gap Reduction`
- `### Token Cost Pressure`

## Mandatory Ambiguity and Silent-Failure Scan

Each review must also include one explicit section named:

`## Silent Failures, Command Surface Misuse, and Ambiguity Scan`

That section must cover:

- silent failures or false greens
- wrong tool or wrong command-family usage
- task/path/worktree ambiguity
- read amplification or repeated governance-document churn

Do not bury these items only inside the remediation list.

## Non-Negotiable Rule

If a review claims improvement, it must also name:

- what specifically got better
- what still remains bad
- the next structural fix that should remove more friction or debt

Do not allow vague "moving in the right direction" language without receipts.

Repeated governance-document rereads, repeated command-surface rediscovery, and repeated path/worktree checks after startup are not neutral diligence. Treat them as evidence that the workflow is still ambiguous, too expensive, or both.
