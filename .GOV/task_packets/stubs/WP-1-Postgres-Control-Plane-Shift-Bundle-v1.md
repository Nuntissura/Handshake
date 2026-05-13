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
- STUB_STATUS: SUPERSEDED_AS_KERNEL_ACTIVATION_VEHICLE
- SUPERSEDED_BY: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- SUPERSEDED_PACKET_FILE: .GOV/task_packets/WP-KERNEL-001-Event-Ledger-Session-Broker-v1/packet.md
- SUPERSESSION_SCOPE: Kernel-first event-ledger/session-broker activation vehicle only; residual FEMS memory-store, full DCC projection, and generic workflow durable-execution scope remain preserved for downstream packets and stubs.
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
- There is no upper MT-count bias for this bundle. Activation Manager must prefer 20+ small deterministic MTs over fewer broad MTs when that improves trackability, restart recovery, validator targeting, or suitability for smaller local/cloud coding models.
- The MT list below is a minimum decomposition guide, not a cap. During activation, split any MT again if its code surfaces, test proof, or failure modes would force a small coder model to reason across unrelated authority boundaries.
- Coder sessions must not implement cross-MT opportunistic cleanup. Shared schema/helper work must be assigned to the earliest MT that needs it and then reused by later MTs.

## SOURCE_STUBS_FOLDED

## KERNEL_RESET_SUPERSESSION_NOTE
- This consolidated PostgreSQL bundle is no longer the activation vehicle for the kernel reset first slice.
- Kernel-relevant material from all folded source stubs has been moved into `WP-KERNEL-001-Event-Ledger-Session-Broker-v1` as no-context packet/refinement/microtask requirements.
- The move is partial by design: `WP-KERNEL-001-Event-Ledger-Session-Broker-v1` implements the first product kernel proof, not the full previous scope of FEMS PostgreSQL memory runtime, full DCC PostgreSQL projections, or generic workflow durable execution.
- Do not reactivate this bundle as a single WP unless the Operator explicitly reverses the kernel-first plan. Use the official Kernel001 packet for event-ledger/session-broker work and preserve residual scope in downstream kernel/product stubs.

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

## MICRO_TASK_PLAN (DRAFT MINIMUM DECOMPOSITION)

Activation Manager must convert this bundle into concrete official packet microtask files (`MT-001.md`, `MT-002.md`, etc.) during activation. The exact final split may exceed this list. Do not compress the plan just to keep the count low.

### MT-001 PostgreSQL Live-Service Dev Profile
- Focus: local PostgreSQL service/test profile, environment variable contract, startup/skip semantics, and first connection smoke path.
- Acceptance: one targeted command can prove a live PostgreSQL connection or emit a deterministic environment blocker; service-down and config-missing are distinguishable.
- Carry-over risk: live PostgreSQL service connection was not exercised.

### MT-002 Migration Reset and Seeded Fixture Path
- Focus: reset/reapply migrations against PostgreSQL and create a minimal seed fixture set for downstream queue/session/memory/workflow tests.
- Acceptance: a clean PostgreSQL database can migrate from empty state and load deterministic fixtures without relying on SQLite state.

### MT-003 Targeted PostgreSQL Proof Command Matrix
- Focus: narrow proof commands for storage, migration, lease, ModelSession, FEMS, workflow, and DCC slices, with host-load-safe timeouts.
- Acceptance: proof output separates PASS, PRODUCT_FAIL, ENVIRONMENT_BLOCKED, TIMEOUT_INCONCLUSIVE, and unrelated bare cargo failures.
- Carry-over risk: bare `cargo test` still has unrelated integration-bin failures.

### MT-004 Storage Mode Authority Contract
- Focus: PostgreSQL-required mode declaration, storage authority errors, and fail-closed runtime behavior when PostgreSQL authority is unavailable.
- Acceptance: authoritative control-plane writes cannot silently fall back to SQLite under PostgreSQL-required mode.

### MT-005 SQLite Cache and Offline Boundary Labels
- Focus: SQLite cache/offline/index labels, source identifiers, freshness timestamps, and rebuildability metadata.
- Acceptance: any SQLite-derived control-plane read surface advertises source/freshness and cannot masquerade as PostgreSQL authority.

### MT-006 SQLite Write Guard for Control-Plane Authority Paths
- Focus: explicit rejection of SQLite writes for runtime/control-plane authority surfaces that now require PostgreSQL.
- Acceptance: tests prove the forbidden write paths fail closed and return actionable authority errors.

### MT-007 PostgreSQL Control-Plane Schema Baseline
- Focus: shared tables/types for queues, leases, claims, worker identity, runtime item state, attempt counts, timestamps, and terminal status.
- Acceptance: schema supports pending, claimed, running, completed, retryable, stalled, and dead-letter states without subsystem-specific forks.

### MT-008 Atomic Claim and Lease Acquisition
- Focus: concurrent claim helper, lock/update strategy, worker identity, lease duration, and one-winner semantics.
- Acceptance: parallel claim attempts produce exactly one owner per item.

### MT-009 Lease Heartbeat, Expiry, and Reclaim
- Focus: heartbeat update, expired-lease detection, reclaim eligibility, stale worker metadata, and audit fields.
- Acceptance: expired work can be reclaimed deterministically while live claims remain protected.

### MT-010 Retry, Dead-Letter, and Backpressure Limits
- Focus: retry counters, max-attempt rules, dead-letter transition, queue caps, and pressure signals for blocked new claims.
- Acceptance: retry exhaustion and configured backpressure are visible as queryable state, not only logs.

### MT-011 Authority/Freshness Enforcement Consumption
- Focus: consuming authority/freshness labels in lease, queue, projection, and read paths that make runtime decisions.
- Acceptance: stale, SQLite-derived, or unknown-authority state cannot drive authoritative scheduling decisions.
- Carry-over risk: MT-003 authority/freshness labels need downstream enforcement consumption.

### MT-012 ModelSession Queue Schema and Persistence
- Focus: ModelSession queue item tables, payload serialization, status fields, profile IDs, fallback profile metadata, and timestamps.
- Acceptance: queued ModelSession work persists through process restart and reads back with model/profile metadata intact.

### MT-013 ModelSession Worker Claim and Cancellation
- Focus: worker claim path, cancellation state, cooperative stop, and claim release for ModelSession work.
- Acceptance: multiple workers claim distinct ModelSession items and cancelled items stop without being reprocessed as normal retries.

### MT-014 ModelSession Messages and Checkpoints
- Focus: persisted messages, checkpoints, turn/run state, and resume pointers tied to ModelSession identity.
- Acceptance: resume reads canonical PostgreSQL state and does not depend on transient runtime mirrors.

### MT-015 ModelSession Crash-Resume Proof
- Focus: tests or harness path simulating interrupted ModelSession work and recovery from PostgreSQL.
- Acceptance: interrupted work resumes or transitions to retry/dead-letter according to recorded state.

### MT-016 FEMS Memory Record and Pack Schema
- Focus: MemoryItem, MemoryPack, provenance, source session, write context, tombstone, and replay metadata in PostgreSQL.
- Acceptance: memory records and packs round-trip from PostgreSQL with provenance needed for later replay/evaluation.

### MT-017 FEMS Memory Job Queue
- Focus: memory extraction/injection/maintenance job item schema and shared lease/claim integration.
- Acceptance: FEMS jobs use the same PostgreSQL claim/lease primitives instead of a separate queue model.

### MT-018 FEMS Parallel Write and Dedup Proof
- Focus: concurrent writes, dedup/canonicalization behavior, tombstones, and conflict visibility.
- Acceptance: parallel memory writes avoid silent duplicate/canonical overwrite and expose conflict state when needed.

### MT-019 FEMS Cross-Session Read and Replay Proof
- Focus: session-to-session memory visibility, pack provenance, replay lookup, and query filters.
- Acceptance: eligible sessions can read memory written by other sessions with source/provenance preserved.

### MT-020 Workflow Instance and Node State Schema
- Focus: workflow instance state, node execution state, checkpoint metadata, terminal status, attempt counters, and timestamps.
- Acceptance: workflow state is persisted in PostgreSQL without relying on in-memory or SQLite authority.

### MT-021 Workflow Runnable Claim and Retry Semantics
- Focus: lease-backed runnable workflow/node claims, retry scheduling, terminal outcome transitions, and claim release.
- Acceptance: parallel workers cannot execute the same workflow node claim.

### MT-022 Workflow Crash-Resume Proof
- Focus: stop/resume proof for workflow instances and node checkpoints.
- Acceptance: resume does not replay completed nodes incorrectly and does not lose retry/dead-letter state.

### MT-023 DCC Session and Queue Projection Backend
- Focus: DCC backend projection models/endpoints for ModelSession state, queue depth, active workers, active leases, stalled work, and backpressure.
- Acceptance: DCC-ready session/queue truth comes from PostgreSQL authority or labeled derived projection state.

### MT-024 DCC Workflow and Memory Job Projection Backend
- Focus: DCC backend projection models/endpoints for workflow instances, runnable claims, memory jobs, retry/dead-letter states, and stale/conflicting states.
- Acceptance: DCC can distinguish healthy, stalled, retrying, dead-letter, stale, and conflicting control-plane states.

### MT-025 DCC Read-Only Versus Governed Action Affordances
- Focus: projection of which DCC controls are read-only, which require governed action, and which are blocked by stale/non-authoritative state.
- Acceptance: DCC action affordances cannot imply authority when the underlying state is stale, derived, or unavailable.

### MT-026 Cross-Subsystem Integration Proof
- Focus: one end-to-end live PostgreSQL path that touches storage authority, lease/claim, ModelSession, FEMS or workflow, and DCC projection truth.
- Acceptance: the integration proof is targeted, repeatable, and does not depend on unrelated bare cargo integration-bin success.

### MT-027 Carry-Forward Debt Ledger and Validator Handoff Map
- Focus: final packet truth for unresolved environment blockers, unrelated bare cargo failures, remaining spec debt, and downstream enforcement gaps.
- Acceptance: any unresolved risk is recorded with owner, blocker type, affected MTs, and whether it blocks Integration Validator PASS.

## ACCEPTANCE_CRITERIA (DRAFT)
- The bundle activates into an official WP with the minimum MT decomposition above or an equivalent finer-grained split that keeps coder assignments bounded.
- Activation Manager may create 20+ MTs and should do so whenever a smaller split improves deterministic execution, review targeting, restart recovery, or local/small-cloud model suitability.
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
