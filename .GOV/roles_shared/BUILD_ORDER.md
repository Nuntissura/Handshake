# Build Order (Advisory) [CX-BO-001]

This file provides a RECOMMENDED build order for Phase 1 Work Packets (WPs).

- This build order is NOT binding: the Master Spec + active task packets define "Done".
- File upkeep IS binding: the Orchestrator MUST keep this file current (see Orchestrator Protocol).

## Source of truth (do not contradict)

- Master Spec: `.GOV/roles_shared/SPEC_CURRENT.md` (resolves to a versioned `Handshake_Master_Spec_vXX.XXX.md`)
- Task Board status: `.GOV/roles_shared/TASK_BOARD.md`
- Base->Active packet mapping: `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- Exact scope/acceptance: the active task packet under `.GOV/task_packets/`

## Update triggers (Orchestrator MUST update)

Update this file when any of the following occurs:
- A stub becomes activated (new official packet created) or a new `-vN` revision becomes the active packet.
- A new blocking dependency is discovered that changes the recommended sequencing.
- SPEC_CURRENT changes in a way that adds/removes/reshapes Phase 1 deliverables.
- A WP becomes "blocking" for a target outcome (e.g., IDE replacement, parallel cloud sessions).

## How to use this file

1. Use this as the default sequencing suggestion when creating/activating WPs.
2. Record concrete dependency edges in:
   - the task packet `## Dependencies`, and
   - `.GOV/roles_shared/TASK_BOARD.md` (blocker lines).
3. If this file and TASK_BOARD disagree, reconcile them in the same governance maintenance pass.

## Recommended sequencing (Phase 1)

### 0) Immediate-value deliveries (can run while foundations are being refined)

- WP-1-Media-Downloader (VALIDATED: WP-1-Media-Downloader-v2)
  - Rationale: urgent family salvage value; exercises engine.job queue, tools, and OutputRootDir materialization.

### 0.5) Prompt->Spec hardening quartet prerequisites (v02.139)

Run these before broader Spec Router expansion so routing stays deterministic and policy-safe:

1. WP-1-Spec-Router-SpecPromptCompiler (active packet: WP-1-Spec-Router-SpecPromptCompiler-v1)
2. WP-1-Spec-Router-CapabilitySnapshot (stub exists)
3. WP-1-Spec-Router-SpecLint (stub exists)

### 0.6) Spec Router session logging (after 0.5)

- WP-1-Spec-Router-Session-Log (stub exists)
  - Rationale: Phase 1 goal requires Spec Router to create Task Board + Work Packet session logs; schedule after prompt determinism hardening so logs capture the required provenance fields.

### 1) Safety baseline for "Handshake as IDE"

MEX remediation is a practical prerequisite for scaling autonomous work safely (gates, observability, UX bridges):

1. WP-1-MEX-Safety-Gates (remediation packet: `...-v2` stub exists)
2. WP-1-MEX-Observability (remediation packet: `...-v2` stub exists)
3. WP-1-MEX-UX-Bridges (remediation packet: `...-v2` stub exists)

### 1.5) Phase 1 MVP workspace surfaces (Atelier + Photo Studio)

These are Phase 1 MVP product surfaces that should not be deferred past the core governance/tooling foundations:

1. WP-1-Atelier-Lens (remediation packet: `...-v2` stub exists; always-on Lens extraction runtime)
2. WP-1-Photo-Studio (remediation packet: `...-v2` stub exists; Photo Studio skeleton surface)

### 2) Model routing + governance prerequisites (before parallel cloud sessions)

These enable "who can call what model/provider, under which policy" with auditability:

1. WP-1-Model-Profiles (remediation packet: `...-v2` stub exists)
2. WP-1-Work-Profiles (stub exists)
3. WP-1-Cloud-Escalation-Consent (VALIDATED: WP-1-Cloud-Escalation-Consent-v2; required for governed cloud fan-out)
4. WP-1-Inbox-Role-Mailbox-Alignment (stub exists; coordination fabric)

### 3) Parallelism foundations (runtime enforcement)

- WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry (stub exists)
  - Rationale: file-scope locks, lifecycle telemetry, execution identity, deterministic blocking codes.

### 4) "Control room" surfaces (IDE replacement path)

These are the planned Operator-facing coordination surfaces:

1. WP-1-Locus-Work-Tracking-System-Phase1 (stub exists)
2. WP-1-Dev-Command-Center-MVP (stub exists)

### 5) Web capture + media browsing/search (after ingestion + control surfaces)

1. WP-1-Handshake-Stage-MVP (stub exists)
2. WP-1-Loom-MVP (VALIDATED: WP-1-Loom-MVP-v1)
3. WP-1-Video-Archive-Loom-Integration (stub exists)

## Notes / constraints

- Concurrency: do not run overlapping WPs that touch the same IN_SCOPE_PATHS; use separate worktrees per WP.
- Governance: cloud use requires explicit consent artifacts; do not "parallelize" by bypassing receipts/policy.
- This file should stay compact; details belong in task packets and TASK_BOARD blocker lines.
