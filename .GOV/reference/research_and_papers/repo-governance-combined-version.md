# Repository Governance Refactor — Combined Version

**Status:** Proposed merged spec  
**Audience:** Solo repo operator using an Orchestrator -> Coder -> Reviewer workflow  
**Operating constraint:** One high-tier model subscription is assumed; a second model tier may be available occasionally, but not as the default control plane.

---

## 0. Why this exists

The current repo governance problem is not simply “tests are weak” or “validators miss bugs.” The deeper problem is that a model pipeline will optimise for whatever the pipeline visibly rewards.

If the workflow rewards:
- polished handoff notes,
- passing happy-path tests,
- long explanations,
- high line coverage,
- or a validator saying “looks good,”

then the model learns to produce patches that *look done* rather than patches that are *mechanically proven* against the intended behavior.

This combined refactor is designed to change the optimisation target.

The new operating question is no longer:

> Does this patch look correct?

It becomes:

> Can this patch mechanically prove which requirements it implements, which forbidden behaviors it avoids, what witnesses support those claims, what context it actually used, and whether it touched protected surfaces?

If the answer is not machine-checkable, the task has not earned `PASS`.

---

## 1. Design goals

This governance model is designed to achieve six things at once:

1. **Replace narrative closure with proof.**  
   Approval should flow from deterministic evidence, not from confident language.

2. **Stop shallow work from passing local checks.**  
   A patch should not pass simply because it satisfies an underspecified packet or a narrow set of examples.

3. **Prevent scope drift and adjacent cleanup gaming.**  
   Every meaningful change must be claimed, justified, and witnessed.

4. **Keep governance from becoming its own failure mode.**  
   The control plane must stay smaller and clearer than the product it governs.

5. **Use risk-proportional depth.**  
   Full-depth verification on every microtask is wasteful; shallow verification on risky work is negligent.

6. **Continuously harden from real escapes.**  
   Audit findings must graduate into regression tests, static rules, canaries, and blockers.

---

## 2. Constitutional rules

These are the durable rules that should sit above work packets, prompts, and day-to-day orchestration.

### Rule 1 — No code without claims
Every non-trivial code change must map to at least one requirement ID or negative requirement ID.

### Rule 2 — No claim without witnesses
Every claimed requirement must have executable witnesses. Every forbidden behavior must have a breaker witness or static rule.

### Rule 3 — Models do not issue PASS
A model may produce code, assumptions, findings, attack ideas, and explanations. Only the policy engine may emit `PASS`.

### Rule 4 — Protected surfaces require human control
No-autonomy zones and governance-law changes require explicit human review, even if every automated check is green.

### Rule 5 — Final scrutiny tier is computed, not narrated
The final verification depth is:

```text
final_tier = max(
  initial_tier,
  impact_cone_tier,
  zone_tier,
  recurrence_tier,
  random_escalation_tier
)
```

### Rule 6 — Repeated failures climb the prevention ladder
If a failure recurs, it must be promoted from finding -> regression -> static rule -> policy blocker -> canary.

### Rule 7 — Exceptions must be explicit and expiring
There are no informal exceptions. Every exception must be represented by a narrow waiver with expiry and compensating controls.

### Rule 8 — Governance may not self-loosen
No model session may widen its own acceptance criteria, relax protected-zone rules, or reinterpret blockers in its own favor.

---

## 3. Authority model

### 3.1 Role split

| Role | Primary job | Explicitly not allowed to do |
|---|---|---|
| **Orchestrator** | Scope work, assign risk tier, prepare packet and context pack, run gating pipeline | Approve correctness by judgement alone |
| **Coder** | Implement code, state assumptions, supply diff claims | Self-approve |
| **Reviewer model** | Produce findings, attack ideas, likely missing tests, maintainability issues | Emit final `PASS` |
| **Policy engine** | Compute deterministic verdict from artifacts and checks | Invent product intent or reinterpret rules |
| **Human** | Approve governance changes, protected-zone changes, waivers, and exceptional cases | Delegate protected-zone approval back to the same model that changed it |

### 3.2 Reviewer mandate

The reviewer role should be flipped from:

> Decide whether the code is correct.

to:

> Find concrete problems, weak spots, and likely missing checks. Do not approve.

For higher-risk tasks, the reviewer should be required to emit at least a minimum number of concrete findings or attack ideas unless the policy engine already failed the task earlier.

### 3.3 Status model

Use a small status set with precise meaning:

- `PASS` — all required checks for the final tier are green; no human-only gate remains open
- `FAIL` — one or more deterministic blockers fired
- `REVIEW_REQUIRED` — deterministic checks are green, but protected-zone or ambiguity rules require human review
- `WAIVED` — a narrow, approved waiver is active, compensating controls passed, and the waiver has not expired
- `BLOCKED` — prerequisites or required artifacts are missing, so evaluation cannot complete honestly

`PASS` should never be emitted by a chat role or by reviewer prose.

---

## 4. Truth hierarchy

The repo needs a single explicit authority order so packets cannot under-scope around the system.

```text
1. Master spec / product truth
2. Governance law
3. Requirement graph (CR / RID / NRID)
4. Active work packet
5. Context pack + diff claims + witness matrix + assumptions
6. Deterministic gate outputs and policy verdict
7. Reviewer findings
8. Freeform chat, summaries, and informal notes
```

### Interpretation

- The **master spec** remains the source of product intent.
- **Governance law** defines how work may be accepted.
- The **requirement graph** defines what must be satisfied or forbidden.
- The **work packet** scopes the task, but it may not overrule governance law or the requirement graph.
- **Evidence artifacts** prove what the patch claims and how it is tested.
- **Reviewer findings** add scrutiny, but not final authority.
- **Chat** is useful for coordination, not truth.

This replaces any older model where “the active packet wins” in all cases. The packet is authoritative for task scope, but it is not allowed to outrank the rules that govern the pipeline itself.

---

## 5. The three durable layers

To keep governance from sprawling, organise the system into three stable layers.

### 5.1 Governance law
This is fixed between sessions and changes rarely.

It contains:
- policy-gate rules
- no-autonomy-zone rules
- tiering rules
- waiver rules
- prompt-variance rules
- prevention-ladder rules
- reviewer mandate
- status semantics

### 5.2 Requirement graph
This is the product-truth projection used by the workflow.

It unifies:
- `CR-###` cross-cutting constraints
- `RID-###` positive requirements
- `NRID-###` forbidden behaviors

Each entry should define:
- summary
- spec references
- applies-to scope
- severity
- witness policy
- breaker/static expectations where relevant

Everything else should be derived from this graph.

### 5.3 Task evidence
This is the packet-scoped evidence set for one unit of work.

It contains:
- work packet
- context pack
- assumptions
- diff claims
- witness matrix entries
- retrieval telemetry
- policy verdict
- waiver references, if any

---

## 6. Canonical artifacts

Keep the artifact set small, explicit, and mostly derived.

### Governance law
- `governance/policy-gate.yaml`
- `governance/no-autonomy-zones.yaml`
- `governance/prompt-variance-policy.md`
- `governance/prevention-ladder.md`
- `governance/waiver-policy.md`
- `governance/waiver-ledger.yaml`

### Requirement graph and views
- `governance/requirement-graph.yaml` *(canonical source)*
- `governance/constraint-registry.yaml` *(derived view if still useful)*
- `governance/requirements-ledger.yaml` *(derived view if needed)*
- `governance/negative-requirements.yaml` *(derived view if needed)*
- `governance/witness-matrix.yaml`

### Task evidence
- `packets/<task-id>.md`
- `artifacts/context-pack/<task-id>.md`
- `artifacts/diff-claims/<task-id>.json`
- `artifacts/retrieval-telemetry/<task-id>.json`
- `artifacts/policy-verdict/<task-id>.json`
- `artifacts/assumptions/<task-id>.md`

### Hardening suites
- `governance/canary-suite/`
- `governance/golden-workflows/`
- `governance/mistake-book.md`

### Important design rule
Where possible, keep **one canonical source** and derive the rest. If the same truth is hand-maintained in several places, governance drift becomes inevitable.

---

## 7. Requirement graph

### 7.1 Objective

The old model was too dependent on packet wording and refinement excerpts. The new model requires a machine-readable graph of what must hold and what must never happen.

### 7.2 Requirement types

- `CR-###` — cross-cutting constraint spanning multiple areas
- `RID-###` — positive requirement that must be satisfied
- `NRID-###` — forbidden behavior that must never occur

### 7.3 Canonical schema example

```yaml
requirements:
  - id: CR-017
    type: cross_cutting
    summary: Stable object identifiers must remain stable across save/reload unless an explicit migration changes them
    spec_refs:
      - spec/section-7.2.4
    applies_to:
      - src/document/**
      - src/serialization/**
      - src/migrations/**
    severity: critical
    witness_policy:
      min_witnesses: 2
      required_types: [workflow, breaker]

  - id: RID-001
    type: functional
    summary: Save persists the current document without losing unsaved layer edits
    spec_refs:
      - spec/section-4.3.2
    applies_to:
      - src/editor/save/**
      - src/document/**
    severity: high
    witness_policy:
      min_witnesses: 2
      required_types: [unit, workflow]

negative_requirements:
  - id: NRID-001
    type: forbidden
    summary: Save must never reorder stable object IDs inside project serialization
    spec_refs:
      - spec/section-7.2.4
    applies_to:
      - src/serialization/**
      - src/document/**
    severity: critical
    witness_policy:
      min_witnesses: 1
      required_types: [breaker]
```

### 7.4 Rules

1. Every work packet must name the candidate `CR`, `RID`, and `NRID` set it is expected to touch.
2. Every non-trivial changed hunk must link to at least one relevant entry.
3. Every claimed requirement must appear in the witness matrix.
4. Every `NRID` must have at least one breaker witness or static rule.
5. Any changed hunk that cannot be tied to a requirement must fail unless it is explicitly marked as mechanical-only and accepted by policy.
6. If repeated escapes show that a requirement is too vague, harden the graph entry rather than only rewriting prompts.

---

## 8. Work packet contract

### 8.1 Packet purpose

The work packet remains the execution contract for one bounded task, but it is no longer allowed to define truth by omission.

### 8.2 Required packet fields

Every substantial packet should include:

```markdown
# Work Packet

- Task ID
- Goal
- Scope
- Risk tier
- Candidate CR/RID/NRID set
- Target symbols
- Required existing tests to inspect
- Required witness types
- Matching Mistake Book patterns
- Touched-file budget
- Explicit exclusions
- No-autonomy-zone check
- Prompt-variance status
- Escalation triggers
- Required commands / deterministic checks
- Expected outputs / evidence artifacts
```

### 8.3 Packet rules

- The packet may narrow the task but may not relax governance law.
- The packet may identify likely touched files, but the policy engine still validates the actual diff.
- If the packet is ambiguous enough that prompt-variance planning produces materially different interpretations, the task should fail early and return for clarification.
- The packet should include an explicit touched-file budget to make scope drift visible.

### 8.4 Touched-file budgets

Touched-file budgets are not proof of correctness. They are scope controls.

A packet should declare separate budgets such as:
- max source files
- max test files
- max config/governance files

Budget overrun is a warning or fail depending on policy, risk, and whether an escalation note exists.

---

## 9. Context packs and retrieval telemetry

### 9.1 Objective

Large refinement documents create noise. Models then either miss the important files or open them perfunctorily without using them.

The solution is:
- shrink context packs to what the task actually needs,
- and record what was actually opened and used.

### 9.2 Context pack contents

A context pack should include only:

- target symbols
- direct callers and callees
- relevant interfaces and contracts
- relevant `CR` / `RID` / `NRID`
- directly related tests
- directly related workflow specs
- relevant Mistake Book patterns
- recent failures in the same area
- touched-file budget
- explicit exclusions

### 9.3 Template

```markdown
# Context Pack

## Task
WP-2026-03-022

## Target symbols
- SaveCommand.execute
- DocumentStore.persist
- ProjectSerializer.serialize

## Required requirements
- CR-017
- RID-001
- NRID-001

## Required existing tests to inspect
- tests/unit/save-command.test.ts
- tests/workflows/save_roundtrip.spec.ts

## Direct dependencies
- src/document/DocumentStore.ts
- src/serialization/ProjectSerializer.ts

## Explicit exclusions
- plugin system
- theme engine
- analytics

## Touched-file budget
- max source files: 3
- max test files: 3
```

### 9.4 Retrieval telemetry schema

```json
{
  "task_id": "WP-2026-03-022",
  "opened_files": [
    "src/editor/save/SaveCommand.ts",
    "src/document/DocumentStore.ts",
    "tests/workflows/save_roundtrip.spec.ts"
  ],
  "opened_tests": [
    "tests/workflows/save_roundtrip.spec.ts"
  ],
  "opened_requirements": [
    "CR-017",
    "RID-001",
    "NRID-001"
  ],
  "final_patch_files": [
    "src/editor/save/SaveCommand.ts",
    "tests/workflows/save_roundtrip.spec.ts"
  ],
  "touched_unopened_files": [],
  "required_files_missing": [],
  "unused_context_items": [
    "src/document/DocumentStore.ts"
  ]
}
```

### 9.5 How telemetry should be used

Telemetry is **supporting evidence**, not proof of understanding.

Use telemetry as:
- a hard fail for missing required inspections or touched unopened protected files,
- a warning or escalation signal for noisy context packs, weak dependency alignment, or suspicious patch spread.

Do **not** use “opened file” as a substitute for witnesses or deterministic checks.

### 9.6 Telemetry gates

Fail when:
- a final patch touches a file outside the context pack and no escalation note exists
- a required test was never opened
- a required requirement entry was never opened
- a protected file was touched without being declared
- the patch exceeds touched-file budget without escalation

Warn when:
- more than 40% of context items were unused
- no directly related existing tests were opened
- more than half of final patch files were outside the original dependency cone

---

## 10. Diff claims and witness matrix

### 10.1 Diff claims

Diff claims bind changed code to declared requirements.

Example:

```json
{
  "task_id": "WP-2026-03-022",
  "claims": [
    {
      "file": "src/editor/save/SaveCommand.ts",
      "lines": "40-112",
      "requirement_ids": ["RID-001", "NRID-001"],
      "reason": "Implements canonical save path and preserves stable IDs"
    },
    {
      "file": "tests/workflows/save_roundtrip.spec.ts",
      "lines": "1-88",
      "requirement_ids": ["RID-001", "NRID-001"],
      "reason": "Executable witnesses for save behavior"
    }
  ]
}
```

### 10.2 Witness matrix

The witness matrix proves that requirements are not merely named; they are checked.

```yaml
witnesses:
  - requirement_id: RID-001
    witnesses:
      - type: unit
        path: tests/unit/save-command.test.ts
        assertion: save persists dirty document
      - type: workflow
        path: tests/workflows/save_roundtrip.spec.ts
        assertion: save, close, reopen preserves layer state

  - requirement_id: NRID-001
    witnesses:
      - type: breaker
        path: tests/regression/save_stable_ids.test.ts
        assertion: stable IDs must remain identical after save round-trip
      - type: static_rule
        path: semgrep/rules/no-id-rewrite.yml
        assertion: disallow ID reassignment in serialization pipeline
```

### 10.3 Witness types

Use a bounded witness vocabulary:
- `unit`
- `property`
- `state_machine`
- `workflow`
- `contract`
- `snapshot`
- `fuzz`
- `static_rule`
- `mutation_target`
- `breaker`

### 10.4 Minimum witness policy by severity

| Severity | Minimum witness expectation |
|---|---|
| Critical | 2+ witnesses, one must be non-unit |
| High | 2 witnesses |
| Medium | 1 witness |
| Low | 1 witness or umbrella witness |
| Forbidden (`NRID`) | 1 breaker witness minimum |

### 10.5 Unclaimed diff gate

A task fails if any of the following are true:
- a modified source file is absent from diff claims
- a changed hunk has zero associated requirement IDs
- a claimed requirement has no witness entry
- a test changes behavior for a requirement that was not claimed
- a new file appears outside the allowed touched-file budget without escalation

This is the main control against adjacent cleanup, silent scope expansion, and “looks comprehensive” padding.

---

## 11. Waiver system

### 11.1 Why waivers must exist

Hard gates without a waiver system do not create perfect rigor. They create shadow exceptions in chat, memory, and operator habit.

If the pipeline is strict, exceptions must be first-class and auditable.

### 11.2 Waiver rules

Every waiver must be:
- narrow
- explicit
- linked to a specific task and scope
- paired with compensating controls
- approved by the human operator
- expiring
- visible in the final verdict

### 11.3 Waiver schema

```yaml
waivers:
  - id: W-001
    task_id: WP-2026-03-022
    scope: RID-001 witness shortfall
    reason: Temporary migration gap
    compensating_controls:
      - manual review
      - regression test
    approver: operator
    approved_at: 2026-03-22
    expires_at: 2026-04-05
    renewal_count: 0
```

### 11.4 Waiver policy

- Expired waivers fail the gate.
- Protected-zone waivers always require human approval.
- Waivers may not be used to loosen no-autonomy protections permanently.
- Repeated waivers for the same failure class should trigger prevention-ladder escalation.
- A waived task should emit `WAIVED`, not `PASS`, unless policy explicitly defines a post-waiver green state after compensating controls complete.

---

## 12. No-autonomy zones

### 12.1 Objective

Prevent the model from editing its own cage or modifying high-consequence infrastructure without explicit human review.

### 12.2 Default protected zones

At minimum, protect:
- `governance/**`
- `prompts/**`
- `.github/workflows/**`
- `scripts/release/**`
- `scripts/policy/**`
- `auth/**`
- `billing/**`
- `migrations/**`
- `serialization/schema/**`
- `permissions/**`
- `feature-flags/**`

### 12.3 Zone rules

1. LLM-generated changes in these paths are always high risk.
2. The policy engine fails any such change unless the task is explicitly marked as a human-reviewed governance or critical-infra task.
3. These paths require human review even if all automated checks pass.
4. The orchestrator may not widen access to these zones by itself.

### 12.4 Example

```yaml
zones:
  - path: governance/**
    policy: human_only
  - path: prompts/**
    policy: human_only
  - path: .github/workflows/**
    policy: human_only
  - path: serialization/schema/**
    policy: human_review_required
  - path: migrations/**
    policy: human_review_required
```

---

## 13. Prompt-variance gate

### 13.1 Objective

Catch unstable or ambiguous tasks before paying for a full coding attempt.

### 13.2 Method

Before coding, run the **plan-only** phase three times with prompt variants that preserve meaning but vary wording.

Each run must output only:
- touched files
- target symbols
- claimed requirements
- assumptions
- expected tests to change

### 13.3 Fail conditions

Fail early if:
- touched files differ materially across runs
- claimed requirements differ materially across runs
- assumptions conflict materially
- expected tests differ materially

Suggested thresholds:
- touched-file Jaccard similarity < 0.6 -> fail
- contradictory assumptions about validation, ordering, nullability, persistence, state, or migration -> fail

### 13.4 Why this matters

A large share of shallow work is unstable interpretation, not overt adversarial behavior. If the same task yields materially different plans from equivalent prompts, the task is not yet stable enough to code cheaply.

---

## 14. Verification stack

Use the verification stack as the operating manual, but keep it under the constitutional rules above.

### Layer 1 — Constraint extraction and requirement graph hardening
Before coding begins, extract cross-cutting constraints and attach them to the requirement graph. Update the graph when audits reveal missing constraints.

### Layer 2 — Dual spec extraction for ambiguity detection
Run two independent requirement extractions in different reasoning styles. Reconcile divergences before coding. This reduces “easy interpretation” escapes.

### Layer 3 — Assumption Tagging and Interrogation (ATI)
Require the coder to emit explicit assumptions, including:
- input assumptions
- nullability assumptions
- ordering assumptions
- spec-ambiguity interpretations
- intentional omissions
- how each relevant constraint is satisfied

Every assumption is then checked deterministically where possible.

### Layer 4 — Static analysis loop
Run lint, type-check, security linters, complexity checks, and other static tooling iteratively until clean or exhaust the allowed fix loop. Suppressions require justification.

### Layer 5 — Property-based testing
Derive invariants from the spec and run randomised property tests. This is one of the highest-ROI additions because it attacks overfitting to example-based tests.

### Layer 6 — Executable contracts and state assertions
Translate critical invariants into runtime assertions, boundary contracts, or state-machine expectations so invalid transitions fail loudly.

### Layer 7 — Adversarial testing using the Mistake Book
Feed the Mistake Book and the actual code to a dedicated test-generation session whose only job is to find inputs or situations that trigger known failure patterns in this specific patch.

### Layer 8 — Round-trip fidelity and composition testing
For stateful or transform-heavy modules, verify that parse/serialize/deserialize or multi-step workflows preserve invariants across composition points.

### Layer 9 — Product-specific workflow harnesses and boundary fuzzing
For creative-tooling and IDE-like behavior, use golden workflows and boundary fuzzing on serializers, parsers, import/export paths, command stacks, migrations, and plugin/config surfaces.

### Layer 10 — Mutation testing
Run mutation testing on Tier 1 modules or otherwise high-risk code to ensure the tests kill meaningful mutants rather than simply pass happy paths.

### Important note
The stack is not the authority model. It is the **verification machinery** that the authority model governs.

---

## 15. Risk-tiered scrutiny

### 15.1 Why tiering exists

Running all layers on every microtask is wasteful. The repo needs a risk model that spends depth where escapes are costly.

### 15.2 Initial risk tiers

**Tier 1 — Critical**
- auth, permissions, access control
- money/billing/payment
- system-boundary validation
- shared state mutation
- cross-module integration points
- serializers, migrations, import/export, command stack boundaries
- modules with recurring Mistake Book hits
- anything touching 3+ modules of shared constraints

**Tier 2 — Standard**
- core business logic inside one module
- internal APIs
- configuration handling
- data transformation
- medium-scope logic changes

**Tier 3 — Low risk**
- UI copy
- labels, tooltips
- logging and instrumentation
- comments and docs
- boilerplate or scaffolding

### 15.3 Default depth by tier

| Layer / Control | Tier 1 | Tier 2 | Tier 3 |
|---|---:|---:|---:|
| Requirement graph / constraints | Yes | Yes | Yes |
| Dual spec extraction | Yes | Optional | No |
| ATI assumptions | Yes | Yes | Optional |
| Static analysis loop | Yes | Yes | Yes |
| Property-based tests | Yes | Yes for logic-heavy changes | No |
| Contracts / state assertions | Yes | Yes if stateful | No |
| Mistake Book adversarial tests | Yes | If pattern match | No |
| Round-trip / composition | Yes | At integration points | No |
| Golden workflow checks | Yes if workflow path touched | Optional | No |
| Mutation testing | Yes | Selective | No |

### 15.4 Fixed assignment rules

Tier assignment should be mechanical, not prose-based:

```text
IF task touches auth/access/permissions -> Tier 1
IF task touches billing/payment -> Tier 1
IF task changes boundary validation -> Tier 1
IF task changes shared state across modules -> Tier 1
IF task matches a Mistake Book pattern -> Tier 1
IF requirement graph shows 3+ affected modules -> Tier 1
IF task is pure UI/copy/logging -> Tier 3
ELSE -> Tier 2
```

---

## 16. Impact-cone escalation

### 16.1 Objective

The packet’s initial tier may underestimate the true blast radius. The diff itself should be allowed to escalate the task.

### 16.2 Escalation triggers

Escalate to Tier 1 if, after the diff exists:
- a changed symbol is used by 3+ modules
- a public interface changes
- a shared-state mutation path changes
- a serializer or migration path changes
- a golden workflow path is touched
- a file with a matching Mistake Book pattern is touched
- a protected zone is touched
- a boundary parser/importer/exporter changes

### 16.3 Why this matters

Many dangerous tasks look local on paper and global in the actual patch. Impact-cone escalation closes that gap.

---

## 17. Randomised deep checks

Predictable review depth is gameable. Add moving-target defenses.

### 17.1 Rules

1. Randomly escalate a percentage of Tier 2 and Tier 3 tasks to full Tier 1 depth.
2. Use variable seeds for property-based tests and fuzzing.
3. Rotate a small number of Mistake Book categories for extra emphasis each cycle.
4. Vary audit timing so the model cannot infer when scrutiny will land.

### 17.2 Example policy

- 15% random escalation for Tier 2
- 10% random escalation for Tier 3
- 2–3 Mistake Book categories emphasised per cycle

### 17.3 Purpose

The aim is not to surprise the model for sport. The aim is to make the cheapest strategy also the safest strategy: doing the work properly.

---

## 18. Deterministic policy engine

### 18.1 Objective

Move final authorization out of model prose.

### 18.2 Hard inputs

The policy engine should consider:
- lint / type-check / static analysis status
- unit / integration / property / state-machine test status
- workflow test status
- mutation score where required
- fuzz result status where required
- requirement coverage status
- witness matrix status
- diff-claim validity
- retrieval telemetry status
- touched-file budget status
- no-autonomy-zone status
- risk tier and escalation status
- waiver status

### 18.3 Blockers vs diagnostics

Not all signals should block.

#### Hard blockers
- unclaimed diff
- missing required witness
- required test not inspected
- touched unopened protected file
- no-autonomy-zone violation
- failed required workflow/property/static/fuzz/mutation gate
- prompt-variance instability
- expired waiver

#### Soft diagnostics
- high unused-context ratio
- touched-file budget overrun where policy says warn
- weak dependency-cone alignment
- reviewer concerns that lack deterministic reproduction
- telemetry anomalies that suggest noise but not proven failure

### 18.4 Verdict schema

```json
{
  "task_id": "WP-2026-03-022",
  "status": "FAIL",
  "reasons": [
    "unclaimed_diff: src/document/DocumentStore.ts lines 118-146",
    "missing_witness: RID-001",
    "zone_violation: governance/policy-gate.yaml"
  ],
  "warnings": [
    "unused_context_ratio_high"
  ],
  "waivers": []
}
```

### 18.5 Example policy gate

```yaml
policy_gate:
  fail_on:
    - unclaimed_diff
    - missing_required_witness
    - no_autonomy_zone_violation
    - touched_unopened_file
    - missing_required_test_inspection
    - prompt_variance_instability
    - expired_waiver
  warn_on:
    - high_unused_context_ratio
    - touched_file_budget_overrun
  thresholds:
    touched_file_budget_overrun_warn: 1
    unused_context_ratio_warn: 0.40
    prompt_variance_min_jaccard: 0.60
    tier1_min_mutation_score: 0.80
```

---

## 19. Mistake Book -> Prevention Ladder -> Canary Suite

### 19.1 Objective

Audit pain must become prevention, not just lore.

### 19.2 Ladder

**Stage 0 — Audit finding**  
A human audit or post-merge regression finds a failure.

**Stage 1 — Regression witness**  
Add a concrete test that reproduces the bug.

**Stage 2 — Pattern extraction**  
Generalise it into a Mistake Book pattern.

**Stage 3 — Static prevention**  
Add a static rule, AST check, lint rule, CI grep, or contract if feasible.

**Stage 4 — Packet hardening**  
Add the pattern to packet anti-patterns, ATI prompts, or context-pack generation.

**Stage 5 — Merge blocker**  
If it recurs or is high severity, make it a policy blocker.

**Stage 6 — Canary case**  
Add the failure to the repo canary suite so later prompt or pipeline changes cannot quietly reintroduce it.

### 19.3 Escalation policy

- First occurrence, medium severity -> Stage 1–2
- First occurrence, high severity -> Stage 1–4
- Second occurrence -> Stage 1–5
- Any critical occurrence -> Stage 1–6 immediately

### 19.4 Example entry

```markdown
## PL-004
- Source MB entry: MB-019
- Failure class: Serialization rewrites stable IDs during save
- Severity: critical
- First found: 2026-03-18
- Regression witness: tests/regression/save_stable_ids.test.ts
- Static rule: semgrep/rules/no-id-rewrite.yml
- Packet anti-pattern: "Do not regenerate identifiers during serialization or save"
- Policy gate: FAIL if serialization diff changes IDs without explicit migration flag
- Canary case: canary-suite/case-011/
```

---

## 20. Repo-specific canary suite

### 20.1 Objective

Use your own historical failure modes as a contamination-resistant benchmark.

### 20.2 Contents of each case

Each canary case should include:
- original task summary
- hidden tests and hidden properties
- required touched files or symbol set
- required `CR` / `RID` / `NRID`
- known anti-patterns
- expected workflow assertions
- acceptance threshold

### 20.3 Selection policy

Start with 20–40 cases drawn from:
- audit escapes
- post-merge regressions
- shallow passes that later failed deeper review
- state bugs
- import/export bugs
- undo/redo bugs
- serialization and migration bugs

### 20.4 When to run canaries

Run the canary suite whenever changing:
- system prompts
- work packet format
- reviewer prompt
- policy-gate rules
- context-pack generation
- model version
- repo structure affecting retrieval

---

## 21. Golden workflow harness

### 21.1 Objective

Creative tooling and IDE-like products fail in workflows and state transitions that ordinary unit tests do not capture well.

### 21.2 Candidate workflows

1. Create document -> edit visually -> undo/redo -> save -> reopen
2. Import asset -> transform -> export -> reimport
3. Toolbar action vs keyboard shortcut reaches the same command path
4. Code-side edit vs visual-side edit preserves the same underlying model
5. Duplicate, group, ungroup, delete, restore, then save/reopen
6. Crash/restart recovery restores the last safe state without corruption
7. Open large project -> edit -> save within latency budget
8. Enable/disable extension -> verify state cleanup and reload behavior
9. Multi-selection edits preserve ordering, IDs, and references
10. Failed operation rolls back partial state cleanly

### 21.3 Required assertions per workflow

- state invariants
- visual or structure snapshot
- undo/redo integrity
- serialization round-trip integrity
- latency budget
- resource ceiling where relevant

### 21.4 Example

```markdown
# GW-003 Toolbar vs Shortcut Parity

## Goal
Verify toolbar save and keyboard save hit the same command boundary and produce identical serialized output.

## Steps
1. Open fixture project `fixtures/basic-scene.project`
2. Modify object name and layer opacity
3. Trigger save via toolbar
4. Capture serialized output hash
5. Reload original fixture
6. Repeat edits
7. Trigger save via keyboard shortcut
8. Capture serialized output hash

## Assertions
- output hashes equal
- stable IDs unchanged
- undo stack cleared consistently
- save duration < 500 ms on reference machine
```

---

## 22. Boundary fuzzing and round-trip stress

### 22.1 Priority targets

Boundary fuzzing should focus on:
- project file parser
- importer / exporter
- plugin manifest loader
- theme or config parser
- command macro parser
- serialization / deserialization
- migration transforms
- clipboard / paste formats

### 22.2 Required invariants

- parse -> serialize -> parse yields equivalent structure
- canonical serialization remains canonical over repeated runs
- invalid input fails explicitly, not silently
- malformed input never corrupts shared state
- large but valid inputs stay within agreed resource budgets
- forbidden fields are rejected unless the spec explicitly allows otherwise

### 22.3 Promotion rule

Any boundary component with repeated audit findings becomes fuzz-required for Tier 1 changes.

---

## 23. Operating metrics

The governance system should be judged by escape reduction, not by paperwork volume.

### 23.1 Outcome metrics
- audit escape rate
- post-merge regression rate
- shallow-pass rate
- rework rate per accepted task
- canary pass rate
- workflow failure rate

### 23.2 Governance metrics
- unclaimed diff failures
- missing witness failures
- no-autonomy-zone violations
- prompt-variance failures
- average touched-file budget overrun
- repeated Mistake Book recurrence rate
- waiver count and renewal count

### 23.3 Test-strength metrics
- mutation score on Tier 1 modules
- property-test discovery rate
- state-machine discovery rate
- fuzz crash/rejection rate
- workflow-oracle discovery rate

### 23.4 Context metrics
- percentage of required files actually opened
- percentage of patches touching unopened files
- unused-context ratio
- required-tests-opened rate

### 23.5 Guardrail

If governance paperwork increases and the escape rate does not fall, the pipeline is being Goodharted.

---

## 24. Recommended implementation order

### Phase 1 — Immediate highest ROI
1. Create the canonical requirement graph with `CR`, `RID`, and `NRID`
2. Add diff claims
3. Add witness matrix
4. Move final `PASS` authority into the policy engine
5. Define no-autonomy zones
6. Add the waiver system

### Phase 2 — Cheap prevention and observability
7. Add retrieval telemetry
8. Add prompt-variance gating for the plan-only phase
9. Promote the top 10 audit findings into the prevention ladder
10. Add policy blockers for recurring high-severity failures

### Phase 3 — Product-specific depth
11. Build 10 golden workflows
12. Add boundary fuzzing for parser/import/export/serialization paths
13. Add impact-cone escalation
14. Build the first 20-case canary suite

### Phase 4 — Ongoing hardening
15. Review metrics weekly
16. Add new Mistake Book entries after each audit
17. Promote repeat offenders into static rules and canaries
18. Periodically shrink context packs if telemetry shows noise
19. Revisit blocker-vs-warning split if the pipeline becomes draggy

---

## 25. Short operating version

If only a minimal merged version is adopted now, adopt these first:

1. **Requirement graph with `CR` / `RID` / `NRID`**
2. **Diff claims + unclaimed diff gate**
3. **Witness matrix**
4. **Deterministic policy engine**
5. **No-autonomy zones**
6. **Waiver system**
7. **Mistake Book -> Prevention Ladder**

Those controls give the largest anti-gaming gain per unit of complexity.

---

## 26. Final operating principle

The operating model should be:

- no unclaimed code
- no unwitnessed requirement
- no `PASS` from models
- no protected-zone edits without human review
- no exception without an explicit, expiring waiver
- no recurring escape without promotion into prevention

The verification stack provides the machinery.

The governance law decides when the machinery is sufficient.

The policy engine, not the prose, decides whether the work is accepted.

---

## 27. Handshake repo-specific implementation delta

This paper is intentionally general. The Handshake repo needed a repo-local implementation layer because the observed failure mode was not only weak proof, but weak workflow truth across packet, runtime, task board, session, communication, and worktree state.

The practical result is that Handshake did not just "adopt the paper." It implemented a governance kernel shaped for orchestrator-managed parallel work with multiple governed roles, external runtime ledgers, packet-scoped direct review, and worktree-level authority.

### 27.1 What Handshake implemented from this paper

#### 27.1.1 Startup truth as a hard gate

Handshake implemented a startup/workflow-readiness hard gate rather than trusting optimistic startup narration.

Technical shape:
- shared readiness evaluation across packet state, task-board state, runtime state, session state, communication state, and worktree state
- folder-aware packet inventory so live packet layouts are not skipped by repo-wide checks
- stale or obsolete governed sessions demoted instead of advertised as resumable truth
- startup blocked when authority roots, worktree targets, or communication state disagree materially

Reason:
- the audit escape was not only a proof problem; it was also a false-ready problem
- a repo doing parallel governed work cannot tolerate split truth at startup because every later gate inherits that lie

#### 27.1.2 Transactional activation instead of partial activation

Handshake implemented transactional `prepare -> packet -> status sync` behavior for orchestrator-managed activation.

Technical shape:
- staged write / snapshot / restore flow across packet state, task board, traceability, build order, and communication bootstrap
- `orchestrator-prepare-and-packet` treated as one coordinated state transition rather than a sequence of loosely coupled side effects
- rollback on downstream failure instead of leaving fake progress behind

Reason:
- partial activation creates governance debt immediately
- if one surface says "active" while another still says "not started," the Orchestrator becomes a manual repair system

#### 27.1.3 Direct review as a governed boundary

Handshake implemented direct coder <-> validator review as a machine-checked workflow boundary.

Technical shape:
- packet-scoped receipts, notifications, runtime route projection, and communication health checks
- required `correlation_id`, `ack_for`, and `target_session` fields for direct-review receipts
- `next_expected_actor`, waiting state, and unread boundary notifications derived from governed artifacts
- handoff and verdict blocked when the required review pair is missing or malformed

Reason:
- narrative review claims were too easy to game
- orchestrator-managed parallel work needs direct-review proof that survives multiple simultaneous coder and validator sessions

#### 27.1.4 Computed closure instead of narrated closure

Handshake implemented a deterministic closure gate rather than trusting validator prose.

Technical shape:
- final closure computed from packet requirement/evidence structures, diff claims, witness coverage, protected-surface state, and waivers
- structured outcomes such as `PASS`, `FAIL`, `REVIEW_REQUIRED`, `WAIVED`, and `BLOCKED`
- pre-threshold structured legacy packets explicitly blocked as remediation-required instead of silently skipped

Reason:
- the main audit finding was that visible completion could outrun defended completion
- once closure is computed, the model cannot create authority just by sounding confident

#### 27.1.5 Scope and anti-spill enforcement

Handshake implemented enforceable scope controls around edit breadth and broad tools.

Technical shape:
- touched-file budgets
- broad-tool allowlists
- post-work spill detection
- packet-claim enforcement for scope declarations

Reason:
- one way LLMs game governance is by satisfying the visible packet contract while editing too broadly or using tools that outrun the intended proof surface

#### 27.1.6 Prevention-ladder and legacy-cleanup promotion

Handshake implemented a repo-local mistake-book/prevention-ladder pattern instead of leaving audit escapes as prose only.

Technical shape:
- named escape classes
- compatibility shim ledger
- deprecation sunset planning
- post-run audit skeleton generation
- runtime placement checks and migration-path truth checks

Reason:
- repeated escape shapes should become governed assets, not institutional memory
- real repositories accumulate historical compatibility surfaces that can poison live authority unless explicitly bounded

### 27.2 What Handshake added that this paper did not cover

The paper describes the anti-gaming model, but Handshake had to add several implementation layers because the repo operates with governed LLM sessions, dedicated worktrees, and packet-scoped communication artifacts.

#### 27.2.1 Externalized live runtime

The paper does not define a repo/external-runtime split. Handshake does.

Technical shape:
- live session/control/communication runtime under `../gov_runtime/roles_shared/*`
- only narrow repo-local exceptions retained under `/.GOV/roles_shared/runtime/`
- runtime placement checks to stop live authority from drifting back into the governance kernel

Reason:
- the kernel should stay reviewable, versioned, and resistant to runtime poisoning
- live mutable runtime state should not masquerade as repo-governed source of truth

#### 27.2.2 ACP broker and governed session-control plane

The paper does not describe a brokered multi-session control plane. Handshake needed one.

Technical shape:
- governed session registry
- governed launch/control request and output ledgers
- ACP broker status / stop / control surfaces
- role-bound startup prompts and governed resume commands
- worktree-aware session truth and stale-session demotion

Reason:
- Handshake runs multiple coders and validators in parallel
- the control plane itself therefore becomes a governance subject, not just a convenience layer

#### 27.2.3 Packet-scoped communication subsystem

The paper says direct review should be machine-checkable, but it does not specify a concrete communication substrate. Handshake built one.

Technical shape:
- per-packet thread, receipt, notification, and runtime-status artifacts
- per-WP communication transaction locking
- route projection from governed receipts
- session-aware notification cursors
- operator-facing communication health views

Reason:
- in a parallel packet system, direct review is not one message; it is a bounded governed conversation with routing, unread state, and replay/audit value

#### 27.2.4 Session identity as part of proof

The paper talks about roles; Handshake had to go further and govern sessions.

Technical shape:
- receipt pairing and notification acknowledgment keyed by role plus session identity where direct review is session-targeted
- mixed-session receipt chains treated as invalid
- unread state and route health computed at the session level, not only the role level

Reason:
- role-only pairing is not strong enough once there can be multiple coders or multiple WP validators active at once

#### 27.2.5 Packet-layout migration handling

The paper does not discuss packet-file layout migration. Handshake had to govern it.

Technical shape:
- shared packet inventory that understands flat historical packets and folder packets
- schema and checker alignment during migration
- explicit handling for legacy packet versions instead of blind compatibility

Reason:
- otherwise repo-wide checks silently skip the active packet shape and report a false green

#### 27.2.6 Command-surface coherence as governance

The paper does not cover protocol-doc drift versus executable command drift. Handshake had to.

Technical shape:
- restored `just` command surfaces for coder, validator, and orchestrator workflows
- command-surface tests that compare documented commands against live recipes
- health sweeps that treat dead wrapper commands as governance defects

Reason:
- in this repo, a stale command is not only documentation debt; it is an execution hazard
- if protocols name commands that do not exist, operator and role behavior diverges from law

### 27.3 Useful Handshake governance that already existed before this refactor

This refactor was able to land quickly because the repo already had several governance assets worth keeping.

#### 27.3.1 Split completion layers and `NOT_PROVEN`

Handshake already had the right intuition that completion is layered and that "not proven" is not the same as "passed."

Technical value:
- split verdict fields created a place to attach computed closure later
- `NOT_PROVEN` created a native home for partial defense instead of forcing false binary PASS/FAIL prose

#### 27.3.2 Structured task packets

The repo already had a strong task-packet culture.

Technical value:
- declared workflow state
- structured claims and evidence sections
- clause and semantic-proof scaffolding
- traceability hooks

This mattered because the refactor could strengthen packet law rather than invent a new artifact family.

#### 27.3.3 Dedicated role protocols and worktree discipline

Handshake already treated Orchestrator, Coder, and Validator as governed roles instead of generic prompts.

Technical value:
- role-local protocols
- dedicated worktree conventions
- explicit startup/resume habits
- PREPARE and packet authority paths

This made it possible to turn existing workflow conventions into executable checks.

#### 27.3.4 Validator gate sequence

The repo already had a stronger-than-average validator gate flow around append / commit / present / acknowledge.

Technical value:
- there was already a location where merge authority could be tightened
- the refactor therefore extended an existing gate instead of introducing a separate approval system

#### 27.3.5 Existing audit culture

Handshake already had audits, audit triggers, and a willingness to revisit "validated" work after deeper review.

Technical value:
- the repo was already generating the evidence needed to seed prevention assets
- the governance refactor could therefore be grounded in concrete failure modes rather than abstract fear

### 27.4 Repo-specific operating principle

For Handshake specifically, the practical operating principle is not only:

- no unclaimed code
- no unwitnessed requirement
- no PASS from models

It is also:

- no split workflow truth
- no session-blind review proof in parallel work
- no live runtime authority hiding inside the governance kernel
- no dead command surface in a role protocol
- no historical packet treated as healthy simply because it once validated

The paper supplies the model.

The repo-specific implementation supplies the control plane, authority boundaries, runtime law, and migration law required to make that model survive real orchestrated LLM work.
