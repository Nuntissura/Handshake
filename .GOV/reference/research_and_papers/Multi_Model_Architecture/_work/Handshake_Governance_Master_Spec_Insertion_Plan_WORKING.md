# Handshake Governance Master Spec Insertion Plan Working Note

Temporary working note that turns the unified governance draft into a section-by-section insertion plan against the current master spec.

This document does not modify the master spec.
It is the patch map to use before any main-body editing.

## Purpose

- identify the exact current master-spec sections that should receive the software-delivery governance overlay clauses
- separate first-pass mergeable law from second-pass deeper runtime/control-plane law
- prevent wholesale document import or duplicate-law drift

## Governing Order

This plan assumes the repo's existing ordering rules:

1. patch Master Spec Main Body first
2. patch Appendix 12 / EOF blocks second
3. patch Roadmap and Coverage Matrix third
4. only then synchronize backlog-facing artifacts

This note is only about step 1: the main-body insertion plan.

## Source Inputs

Primary source to mine:

- `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md`

Supporting narrowed source:

- `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Mini_Spec_WORKING.md`

Current target authority:

- `.GOV/spec/Handshake_Master_Spec_v02.180.md`

## Merge Strategy

Do not create one new giant "governance harness" chapter.

Instead:

- patch existing sections in place
- specialize already-defined product law for the software-delivery overlay
- add only the minimum new record-family law that does not already have a clean existing home
- defer deeper state machines and richer control-plane semantics to a second enrichment pass

## First-Pass Main-Body Patch Set

### Patch 1: Governance Pack boundary and overlay authority

Current master-spec target:

- `7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)`
- anchor region around [Handshake_Master_Spec_v02.180.md:31838](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:31838>)

Why this section:

- it already defines Governance Pack, repo/runtime boundary, project-parameterized identity, and additive overlay posture
- it is the natural place to say what software-delivery governance means inside the product

Insert on first pass:

- software-delivery governance is one additive overlay profile inside Handshake, not the whole governance kernel
- imported repo-governance artifacts define overlay meaning, but product runtime owns live truth
- repo `/.GOV/**` remains governance workspace and canonical export/snapshot source where allowed, not live product runtime authority
- active software-delivery runtime state lives in product-owned records under `.handshake/gov/`
- imported overlay assets are classified as definition/import sources, not as live execution truth

Source from unified draft:

- [Compliance Posture](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:30>)
- [Executive Synthesis](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:61>)
- [Imported Overlay and Bounded Execution](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:535>)

Defer from this section:

- full schema definitions for every record family
- claim/lease and queued instruction primitives
- lifecycle state machines

### Patch 2: Governance Check Runner to validator-gate convergence

Current master-spec target:

- `7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD)`
- anchor region around [Handshake_Master_Spec_v02.180.md:31905](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:31905>)

Why this section:

- it already owns bounded imported check execution
- it already defines `CheckResult`
- it is the cleanest place to anchor how check execution affects canonical software-delivery gate posture

Insert on first pass:

- software-delivery validation posture MUST be represented in a dedicated validator-gate runtime record family
- that record family must bridge legacy gate compatibility posture and bounded `CheckResult` rollups
- non-pass, blocked, unsupported, and advisory outcomes must remain queryable through canonical runtime gate state
- imported check execution may contribute to gate posture, but may not itself become workflow truth

Source from unified draft:

- [Core Record Families](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:219>)
- [Validator gate record](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:290>)
- [Validator and Closeout Model](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:448>)

Defer from this section:

- full gate phase state machine
- pass-authority proof details beyond a concise first-pass rule
- per-check child-record decomposition

### Patch 3: Structured-collaboration substrate specialization for software-delivery runtime truth

Current master-spec target:

- `2.3.15` Locus / structured collaboration authority region
- especially the base envelope, compact summary, mirror sync, workflow-state, and transition-rule areas around:
  - [Handshake_Master_Spec_v02.180.md:6843](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:6843>)
  - [line 6878](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:6878>)
  - [line 6933](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:6933>)

Why this section:

- it already owns the shared collaboration substrate and routing law
- the software-delivery overlay should specialize this substrate instead of creating a parallel one

Insert on first pass:

- software-delivery overlay runtime truth resolves through canonical structured records, not packet prose, board order, mailbox chronology, or side-ledger files
- introduce the minimum record-role law for:
  - software-delivery work contract truth
  - workflow binding truth
  - governed action request/resolution
  - validator gate truth
  - checkpoint and evidence references
- make explicit that task packet Markdown survives as source artifact and readable mirror, not as mutable operational ledger
- state that overlay-specific fields remain profile-extension or software-delivery specializations over the shared base envelope

Source from unified draft:

- [Workflow-State and Routing Law](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:188>)
- [Core Record Families](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:219>)
- [Canonical Runtime Semantics](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:246>)

Important term-alignment note:

- do not blindly import every working record name as a new top-level primitive
- first align work-contract and workflow-binding semantics against existing Work Packet / Locus / Workflow Engine language
- validator gate record is the strongest candidate for a clearly new dedicated family

Defer from this section:

- claim/lease as a mandatory first-pass primitive
- queued instruction as a mandatory first-pass primitive
- full JSON schema examples

### Patch 4: Role Mailbox authority/transcription clarification for the software-delivery overlay

Current master-spec target:

- `2.6.8.10 Role Mailbox (Normative)`
- anchor region around [Handshake_Master_Spec_v02.180.md:10603](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:10603>)
- plus the thread-lifecycle / claim-or-lease / handoff expansions around:
  - [line 7021](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:7021>)
  - [line 7031](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:7031>)
  - [line 7040](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:7040>)

Why this section:

- mailbox already has strong non-authority rules
- the overlay needs only a tighter statement of where mailbox traffic lands in software-delivery runtime truth

Insert on first pass:

- mailbox summaries, announce-back traffic, and handoff notes may inform software-delivery work state but must not mutate authoritative work meaning directly
- authoritative software-delivery changes must resolve through governed actions or explicit transcription into canonical runtime records
- mailbox-linked waits, retries, or escalation posture contribute queue reasons and evidence, but do not become substitute workflow truth

Source from unified draft:

- [Normative Principles](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:99>)
- [Projection Model](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:480>)
- [Validator and Closeout Model](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:448>)

Defer from this section:

- claim/lease as a brand-new generalized product primitive unless the existing mailbox pass is insufficient
- queued instruction semantics

### Patch 5: Dev Command Center / Task Board first-pass control-plane clarification

Current master-spec target:

- existing DCC / Task Board main-body and appendix-owned projection law
- most important currently visible authority anchors:
  - [Handshake_Master_Spec_v02.180.md:83283](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:83283>)
  - [line 83307](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/spec/Handshake_Master_Spec_v02.180.md:83307>)

Why this section:

- DCC and Task Board already have strong projection law
- the overlay needs only a targeted extension saying which software-delivery control-plane state must be visible there

Insert on first pass:

- DCC should surface software-delivery work contract state, workflow-binding state, pending approvals, validator-gate posture, checkpoint lineage, and evidence/replay posture as projection over canonical runtime records
- Task Board remains a planning/synchronization mirror and must not become authority for state family, closeout truth, or next-action legality for the overlay
- readable packet/board mirrors must continue to expose reconciliation posture

Source from unified draft:

- [Projection Model](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:480>)
- [Evidence and Replay](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:516>)

Defer from this section:

- deeper health and alerting model
- backpressure and cost-aware routing
- explicit DCC queue states beyond first-pass visibility requirements

### Patch 6: Closeout as derived convergence

Current master-spec target:

- Locus / workflow-state / validator authority regions
- likely a short additive clause near workflow or validation/closeout law rather than a whole new chapter

Why this section:

- this is one of the strongest results from the research and one of the highest-value fixes to current repo-governance brittleness

Insert on first pass:

- software-delivery closeout must be derived from canonical runtime state, validator gate posture, governed action resolutions, and evidence, not from packet surgery or manual ledger convergence
- readable closeout sections may be synchronized after authoritative closeout becomes true
- closeout invalidity should be explained by explicit missing-owner / missing-evidence / unresolved-gate reasons

Source from unified draft:

- [Validator and Closeout Model](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:448>)

Defer from this section:

- the full closeout status object
- exhaustive closeout reason taxonomy

## Second-Pass Main-Body Patch Set

These are valuable, but should not be merged in the first pass.

### Second-pass A: Extension primitives

Candidate additions:

- claim or lease record family
- queued instruction record family

Why defer:

- both are useful, but they are not required to establish the initial overlay boundary and canonical truth model
- they add concurrency/control-plane complexity that should come after the first-pass authority model is accepted

Source from unified draft:

- [Core Record Families](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:219>)
- [State and Lifecycle Model](</D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Handshake_Governance_Unified_Technical_Spec_WORKING.md:326>)

### Second-pass B: Detailed state machines

Candidate additions:

- workflow-binding lifecycle
- governed-action lifecycle
- validator-gate lifecycle
- checkpoint lifecycle

Why defer:

- first-pass merge should establish law and record families, not freeze every transition table yet
- transition tables are more likely to need iteration during early implementation

### Second-pass C: Control-plane lifecycle and operational policy

Candidate additions:

- start / steer / cancel / close / recover sequences
- health model
- backpressure model
- alerting model
- cost-aware routing

Why defer:

- this is implementation-adjacent control-plane detail
- merge it after the canonical truth model has landed and term alignment against existing runtime/session primitives is complete

## Not Recommended For Main-Body Merge

Do not merge these into the master spec as normative main-body text in the first pass:

- external system registry
- research traceability appendix
- long-form blocker taxonomy
- full implementation roadmap from the unified draft
- prose that re-explains already-defined master-spec law without adding software-delivery overlay specificity

Those belong in:

- working notes
- ADRs
- later appendix or roadmap reflection

## Exact Working Merge Order

When actual master-spec editing begins, use this order:

1. patch `7.5.4.8 Governance Pack`
2. patch `7.5.4.9 Governance Check Runner`
3. patch `2.3.15` structured collaboration / Locus / workflow-state authority region
4. patch `2.6.8.10 Role Mailbox`
5. patch DCC / Task Board projection clauses
6. patch concise closeout derivation law

After that, do the mandatory follow-on maintenance in the usual repo order:

- Appendix 12 / EOF
- Roadmap phase reflection
- Coverage Matrix only if a new `## X.Y` row is introduced

## Working Decision

For the current merge effort:

- use `Handshake_Governance_Mini_Spec_WORKING.md` as the smallest first-pass clause source
- use `Handshake_Governance_Unified_Technical_Spec_WORKING.md` as the deeper technical source when a first-pass patch needs clarification
- avoid importing the unified draft wholesale into the master spec
