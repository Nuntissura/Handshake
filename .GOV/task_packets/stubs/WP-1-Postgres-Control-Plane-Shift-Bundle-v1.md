# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Postgres-Control-Plane-Shift-Bundle-v1

## STUB_METADATA
- WP_ID: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- BASE_WP_ID: WP-1-Postgres-Control-Plane-Shift-Bundle
- CREATED_AT: 2026-05-06T12:45:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Storage-Abstraction-Layer, WP-1-Storage-Trait-Purity, WP-1-Dual-Backend-Tests, WP-1-Migration-Framework, WP-1-ModelSession-Core-Scheduler, WP-1-Workflow-Engine, WP-1-Front-End-Memory-System, WP-1-Dev-Command-Center-Control-Plane-Backend
- BUILD_ORDER_BLOCKS: WP-1-FEMS-Bitemporal-Indexing, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-FEMS-Injection-Scoring-Graceful-Degradation, WP-1-FEMS-Outcome-Feedback-Loop, WP-1-FEMS-Pinned-Core-Memory, WP-1-Dev-Command-Center-MVP, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Workflow-Transition-Automation-Registry, WP-1-Workflow-Projection-Correlation
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT PostgreSQL-primary storage, ModelSession, FEMS, workflow durable execution, DCC runtime truth, and SQLite cache/offline boundary anchors
- FOLDS_STUBS:
  - WP-1-Postgres-Dev-Test-Container-Matrix-v1
  - WP-1-Postgres-Control-Plane-Leases-Backpressure-v1
  - WP-1-ModelSession-Postgres-Queue-Workers-v1
  - WP-1-FEMS-Postgres-Memory-Store-v1
  - WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
  - WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - WP-1-SQLite-Cache-Offline-Boundaries-v1
- EXCLUDES_STUBS:
  - WP-1-Loom-Storage-Portability-v4
  - WP-1-Loom-MVP-v1
  - WP-1-Video-Archive-Loom-Integration-v1
  - WP-1-Media-Downloader-Loom-Bridge-v1
  - WP-1-Loom-Preview-VideoPosterFrames-v1
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Storage portability, dual-backend, PostgreSQL-primary, and SQLite cache/offline anchors around Master Spec storage sections.
  - ModelSession scheduler, memory policy, provider profile, and parallel-session runtime anchors.
  - FEMS memory integration anchors, including shared memory records, replay, safeguards, and pack provenance.
  - Workflow durable execution anchors for workflow instance state, checkpoints, retries, and resumption.
  - DCC/work-tracking projection anchors for runtime truth, blocked/stalled work, queue depth, and intervention affordances.

## INTENT (DRAFT)
- What: Fold the immediate PostgreSQL-primary follow-on stubs into one larger control-plane shift WP, while keeping implementation split into microtasks small enough for cheaper coding-focused models.
- Why: The governance workflow can now babysit a larger packet mechanically, and the PostgreSQL pivot is tightly coupled enough that separate packets would force repeated setup, repeated schema decisions, repeated runtime-source labeling, and repeated DCC/FEMS/workflow handoff drift.
- Why now: `WP-1-Postgres-Primary-Control-Plane-Foundation-v1` is validated and contained in `main`, so follow-on work can build from an approved PostgreSQL-primary storage mode instead of rediscovering the same foundation.

## CARRY_OVER_RISKS_FROM_FOUNDATION_WP
- Live PostgreSQL service connection was not exercised.
- Bare `cargo test` still has unrelated integration-bin failures.
- MT-003 authority/freshness labels need downstream enforcement consumption.

## BUNDLE_SCOPE_POLICY
- This bundle intentionally includes PostgreSQL-primary control-plane follow-ons plus the SQLite cache/offline boundary because that boundary prevents split-brain fallback during the same pivot.
- This bundle intentionally excludes Loom. Loom still uses SQLite today and should use PostgreSQL in the future, but it remains a separate stub/WP family because it has distinct product surface, media/archive semantics, and historical smoketest lineage.
- Activation must preserve microtask-sized implementation slices. A larger WP is allowed only if every coder assignment stays bounded by a packet MT and each MT can be reviewed independently.
- Coder sessions must not implement cross-MT opportunistic cleanup. Shared schema/helper work must be assigned to the earliest MT that needs it and then reused by later MTs.

## SOURCE_STUBS_FOLDED

### WP-1-Postgres-Dev-Test-Container-Matrix-v1
- Intent carried forward: reproducible PostgreSQL developer/test service, migration reset, seeded fixtures, and CI-ready smoke profiles.
- Fold rationale: later lease, queue, FEMS, workflow, and DCC MTs require the same live PostgreSQL proof path.

### WP-1-Postgres-Control-Plane-Leases-Backpressure-v1
- Intent carried forward: shared PostgreSQL claim, lease, heartbeat, retry, dead-letter, and backpressure primitives for parallel control-plane work.
- Fold rationale: ModelSession, FEMS, and workflow workers need the same primitive set and should not fork incompatible queue/lease semantics.

### WP-1-ModelSession-Postgres-Queue-Workers-v1
- Intent carried forward: PostgreSQL authoritative ModelSession queues, worker claims, persisted messages, checkpoints, and model-profile state.
- Fold rationale: ModelSession queue workers are the first high-value consumer of leases/backpressure and the source for DCC session projection truth.

### WP-1-FEMS-Postgres-Memory-Store-v1
- Intent carried forward: PostgreSQL authoritative FEMS memory records, memory packs, memory jobs, replay metadata, and parallel-safe writes.
- Fold rationale: memory jobs need shared worker/lease semantics and ModelSession identity; landing separately would duplicate control-plane plumbing.

### WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
- Intent carried forward: PostgreSQL workflow instance state, node checkpoints, retry state, and crash-resume semantics.
- Fold rationale: workflow durable execution needs the same leases/backpressure and runtime truth model as ModelSession and FEMS jobs.

### WP-1-DCC-Postgres-Control-Plane-Projections-v1
- Intent carried forward: DCC projections over PostgreSQL runtime truth for sessions, queues, leases, workflows, memory jobs, and dead-letter states.
- Fold rationale: DCC should project the same canonical state introduced by the earlier MTs instead of inferring it from stale mirrors.

### WP-1-SQLite-Cache-Offline-Boundaries-v1
- Intent carried forward: explicit SQLite cache, index, offline, and rebuildable-projection boundaries so SQLite does not become accidental runtime authority.
- Fold rationale: PostgreSQL-primary implementation must fail closed when authority is required and must label any SQLite cache/offline projection from the start.

## MICRO_TASK_PLAN (DRAFT)

### MT-001 PostgreSQL Dev/Test Harness and Live-Service Proof
- Focus: local PostgreSQL service/test profile, migration reset, seeded fixtures, targeted storage proof commands, and clear skip/fail semantics.
- Acceptance: at least one live PostgreSQL service-backed proof path exists or activation records an explicit environment blocker; service-down, migration-failed, schema-drift, and fixture-drift failures are distinguishable.
- Must address carry-over risk: live PostgreSQL service connection was not exercised.

### MT-002 Storage Authority and SQLite Boundary Enforcement
- Focus: storage mode policy consumption, SQLite cache/offline/index boundaries, source/freshness metadata enforcement, and fail-closed writes when PostgreSQL authority is required.
- Acceptance: runtime control-plane writes cannot silently land in SQLite under PostgreSQL-required mode; derived/cache surfaces carry source and freshness labels.
- Must address carry-over risk: MT-003 authority/freshness labels need downstream enforcement consumption.

### MT-003 PostgreSQL Lease, Claim, Retry, Dead-Letter, and Backpressure Primitives
- Focus: shared queue/lease schema and helpers for pending, claimed, running, stalled, retryable, dead-letter, and completed work.
- Acceptance: concurrent claim attempts produce one winner; expired leases are reclaimable; configured limits block new claims with auditable state.

### MT-004 ModelSession PostgreSQL Queue Workers
- Focus: ModelSession run queue persistence, worker claims, persisted messages/checkpoints, cancellation/crash resume, and model-profile metadata.
- Acceptance: multiple workers claim distinct session items; profile IDs and fallback profile metadata persist; resume reads canonical PostgreSQL state.

### MT-005 FEMS PostgreSQL Memory Store and Memory Jobs
- Focus: MemoryItem, MemoryPack, provenance, tombstone/replay metadata, memory-job claim state, and parallel-safe reads/writes keyed to ModelSession identity.
- Acceptance: eligible sessions can read memory written by other sessions; concurrent memory writes avoid silent duplicate/canonical overwrite; replay provenance is queryable.

### MT-006 Workflow Engine PostgreSQL Durable Execution
- Focus: workflow instance state, node execution state, checkpoint payload metadata, retry counters, terminal outcomes, and lease-backed runnable workflow claims.
- Acceptance: workflow execution can stop/resume from PostgreSQL without replaying completed nodes incorrectly; parallel workers cannot execute the same node claim.

### MT-007 DCC PostgreSQL Control-Plane Projections
- Focus: projection models/endpoints for queue depth, active leases, stalled work, ModelSession state, workflow state, memory jobs, dead-letter items, and read-only versus governed action affordances.
- Acceptance: DCC-ready projection truth comes from PostgreSQL authority or declared derived projections with source/version/freshness metadata; stale/missing/conflicting states are explicit.

## ACCEPTANCE_CRITERIA (DRAFT)
- The bundle activates into an official WP with the seven MTs above or an equivalent microtask split that keeps coder assignments bounded.
- PostgreSQL-required runtime/control-plane operations fail closed when PostgreSQL authority is unavailable.
- SQLite is allowed only as cache, offline, embedded demo, search index, or rebuildable local projection where explicitly labeled.
- Queue/lease/backpressure primitives are shared by ModelSession, FEMS, and workflow jobs instead of independently reimplemented.
- DCC projections read canonical PostgreSQL state or clearly labeled derived projection state.
- Foundation carry-over risks are either resolved or carried forward as explicit non-blocking/product blockers by Integration Validator.
- Loom storage and video/archive stubs remain separate and are not activated through this bundle.

## RISKS / UNKNOWNs (DRAFT)
- Risk: the bundle is too large if MTs are allowed to bleed into each other; activation must enforce one MT per coder turn and one review route per MT.
- Risk: live PostgreSQL tests may be slow under host load; activation should set generous bounded timeouts and classify inconclusive environment failures separately from product failures.
- Risk: memory poisoning safeguards may need to land with MT-005 rather than as a later FEMS follow-up.
- Unknown: whether pgvector belongs in this bundle or remains an indexing/cache follow-up.
- Unknown: whether CI should require PostgreSQL live-service checks immediately or begin as a gated required profile.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the bundled requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block with explicit MT boundaries and the source-stub fold map.
- [ ] Obtain USER_SIGNATURE for the bundled WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-Postgres-Control-Plane-Shift-Bundle-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
- [ ] Keep folded source stubs superseded; do not reactivate them separately unless the Operator splits the bundle again.
