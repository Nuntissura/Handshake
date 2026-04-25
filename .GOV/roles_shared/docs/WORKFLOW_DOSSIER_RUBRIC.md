# Workflow Dossier Rubric

Use this rubric as the closeout judgment layer inside the Workflow Dossier. Mandatory for every dossier created from `.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md`.

## Purpose

- measure whether the workflow is getting smoother after each run
- measure whether the real Master Spec gap surface is actually shrinking
- measure whether operator and token cost are actually going down
- track communication maturity between governed roles
- track terminal hygiene and session lifecycle cleanliness

## Required Targets

Every qualifying review must score all five targets:

1. Workflow Smoothness
2. Master Spec Gap Reduction
3. Token Cost Pressure
4. Communication Maturity
5. Terminal and Session Hygiene

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
- `NUMERIC_SCORE: 0-10` (enables quantitative trend tracking across WPs)
- `Evidence:`
- `What improved:`
- `What still hurts:`
- `Next structural fix:`

At least one of the mandatory probe families above must be mentioned whenever it materially contributed to the target's current state.

Interpretation:

- `TREND` compares the current run to the most relevant predecessor dossier or audit baseline.
- `CURRENT_STATE` measures how much unresolved friction, debt, or cost is still present right now.
- `NUMERIC_SCORE` enables quantitative comparison across WPs. Use the scoring guidance below.

## Scoring Guidance

### Workflow Smoothness (0-10)

Evaluate:

- operator interruptions needed to keep the run valid
- extra Orchestrator steering beyond routing and hard-gate enforcement
- runtime, topology, scope, or status repairs needed after technical work was already correct
- declared role/worktree topology versus what actually got used
- whether closeout was atomic or repair-heavy
- silent failures where a surface looked green before truth was actually settled
- wrong command families or wrong role/tool surfaces being used and then corrected later
- whether microtask loop executed as designed (per-MT commits, per-MT review, bounded fix cycles)

Score guide:
- 0-2: Fully manual, orchestrator doing everything, no governed automation
- 3-4: Some automation works but orchestrator manually repairs most lifecycle steps
- 5-6: Core loop works (MT commits, validation) but closeout and communication are manual
- 7-8: Most lifecycle steps are automated, closeout mostly mechanical, communication mostly governed
- 9-10: Full governed lifecycle with mechanical closeout, governed communication, and zero manual repair

### Master Spec Gap Reduction (0-10)

Evaluate:

- which concrete product/spec gaps were actually closed
- whether the remaining gap list is smaller, narrower, and more explicit than before
- whether the run surfaced new adjacent debt or only repeated old uncertainty
- whether validation produced real negative proof rather than shallow PASS language
- whether task wording, scope wording, or file/path ownership was ambiguous enough to weaken implementation or review depth
- whether feature discovery (RGF-94) produced new primitives, stubs, matrix edges, or UI controls

Score guide:
- 0-2: No gaps closed, new debt surfaced, validation was shallow
- 3-4: One gap partially closed, some debt visible, validation had findings
- 5-6: Primary gap closed, validator found real issues, some stubs created
- 7-8: Gap closed with genuine negative proof, multiple stubs and discoveries, spec enrichment planned
- 9-10: Gap closed, all acceptance criteria proven, rich discovery output, no remaining ambiguity

### Token Cost Pressure (0-10)

Evaluate:

- repeated operator clarifications
- repeated Orchestrator steering prompts
- repeated status-sync or runtime-repair actions
- unnecessary session churn, topology churn, or duplicate reviews
- places where better gates, templates, or runtime truth would shorten future runs
- repeated rereading of large governance documents
- repeated command-surface inspection or `just --list`-style rediscovery
- repeated worktree/path/source-of-truth checks after startup when context had not materially changed
- polling vs fire-and-forget dispatch pattern
- refinement format iteration count

Score guide:
- 0-2: Most tokens spent on overhead (format iteration, polling, manual relay, closeout formatting)
- 3-4: Significant overhead but some productive work
- 5-6: Majority of tokens on productive work, moderate closeout overhead
- 7-8: Low overhead, efficient per-MT prompts, minimal closeout formatting
- 9-10: Nearly all tokens on productive work, mechanical closeout, zero polling

### Communication Maturity (0-10)

Evaluate:

- did the coder and validator communicate directly or only through orchestrator relay?
- were governed receipts (wp-review-request/response) used?
- were notifications (wp-notification) consumed to trigger role transitions?
- was the communication trail auditable (RECEIPTS.jsonl, NOTIFICATIONS.jsonl)?
- did the orchestrator act as a relay bottleneck or a monitor?

Score guide:
- 0-1: Zero communication between roles, orchestrator did everything
- 2-3: Orchestrator relayed messages via raw SEND_PROMPT, no governed receipts
- 4-5: Some governed receipts created, orchestrator still primary relay
- 6-7: Most communication through governed receipts, orchestrator monitors instead of relaying
- 8-9: Coder and validator communicate directly through governed surfaces, orchestrator intervenes only on exceptions
- 10: Fully governed communication loop with mechanical notification triggers, zero manual relay

### Terminal and Session Hygiene (0-10)

Evaluate:

- did terminal windows close automatically after sessions completed?
- were any blank/stale terminals left on the operator's desktop?
- did session reclamation work at closeout?
- were sessions cancelled and cleaned up properly?
- did the broker leave orphan processes or connections?

Score guide:
- 0-2: Multiple stale/blank terminals, no cleanup attempted
- 3-4: Terminals launched, cleanup attempted but partial (reclaimed_count=0, windows still open)
- 5-6: Most terminals cleaned up, some stragglers
- 7-8: Terminals close on completion, reclaim works, minimal stragglers
- 9-10: Zero stale terminals, auto-close on completion/failure/stale, clean session lifecycle

## Required Output Shape

Use this canonical section title in dossiers:

`## Workflow Dossier Closeout Rubric`

Then use these exact subsection titles:

- `### Workflow Smoothness`
- `### Master Spec Gap Reduction`
- `### Token Cost Pressure`
- `### Communication Maturity`
- `### Terminal and Session Hygiene`

## Mandatory Ambiguity and Silent-Failure Scan

Each review must also include one explicit section named:

`## Silent Failures, Command Surface Misuse, and Ambiguity Scan`

That section must cover:

- silent failures or false greens
- wrong tool or wrong command-family usage
- task/path/worktree ambiguity
- read amplification or repeated governance-document churn

Do not bury these items only inside the remediation list.

## Mandatory Drift Lens

Because the Workflow Dossier keeps mechanical evidence first, the closeout rubric must also assess:

- context drift between original session intention and final narrative
- cognitive drift between actual runtime movement and Orchestrator interpretation
- hallucinated or weakly supported claims in the final opinion layer

If drift is visible, name it explicitly and anchor it to ACP/runtime/receipt evidence.

## Non-Negotiable Rules

If a review claims improvement, it must also name:

- what specifically got better
- what still remains bad
- the next structural fix that should remove more friction or debt

Do not allow vague "moving in the right direction" language without receipts.

Repeated governance-document rereads, repeated command-surface rediscovery, and repeated path/worktree checks after startup are not neutral diligence. Treat them as evidence that the workflow is still ambiguous, too expensive, or both.

Every claim in the review must be verifiable. If a claim says "17/17 tests pass," the review must cite the test command and output location. If a claim says "validator found 6 issues," the review must cite the session output file and message IDs. Unverified claims must be marked `[UNVERIFIED]`.
