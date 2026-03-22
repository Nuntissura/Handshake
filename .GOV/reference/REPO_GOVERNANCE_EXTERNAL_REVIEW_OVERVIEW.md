# Repository Governance External Review Overview

This document is a descriptive overview for external review.

It explains how the repository governance system currently works, what problems it is designed to solve, how work moves through the system, and how authority is divided across roles.

It is intentionally non-authoritative. The live workflow rules remain in the codex, role protocols, active work packets, and governed checks.

## 1. Purpose

The governance system exists to make implementation against a master specification more trustworthy.

It is designed around a hard lesson common to AI-assisted software work: visible progress, passing tests, or polished handoff notes do not reliably prove that a system is correctly implemented. A workflow that only rewards motion, narratives, or happy-path results will eventually accept shallow implementation, drift, or false closure.

This governance therefore acts as a control plane.

Its purpose is to:
- define a formal contract for each unit of work
- separate workflow authority from technical authority
- force evidence-based handoff instead of informal claims
- keep multiple parallel work streams coordinated without silent scope collisions
- preserve uncertainty honestly when proof is incomplete
- create an audit trail that can later be reviewed against the master specification

The governance is also intentionally treated as a prototype of the future Handshake product control plane. In other words, governance defects are not seen merely as process defects. They are treated as early product-grade control failures.

## 2. What The Governance Consists Of

The governance model has several layers.

### 2.1 Product Truth

At the top is the master specification.

This is the source of product intent, architecture, required behavior, and compatibility obligations.

### 2.2 Governance Law

The next layer is governance law.

This consists primarily of:
- a repository codex
- role-specific protocols
- shared workflow and session orchestration rules

This layer defines how work may be performed, who is allowed to do what, what evidence is required, and how conflicts between workflow surfaces are resolved.

### 2.3 Packet-Level Contracts

Every substantial implementation task is expressed as a Work Packet.

A Work Packet is the operational contract for one bounded unit of work. It records:
- scope
- authority
- expected outcomes
- required tests
- risk tier
- worktree and branch assignment
- communication surfaces
- validation expectations

### 2.4 Refinement

Before a packet becomes execution-ready, it is expected to undergo refinement.

The refinement step turns a work request into a more technically grounded packet by clarifying:
- architectural choices
- governing spec anchors
- important risks
- unresolved uncertainty
- constraints that must be enforced during implementation and validation

### 2.5 Shared Governance Records

The system also maintains shared records such as:
- a task board for high-level visibility
- a traceability registry that maps a stable base work item to the currently active packet version
- build-order and dependency projections
- debt and closure tracking surfaces

These are projections and coordination aids. They are important, but they do not outrank the active packet.

### 2.6 Runtime Coordination Artifacts

The governance also uses live runtime artifacts for machine-readable coordination.

These include:
- session launch queues
- a session registry
- control request and control result ledgers
- packet-scoped communication folders containing a thread, runtime state, and structured receipts

These artifacts do not define product truth. They define operational state and coordination evidence.

## 3. Core Design Principles

Several principles define how the system behaves.

### 3.1 Packet Truth Wins

The active Work Packet is the authoritative contract for a work item.

If the packet disagrees with projections, runtime state, or informal discussion, the packet wins and the inconsistency must be repaired.

### 3.2 Workflow Authority And Technical Authority Are Separated

The role that coordinates workflow is not the same role that owns final technical verdict.

This prevents the system from collapsing into a single role that both pushes work forward and judges whether the work is actually good enough.

### 3.3 Evidence Over Narrative

The system is intentionally skeptical of chat summaries and self-reported completion.

It prefers:
- file-backed evidence
- machine-checked gates
- explicit handoff receipts
- repeatable commands
- explicit non-pass states when proof is incomplete

### 3.4 Honest Non-Pass States

The governance treats false `PASS` as a serious failure.

If evidence is incomplete, the workflow should preserve outcomes such as:
- not proven
- blocked
- partial
- outdated only

This is intended to stop the system from translating uncertainty into closure.

### 3.5 Parallel Work Requires Explicit Scope Boundaries

Parallel work is allowed, but only under explicit packet scope.

The workflow does not assume that multiple active work items can safely coexist unless scope, worktree, and authority boundaries are explicit and machine-checkable.

## 4. Repository Governance Structure

The repository is structured so governance, product implementation, and live runtime state are clearly separated.

### 4.1 Product Areas

The product side contains:
- the application shell and frontend
- backend core logic
- tests
- shared types and contracts where needed

These are the surfaces where product implementation lives.

### 4.2 Governance Areas

The governance side contains:
- role-owned protocol and tooling bundles
- shared governance records, checks, scripts, and schemas
- work packets
- refinements
- templates
- audits
- operator-private material
- governed tools such as session bridges and coordination utilities

### 4.3 Governance Kernel Model

Governance is maintained through a single live governance kernel rather than being independently copied and edited in every worktree.

This means:
- governance is centralized
- role worktrees do not diverge in governance law
- governance updates are live across active worktrees
- governance is committed separately from product feature branches

### 4.4 External Runtime Model

Live runtime coordination artifacts are intentionally stored outside ordinary product source areas.

This includes:
- session launch ledgers
- session registries
- broker and control artifacts
- packet-scoped communication folders

This separation exists to keep runtime coordination state from being confused with version-controlled implementation truth.

### 4.5 Orchestrator-Managed Workflow And ACP

The governance supports more than one workflow lane, but its most structured lane is the Orchestrator-managed workflow.

In that lane, the Orchestrator does not merely prepare packets and wait. It actively governs session startup, session routing, and workflow progression across the packet lifecycle.

The live session model has two parts.

First, there is a governed launch layer.

This is responsible for:
- starting packet-scoped role sessions
- recording launch attempts
- projecting current session state into a shared registry
- making startup behavior observable instead of implicit

Second, there is a governed control layer built around an ACP-style thread model.

In practical terms, this means the system treats a governed role session as a resumable controlled thread rather than a disposable terminal prompt. The Orchestrator can:
- start a session
- send a governed prompt into that session
- cancel a governed run
- close the session when it is no longer needed

Those actions are not treated as informal chat events. They are recorded as append-only control requests and control results, with per-command event logs for deeper inspection.

This matters because the workflow is trying to make orchestration machine-readable.

Instead of relying on:
- someone remembering which terminal is alive
- informal chat saying "the validator has been told"
- ad hoc relaying between roles

the system tries to record:
- which governed role session exists
- which packet it belongs to
- whether it was launched successfully
- what command is active
- whether it completed, failed, or was cancelled

The launch bridge and the ACP-style control lane do different jobs.

The launch bridge is mainly a bootstrap transport. It gets the governed session started in a host environment.

The ACP-style control lane is the ongoing steering path. It is the part that allows the Orchestrator to continue interacting with an already-started governed session in a structured and resumable way.

This distinction is important because terminal dispatch by itself is not treated as proof that useful governed work is happening. The stronger evidence is:
- governed control state
- packet-scoped receipts
- packet runtime-state movement
- actual packet-scoped communication activity

The packet still remains the authority for work truth.

The Orchestrator-managed ACP/session system does not replace the packet. It does not decide scope, acceptance, or verdict. Instead, it provides the transport and control layer that allows the workflow to coordinate multiple packet-scoped role sessions without turning the Orchestrator into an informal human message relay.

The intended effect is:
- startup is governed
- steering is governed
- wake-ups are governed
- session state is inspectable
- workflow routing becomes more automatic

At the same time, packet-scoped communication artifacts remain the collaboration authority for the work item itself. The Orchestrator-managed control lane is therefore best understood as workflow transport and control, not as the source of technical truth.

Because infrastructure can fail, the governance also defines a fallback rule.

The normal path is plugin-first governed startup. If plugin or bridge instability crosses a defined threshold, the workflow can switch to an explicit command-line escalation mode. More recently, that logic has been strengthened so repeated instability can switch the whole governed batch into CLI escalation mode rather than rediscovering the same launch failure packet by packet.

For external review, the most important point is this:

the Orchestrator-managed workflow and ACP-style control lane are how this governance turns multi-session AI work from an informal terminal habit into a governed, inspectable, replayable coordination system.

## 5. Roles And Their Responsibilities

The governance uses a multi-role model with explicit authority boundaries.

### 5.1 Operator

The Operator is the human authority.

The Operator provides:
- major approvals
- signature authorization
- cleanup authorization
- escalation decisions
- governance direction when the system encounters ambiguity or conflict

The Operator is not expected to be the technical implementer.

### 5.2 Orchestrator

The Orchestrator is the workflow authority.

Its job is to:
- define and refine work packets
- assign execution ownership
- record the packet’s execution setup
- maintain workflow coherence across packet state, task board state, runtime state, session state, and worktree state
- launch governed sessions
- keep the workflow moving without silently changing technical truth

The Orchestrator is intentionally non-agentic as a role. It coordinates, but it is not meant to become a general-purpose autonomous swarm leader that absorbs everyone else’s duties.

### 5.3 Coder

The Coder is the implementation role.

Its job is to:
- work only within approved packet scope
- honor the packet’s acceptance contract
- run pre-work and post-work gates
- produce evidence, not just code
- hand off honestly to validation
- surface weak spots instead of hiding them

The Coder does not own final merge authority.

### 5.4 WP Validator

The WP Validator is the packet-scoped advisory reviewer.

Its job is to:
- review the coder’s change against packet scope and spec anchors
- challenge weak claims and unclear proof
- provide technical criticism early enough to change direction when necessary

This role is advisory, not final.

### 5.5 Integration Validator

The Integration Validator is the final technical authority.

Its job is to:
- independently review the implementation
- determine whether the work is actually proven
- own the final verdict
- own merge-to-main authority
- ensure the integrated state is the real validation target when closure matters

This role exists so final technical closure is not delegated back to the same lane that created the work.

## 6. The Single Work Packet Workflow

The life of one Work Packet follows a structured sequence.

### 6.1 Work Definition

The work starts when the Orchestrator defines or updates a Work Packet.

This packet establishes:
- the work item identity
- current status
- scope
- expected result
- risk
- execution owner
- review and communication expectations

### 6.2 Refinement

If the work is non-trivial, a refinement step clarifies technical direction before implementation begins.

This reduces ambiguity and creates a more realistic contract for implementation and validation.

### 6.3 Signature And Activation

The packet is activated through a signature bundle.

This records:
- user approval
- workflow lane
- execution owner

At this point the packet becomes the live governed contract for the work.

### 6.4 Prepare

The Orchestrator then records the execution setup for the packet.

This includes:
- the assigned feature branch
- the assigned worktree
- the execution owner
- the runtime coordination expectations

This step is important because the system treats split truth as a real failure. Packet, worktree, task board, and runtime state are expected to align.

### 6.5 Session Launch

If governed role sessions are needed, the Orchestrator starts them.

Fresh role sessions are not self-started by Coders or Validators.

The primary launch path uses a governed session bridge and session registry. If that primary path becomes unreliable, the system can escalate to an explicit command-line session mode under governed rules.

### 6.6 Pre-Work Gate

Before implementation begins, the Coder must pass a pre-work gate.

This verifies that:
- the packet exists
- the packet is structurally valid
- the current worktree and branch are correct
- the current scope and execution context are not ambiguous

### 6.7 Skeleton And Implementation

The workflow expects an early skeleton or checkpoint step before broad coding continues.

The point of this is to make the implementation shape visible early and prevent large hidden coding moves before the work has been checked for structural sanity.

Then the Coder implements only within the packet’s approved scope.

### 6.8 Packet-Scoped Communication

During the work, the packet-scoped communication artifacts are used for:
- freeform discussion
- liveness
- structured receipts for handoff and review

The workflow increasingly depends on machine-visible direct exchange rather than informal chat relay.

### 6.9 Post-Work Gate And Handoff

When the Coder believes the work is ready, a post-work gate is run.

This is meant to check closure evidence, not just code existence.

The handoff is then made to validation, along with explicit proof surfaces and weak points.

### 6.10 Advisory Review

The WP Validator reviews the packet-scoped work.

This role is expected to question both implementation and proof quality.

### 6.11 Final Validation

The Integration Validator performs the independent final review.

At this point, the system is meant to distinguish between:
- workflow being complete
- scope being respected
- proof actually being sufficient
- integration being ready

Only then can final closure or merge proceed.

## 7. The Parallel Work Packet Workflow

Parallel execution uses the same packet logic, but adds stronger isolation and coordination requirements.

### 7.1 One Packet, One Branch, One Coder Worktree

Each active packet gets:
- one feature branch
- one dedicated coder worktree

This prevents active coders from contaminating each other’s working state.

### 7.2 Shared Governance, Separate Product Work

Governance remains centralized and live through the governance kernel.

Product execution remains separated through packet-specific branches and worktrees.

This means:
- governance is shared
- implementation state is isolated

### 7.3 Packet-Scoped Communication Per Work Item

Every active packet has its own communication surfaces.

This gives each parallel work item its own:
- thread
- runtime status
- receipt ledger

That keeps discussion and workflow receipts from collapsing into an unreadable shared stream.

### 7.4 Orchestrator Coordination Across The Batch

The Orchestrator remains the workflow authority across all active packets.

It is responsible for:
- launching sessions
- routing wake-ups
- maintaining packet truth
- preventing split authority across the batch

### 7.5 Validator Layering In Parallel

Packet-scoped validators may operate in parallel.

Final technical closure is still intentionally centralized through Integration Validation so that the batch does not devolve into many local “passes” with no integrated technical authority.

### 7.6 Batch-Level Runtime Controls

Because parallel execution depends on governed session infrastructure, the workflow also tracks batch-level runtime conditions.

If the launch/bridge path becomes unstable enough, the batch can switch to a governed CLI escalation mode instead of retrying unstable startup paths packet by packet.

## 8. Truth Hierarchy

The governance uses an explicit truth hierarchy because many workflow failures come from competing surfaces claiming to be authoritative.

The hierarchy is:
- master specification for product truth
- codex and role protocols for governance law
- active Work Packet for work-item contract truth
- task board and traceability records as projections
- runtime artifacts as operational evidence
- freeform chat as lowest-confidence context unless reflected into governed surfaces

This hierarchy exists so the system can resolve drift rather than narrate around it.

## 9. Why This Governance Exists

The overall intention is to produce a workflow where:
- shallow implementation is harder to close
- formal proof matters more than momentum
- role boundaries create useful tension instead of redundancy
- parallel work does not automatically create chaos
- uncertainty remains visible instead of being rounded up into false confidence

The governance is not meant to guarantee perfect implementation.

It is meant to make false closure, vague ownership, and unproven correctness much harder to get through the system unnoticed.

## 10. Present Character Of The System

The current governance is best understood as a structured, evidence-oriented, multi-role repository control plane.

It is stronger than a simple task board and branch workflow because it includes:
- explicit work contracts
- refinement before execution
- layered role authority
- machine-readable runtime coordination
- packet-scoped review artifacts
- non-pass verdict preservation
- separation of workflow management from final technical closure

Its core bet is that implementation quality improves when the workflow stops rewarding appearance and starts rewarding proof.
