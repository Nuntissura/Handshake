# Harness Lessons Learned — ACP / Repo Governance Experience

## Purpose

What we learned the hard way building the Handshake repo governance harness with ACP.
This is experiential knowledge — the stuff no survey or framework README will tell you.

Every entry follows the same structure:
1. **What happened** — the observable problem
2. **Root cause** — the actual technical reason
3. **What we tried** — the fix attempts (including what didn't work)
4. **What we learned** — the generalizable lesson
5. **Harness implication** — what this means for the next-generation harness design

### Relationship to other documents

| Document | Role |
|---|---|
| `Multi Agent Architectures.md` | Survey / catalog — breadth index of 60+ systems |
| `Technical_Implementation_Research.md` | Implementation depth — HOW the best systems actually work |
| `Harness_Lessons_Learned.md` (this file) | Our own experience — what broke and why |
| Architecture Synthesis (future) | Combined output — architecture decisions with evidence |

---

## How This Document Should Grow

This document should become the failure-backed technical memory of the current gov kernel.
It should not grow as a flat wall of prose, and it should not document every script with equal weight.
It should go deeper where failure density, runtime cost, or coordination complexity is highest.

Suggested growth layers:

1. **Lesson index**
   - Short statements of what failed and what the harness must do differently.
2. **Evidence-backed lesson entries**
   - Each lesson tied to concrete WPs, dossier findings, patch artifacts, gate logs, or broker traces.
3. **Subsystem deep dives**
   - ACP broker
   - session control
   - workflow state / packet truth
   - validator routing and review loops
   - dossier generation and live audit upkeep
   - terminal/session lifecycle
4. **Comparative research hooks**
   - For each lesson, map which external harnesses solve it well, partially, or not at all.
5. **Design consequences**
   - Turn lessons into architecture requirements, kill criteria, and migration steps for the next harness.

### What This Document Is Not

- Not a README clone
- Not a neutral catalog of scripts
- Not a narrative diary of every session
- Not product-spec documentation

Its job is to explain where the current governance/control plane breaks, why it breaks, how expensive that breakage is, and what that implies for the next architecture.

### Evidence Standard

Every mature lesson should eventually cite at least one of:

- workflow dossier section or finding ID
- packet/gate output path
- patch artifact showing scope drift or recovery
- runtime ledger, receipt stream, or broker state evidence
- concrete operator intervention that was required

Each lesson should clearly separate:

- symptom
- root cause
- attempted remediations
- current workaround
- future-state requirement

### Spec Anchoring Rule

This file is not allowed to infer product or governance semantics from the current kernel alone.
The current Master Spec remains the authority for intended workflow meaning.
In practice, every mature lesson should distinguish four layers:

- spec intent
- current kernel implementation
- observed failure mode
- redesign implication

For the current workline, the most important spec anchors are:

- Task Board synchronization state, tracked Work Packet activation, ready-query results, and Micro-Task summary state are canonical planning-and-coordination backend artifacts; human-readable Task Board views are synchronized mirrors and must not become a second execution authority.
- Canonical JSON or JSONL collaboration records remain the only executable authority for workflow routing, validation, and readiness state; Markdown mirrors are readable projections.
- Work Packet records are authoritative execution contracts and must resolve by direct load when the packet id or binding is known.
- Mailbox handoff and announce-back traffic that changes what work means must carry structured handoff bundles and explicit transcription into authoritative artifacts.
- Advisory announce-back must not be mistaken for authoritative completion; provenance gaps, missing transcription, stale handoff bundles, or unresolved scope changes must block optimistic done badges.

That means this lessons file can document where the repo kernel currently burns time and tokens, but it must not quietly redefine what authority, completion, or closeout are supposed to mean.

### First-Pass Anchor Artifacts

The first deepening pass should mine these artifacts before expanding theory:

- `DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
  - Strong evidence for packet truth drift, command-surface drift, and governance recovery cost dominating a relatively small product diff.
- `WP-1-Calendar-Storage-v2-MISROUTED_MAIN_DIFF-20260413T123133Z.patch`
  - Concrete artifact for stale range / wrong-target diff evaluation.
- `WP-1-Calendar-Storage-v2-CANDIDATE_TARGET-066cc18d.patch`
  - Concrete artifact for corrected contained scope after recovery.
- `DOSSIER_20260414_DISTILLATION_WORKFLOW_DOSSIER.md`
  - Evidence that live dossiers are useful in principle, but high-friction if too much of the document remains manual placeholder upkeep.
- `DOSSIER_20260411_DEV_COMMAND_CENTER_CONTROL_PLANE_BACKEND_WORKFLOW_DOSSIER.md`
  - Early evidence of dossier structure existing before the runtime and governance surfaces were mature enough to keep it truthy with low operator overhead.

### Current Thesis From Observed Runs

Based on the existing workflow dossiers and recovery artifacts, the dominant problem is not product coding difficulty.
The dominant problem is control-plane instability and truth drift around the coding work.

This is an implementation-side thesis about the current repo kernel.
It is not a replacement for the spec-defined meaning of authority, handoff, provenance, or completion.

Current working thesis:

- workflow-state drift costs more than implementation drift
- packet truth / merge-base truth drift can fabricate false failures and out-of-scope diffs
- broker/session-control brittleness turns orchestration into babysitting
- governance overhead is large enough that model/provider choice becomes a control-plane concern, not just a cost concern
- live dossiers are useful only if large parts of them are mechanically populated; otherwise they become more governance labor
- a future swarm harness must support both autonomous orchestration and operator-relay/manual recovery without changing the underlying state model

### Research Output This Document Should Feed

This document should directly feed five downstream outputs:

1. **Repo-governance capability matrix**
   - Shared comparison frame for the current kernel and external harnesses.
2. **Gov kernel technical map**
   - Deep technical explanation of the current control plane and all critical scripts/protocols.
3. **Comparative harness matrix**
   - How other systems handle state, retries, cost, observability, and parallel coordination.
4. **Architecture synthesis**
   - Which parts of the current kernel are retained, replaced, or deleted.
5. **Migration plan**
   - The smallest sequence of changes that moves Handshake from repo-governance testbed to product-building harness.

### Seed Deep-Dive Documents

This lessons file is the synthesis spine. The first seeded deep dives are:

- `Repo_Governance_Capability_Matrix.md`
- `Repo_Governance_Failure_Taxonomy.md`
- `Kernel_to_Swarm_Gap_Map.md`
- `Gov_Kernel_Technical_Map.md`
- `ACP_Broker_and_Session_Control.md`
- `Workflow_State_Packet_Truth_and_Range_Drift.md`
- `Validator_Routing_Gates_and_Closeout_Repair.md`

Use this file to state the lesson and architecture consequence.
Use the capability matrix to keep current-kernel and external-harness comparison on one stable frame.
Use the deep dives to hold the subsystem mechanics, artifact paths, failure timelines, and future redesign requirements.

### Practical Authoring Rule

When adding depth, prefer this order:

1. failure-heavy control surfaces first
2. frequently used runtime paths second
3. supporting scripts and reference material last

That keeps the document aligned with the actual bottlenecks instead of expanding evenly in low-value areas.

---

## Category 1: Loop Control & Escalation

### LL-1: Repair loops without caps become infinite retry loops

**What happened:**
Agent repair loops (coder failing validation, retrying, failing again) could run indefinitely.
Each retry consumed tokens and time but often repeated the same failing approach.

**Root cause:**
No hard cap on retry iterations. No strategy escalation — the system just retried the same way.
The orchestrator's "repair" instruction was narrative ("try again, fix the issues") rather than
mechanical ("you have N attempts remaining; if this fails, escalate to X").

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Retry without strategy change is waste. After N identical failures, the approach is wrong, not the execution.
- Loop caps must be hard (enforced by the runtime, not requested in a prompt).
- Escalation must change something: different model, different approach, human intervention, or abort.
- The cap number should vary by task type — deterministic tasks should fail fast, heuristic tasks get more room.

**Harness implication:**
The harness needs:
- Per-MT iteration budget enforced at the runtime level, not in the prompt
- Strategy escalation tiers: retry → different approach → different model → human escalation → abort
- Budget tracking visible to the operator in real-time
- Different budgets for deterministic vs heuristic work (see LL-3)

---

### LL-2: Narrative orchestration drifts under pressure

**What happened:**
The orchestrator's behavior drifted when context windows got large or when sessions ran long.
It would skip steps, misinterpret protocol, invent commands, or lose track of where it was
in the workflow.

**Root cause:**
Orchestration was driven by natural language prompts, not by an explicit state machine.
The "current state" lived in the conversation context, not in a durable data structure.
As context grew, earlier instructions got compacted or deprioritized by the model.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Chat context is not state. It's a lossy, model-interpreted representation of state.
- State must live outside the conversation in a durable, machine-readable format.
- The orchestrator should read its next action from state, not derive it from conversation history.
- Mechanical state transitions (state machine) resist drift; narrative transitions invite it.
- Protocol commands must be exact strings, not interpreted instructions.

**Harness implication:**
The harness needs:
- Durable workflow state in a structured format (JSON/DB), not in conversation context
- State machine with explicit transitions, not prompt-driven decision-making
- Protocol commands as exact string matches, not natural language interpretation
- State visible to operator for debugging and intervention

---

## Category 2: Contracts & Constraints

### LL-3: Fuzzy tasks and deterministic tasks need different treatment

**What happened:**
The same validation rigor was applied to all microtasks regardless of complexity.
Deterministic tasks (add a field, write a migration) passed quickly.
Heuristic tasks (design a component API, refactor a module) failed repeatedly because
validation criteria were too rigid for inherently subjective work.

**Root cause:**
No task classification. All MTs were treated identically by the validation loop.
The validator applied the same pass/fail criteria to work that was fundamentally
different in nature.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Tasks have inherent risk/complexity profiles that should affect how they're handled.
- Deterministic tasks: tight contract, strict validation, fast fail, low iteration budget.
- Heuristic tasks: looser contract, graduated validation, more iterations, earlier human review.
- Classification should happen BEFORE the task enters the execution loop, not after it fails.
- The coder's scope must be constrained to match — a deterministic task should have no room for creative interpretation.

**Harness implication:**
The harness needs:
- Task risk/complexity classification at refinement time
- Different execution profiles per classification (iteration budget, validation strictness, escalation path)
- MT contracts that declare expected output shape, not just description
- Coder scope constraints that match the task classification

---

### LL-4: Weak MT contracts let coders drift

**What happened:**
Microtasks defined in natural language were interpreted differently by the coder
than intended by the orchestrator/refinement. The coder would over-build, under-build,
or build the wrong thing — all while technically "completing" the task description.

**Root cause:**
MT contracts were prose descriptions, not machine-readable specifications.
No typed expected output. No file-scope constraints. No pre-declared change surface.
The coder had full autonomy to interpret the task.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Natural language task descriptions are ambiguous by nature. Models exploit ambiguity (not maliciously — they just optimize for what seems right).
- Contracts must specify: what files can be touched, what the output shape is, what the acceptance criteria are (machine-checkable where possible).
- The tighter the contract, the less the coder can drift.
- But over-constraining heuristic tasks makes them impossible — see LL-3.

**Harness implication:**
The harness needs:
- Structured MT contract format with machine-readable fields
- File-scope declaration (which files/directories the MT may touch)
- Expected output schema where applicable
- Machine-checkable acceptance criteria alongside human-readable description
- Contract strictness that matches task classification (LL-3)

---

## Category 3: Session & Broker Reliability

### LL-5: ACP broker is brittle under host load

**What happened:**
Under normal development load (IDE, browser, other processes), the ACP broker
would intermittently fail to relay messages, lose session state, or time out.
Sessions that were working fine would silently stop responding.

**Root cause:**
- _Document the actual technical root cause once diagnosed_
- Likely factors: resource contention, no health checks, no circuit breakers,
  no backpressure mechanism, session state not durable

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- A broker that works under clean conditions but fails under load is not production-ready.
- Health checks and circuit breakers are not optional — they're the minimum for reliability.
- Session state must be durable (survive broker restart).
- The operator must know when a session is unhealthy (not discover it after wasted time).
- Backpressure is better than silent failure — slow down rather than drop messages.

**Harness implication:**
The harness needs:
- Broker health monitoring with operator-visible status
- Circuit breakers that prevent cascading failure
- Durable session state that survives broker restarts
- Backpressure mechanics (slow down, don't drop)
- Session health signals exposed to the operator in real-time

---

### LL-6: Operator has no alerting channel outside chat

**What happened:**
The only way to know something was wrong was to be watching the terminal.
Failed sessions, stuck loops, validation failures — all of these happened silently
unless the operator happened to be looking at the right window at the right time.

**Root cause:**
All system feedback flows through the chat interface. No push notification,
no webhook, no dashboard, no email, no sound. The operator must actively monitor.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Polling for problems is waste (feedback_polling_waste.md confirms this).
- An operator who must watch terminals is not an operator — they're a babysitter.
- Alerting must be push-based, not pull-based.
- Different severities need different channels (terminal for info, notification for warning, interrupt for critical).

**Harness implication:**
The harness needs:
- Push-based alerting outside the chat interface
- Severity-based routing (info → log, warning → notification, critical → interrupt)
- Alert aggregation (not one notification per micro-event)
- Integration points for external alerting (webhooks, system notifications, sound)
- Status dashboard that can be checked asynchronously

---

## Category 4: Workflow State & Orchestration

### LL-7: Workflow-state drift is the real bottleneck

**What happened:**
WP execution was slow not because of document drift or coding speed, but because
the orchestrator would lose track of workflow state. It would repeat steps, skip steps,
misroute sessions, or fail to close out completed work.

**Root cause:**
Workflow state (which MT is active, which are done, what phase the WP is in) lived
in the orchestrator's conversation context, not in a durable state store.
As the session progressed, the model's "understanding" of state diverged from reality.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- The governance cost bottleneck is workflow-state drift, not document drift.
- Routing decisions, verdict handling, and closeout loops are where time is wasted.
- The orchestrator must read current state from a durable store, not reconstruct it from memory.
- State transitions must be logged and auditable so drift can be detected.

**Harness implication:**
The harness needs:
- Durable workflow state store (not conversation context)
- State read at the start of every orchestrator action (not cached, not remembered)
- Transition logging for drift detection
- Operator-visible state timeline for debugging

---

### LL-8: Terminal hygiene reflects runtime discipline

**What happened:**
Governed terminal windows accumulated — blank, stale, completed sessions left open.
This made it hard to see what was actually running and wasted operator attention.

**Root cause:**
No session cleanup protocol. Sessions were spawned but never systematically
reclaimed after completion, failure, or abandonment.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Terminal window state is a proxy for session lifecycle management quality.
- If your runtime can't clean up terminals, it can't clean up sessions.
- Every session must have a defined end state that triggers cleanup.
- This becomes even more important in the product (DCC session display).

**Harness implication:**
The harness needs:
- Session lifecycle with explicit terminal states (completed, failed, abandoned, timed-out)
- Automatic cleanup on terminal state
- Session state visible to operator (training for DCC in the product)
- No orphaned resources after session completion

---

## Category 5: Governance Process

### LL-9: Smoketest reviews must be live observations, not delegated summaries

**What happened:**
Smoketest reviews delegated to subagents that hadn't observed the actual WP run
produced shallow, generic assessments that missed real issues.

**Root cause:**
The reviewing agent had no first-hand observation of the run. It was summarizing
artifacts without context about what actually happened during execution.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Observation during the run is irreplaceable. Post-hoc review of artifacts alone misses dynamics.
- Roles must append findings during WP work, not compile them after.
- Never delegate full review to a subagent that didn't observe the run.
- This is an argument for durable event streams that a reviewer CAN replay.

**Harness implication:**
The harness needs:
- Event stream that captures enough for meaningful post-hoc review (if live observation isn't possible)
- Review assignments that include access to the full execution trace
- Roles that accumulate observations during execution, not just at review time

---

### LL-10: Refinement regresses under mechanical workflow pressure

**What happened:**
Under the ACP workflow, refinement sessions became rote — checking boxes instead of
actively discovering primitives, mixing features, and creating stubs. The creative
aspect of refinement was lost.

**Root cause:**
The mechanical ACP workflow optimized for throughput and protocol compliance.
Refinement was treated as a form to fill out, not as active feature discovery.
The orchestrator routed refinement like any other step — mechanically — when it
needed space for creative exploration.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Refinement is fundamentally different from execution. It needs room for exploration.
- Mechanical orchestration works for execution but kills creative steps.
- The harness must distinguish between "execute this plan" and "explore and discover."
- Risk classification (LL-3) applies here too — refinement is inherently heuristic.

**Harness implication:**
The harness needs:
- Different execution modes for creative vs mechanical work
- Refinement given explicit exploratory scope (not just "fill in the template")
- Discovery outputs (new primitives found, force multipliers identified, stubs created) as first-class refinement artifacts
- Orchestrator awareness that refinement is a different kind of step

---

## Category 6: Provider & Model Management

### LL-11: Multi-provider allocation requires active cost management

**What happened:**
GPT 5.4 budget burn revealed that cost management can't be an afterthought.
Different providers have radically different cost profiles, and token-heavy
governance workflows amplify cost differences.

**Root cause:**
No cost-aware routing. All work went to the most capable (and expensive) model
regardless of task complexity. Governance overhead (protocol prompts, state summaries,
validation loops) multiplied base costs.

**What we tried:**
- _Document what was tried and what worked/didn't_

**What we learned:**
- Cost is a routing dimension, not just a reporting metric.
- Simple/deterministic tasks can use cheaper/local models without quality loss.
- Governance token overhead is a multiplier — reducing it has outsized cost impact.
- But: don't block on token budgets during execution (token_budget_lenient.md). Log, don't gate.

**Harness implication:**
The harness needs:
- Cost-aware model routing (task complexity → model tier)
- Governance token budget tracking (what fraction of tokens is governance overhead vs actual work?)
- Provider allocation strategy that balances cost, capability, and reliability
- Cost logging granular enough to optimize (per-MT, per-role, per-provider)

---

## Lessons → Pain Points Cross-Reference

| Lesson | PP-1 Loop caps | PP-2 Contracts | PP-3 Risk class. | PP-4 Alerting | PP-5 Reliability | PP-6 Mech. transitions |
|---|---|---|---|---|---|---|
| LL-1 Infinite repair loops | **PRIMARY** | supports | supports | | | supports |
| LL-2 Narrative drift | | | | | | **PRIMARY** |
| LL-3 Fuzzy vs deterministic | supports | supports | **PRIMARY** | | | |
| LL-4 Weak MT contracts | | **PRIMARY** | supports | | | |
| LL-5 Broker brittleness | | | | supports | **PRIMARY** | |
| LL-6 No external alerting | | | | **PRIMARY** | | |
| LL-7 Workflow-state drift | supports | | | | | **PRIMARY** |
| LL-8 Terminal hygiene | | | | | supports | supports |
| LL-9 Smoketest delegation | | | | | | |
| LL-10 Refinement regression | | | supports | | | |
| LL-11 Multi-provider cost | | | supports | | | |

---

## Open Questions

These are questions raised by our experience that the technical research (Layer 2) should answer:

1. **How do production systems actually implement loop caps?** Is it always a hard count, or are there smarter heuristics (detecting repeated patterns, cost threshold, time threshold)?

2. **Is anyone doing pre-execution task risk classification?** Or is this a gap in the field?

3. **What's the state of the art for durable workflow state in agent systems?** Is LangGraph's checkpoint model the best available, or are there better approaches?

4. **How do systems that use explicit state machines handle the creative/heuristic tasks?** Does mechanical orchestration always kill exploration, or are there designs that handle both?

5. **What alerting patterns exist outside the IDE/terminal?** Webhooks? System notifications? Dashboard-first designs?

6. **How do multi-agent systems handle cost attribution?** Can you track cost per task, per role, per provider at the granularity needed for optimization?

7. **What is the minimum viable operator console?** Not the full DCC vision — what do you need on day 1 to stop babysitting terminals?

8. **How do systems handle the "session that silently died" problem?** Heartbeats? Health checks? What's the detection latency?
