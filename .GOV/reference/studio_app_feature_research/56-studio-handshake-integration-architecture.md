---
file_id: studio-app-feature-research-handshake-integration-architecture
topic_id: SFR-STUDIO-INTEGRATION
title: "Studio ↔ Handshake Integration Architecture"
status: draft
summary: "Wires the Studio module into Handshake kernel pillars: EventLedger/PostgreSQL authority, CRDT, sandbox→validation→PromotionGate, three-tier diagnostics, Argus, Loom, FEMS, Job Runtime, DCC, UserManual; plus model visibility, visual steerability, parallel workflows, propose-work, per-file history/undo, visual inspection duty, headless/quiet law, the unified operator-facing creative surface, and the dual-audience Studio UserManual strategy."
sources: 14
updated_at: "2026-07-09"
---

## [SFR-STUDIO-INTEGRATION] Studio ↔ Handshake Integration Architecture

> **REFERENCE ONLY — NOT PRODUCT AUTHORITY.** This file is research/planning material for a
> future Master Spec enrichment. It does not authorize product code, spec edits, task status
> changes, or validator gates by itself. Handshake-side claims are grounded in the repo files
> listed in the EOF sources block; records marked `proposed_concept: true` name NEW concepts
> that do not yet exist in the product reference, spec, or HBR/Argus authority and require
> spec-enrichment adoption before use. The Tailor creative module (Master Spec §13.11/§13.13)
> is used throughout as the canonical kernel-binding pattern that Studio should follow.

### [SFR-STUDIO-INTEGRATION.pillar-wiring] Pillar Wiring

Studio binds to the kernel the same way Tailor does (§13.11): a `handshake_core` domain module
holding a `sqlx::PgPool`, a dedicated event-family constants block, sandbox dispatch through a
`SandboxAdapter`, validation through a domain `ValidationDescriptor`, authority mutation only
through PromotionGate, and model lanes through the `ModelAdapter` trait. One record per wired
pillar/capability follows. Per HBR-INT-009, tiers that are WIP (internal_diagnostics, Palmistry)
are recorded as typed DEFERRED postures, never silently skipped.

```yaml
pillar_wiring_records:
  - id: studio.integration.pillar-wiring.eventledger-authority
    pillar: "PostgreSQL + CRDT authority (product reference pillar 15)"
    wiring: >
      Every Studio mutation is a typed EventLedger event persisted to PostgreSQL before (or in
      the same CTE as) the authority row write, following the idempotent event-before-row
      pattern Tailor mandates (§13.11 11-K-2.2). Studio introduces the STUDIO_* wire-string
      event family concept (KernelEventType variants such as STUDIO_LAYER_CREATED,
      STUDIO_EDIT_PROPOSAL_RECORDED, STUDIO_EDIT_PROMOTED, STUDIO_EXPORT_RENDERED,
      STUDIO_HISTORY_UNDONE, STUDIO_HISTORY_REDONE) with dot-namespaced lowercase
      event_family constants (studio.layer_graph, studio.raster, studio.vector,
      studio.typography, studio.layout, studio.export, studio.history, studio.proposal)
      mirroring the tailor.<domain> convention (§13.11 11-K-7.3). File 10's seed contracts
      already assign per-command eventledger_event_family values (e.g.
      studio.layer_graph.layer_created); this record generalizes them into the family registry.
      Every WriteContext carries a KernelActor variant so model-authored Studio rows are
      distinguishable from operator rows in the audit log (§13.11 11-K-2.3/2.4).
    posture: WIRED_AT_BUILD
    proposed_concept: true   # STUDIO_* KernelEventType variants and family constants are new
    source_ids: [SI-S04, SI-S02, SI-S07]
  - id: studio.integration.pillar-wiring.crdt-collaboration
    pillar: "CRDT collaboration (product reference pillar 15; HBR-INT-004)"
    wiring: >
      Each open Studio file is one live CRDT document reusing the existing kernel/crdt/
      infrastructure (kernel_crdt_updates table) without a Studio-specific CRDT store,
      mirroring Tailor 11-K-12.1. Document mapping: document_id = studio file id;
      layer-graph subtrees, vector subtrees, story subtrees, and selection overlays are CRDT
      map subtrees within the single per-file document (matching the crdt_scope values file 10
      already assigns per command: document.layer_graph, document.vector_graph,
      document.story_graph, document.selection_overlay). Frontend collaborative editing uses
      the yjs_bridge path; server-side model-proposed mutations use the ai_edit_proposal path
      (§13.11 11-K-12.3/12.4). CRDT updates must persist, replay after reconnect, and promote
      through validation before authority per HBR-INT-004.
    posture: WIRED_AT_BUILD
    proposed_concept: false
    source_ids: [SI-S04, SI-S02, SI-S06]
  - id: studio.integration.pillar-wiring.sandbox-validation-promotiongate
    pillar: "Sandbox → validation → PromotionGate lifecycle (HBR-INT-003; Tailor §13.11 pattern)"
    wiring: >
      Model-authored Studio edits never write authority directly. A StudioSandboxAdapter
      (SandboxAdapter trait, process isolation tier, filesystem scoped to sandbox scratch, no
      network — mirroring TailorSandboxAdapter 11-K-8) executes command batches and renders
      previews in draft space; a StudioValidationDescriptor (wrapping the kernel
      ValidationDescriptor, with a stable check-code catalog and Blocking/Advisory severities
      like Tailor 11-K-9) validates document integrity; PromotionGate::evaluate() with
      OperatorApprovalEvidence and an idempotency key converts accepted work into authority
      events (11-K-10). Negative tests must prove direct mutation paths fail closed
      (HBR-INT-003). Automated self-approval is architecturally blocked (11-K-10.5).
    posture: WIRED_AT_BUILD
    proposed_concept: true   # StudioSandboxAdapter / StudioValidationDescriptor are new names following the Tailor pattern
    source_ids: [SI-S04, SI-S06]
  - id: studio.integration.pillar-wiring.flight-recorder-tier1
    pillar: "Flight Recorder — tier 1 business events (product reference pillar 1; HBR-INT-009 tier 1)"
    wiring: >
      Studio business events (file opened, edit promoted, proposal decided, export rendered,
      history undone/redone, import completed) emit Flight Recorder business-event records in
      the existing typed-event-family style (FR-EVT-* families; Loom already has
      FR-EVT-LOOM-*). A FR-EVT-STUDIO-* family is the natural extension, subject to
      spec-enrichment naming. Tier 1 is the kept-as-is business-event ledger; no schema
      re-open (HBR-INT-009).
    posture: WIRED_AT_BUILD
    proposed_concept: true   # FR-EVT-STUDIO-* family name is new
    source_ids: [SI-S03, SI-S05]
  - id: studio.integration.pillar-wiring.internal-diagnostics-tier2
    pillar: "internal_diagnostics — tier 2 internal self-diagnostics (product reference pillar 23; HBR-INT-009 tier 2; WIP in WP-KERNEL-012)"
    wiring: >
      Studio hooks the open diagnostic-event API: frame-time counters on every canvas/viewport
      render, panic hooks in engine modules (studio_raster, studio_vector, solver-style GPU
      kernels), UI-thread heartbeat participation for Studio panes, GPU/CPU/RSS counters
      during render and filter jobs, and typed diagnostic events for render-cache invalidation
      storms, GPU device-lost, and readback failures. No project/sensitive data (typed
      allowlist per HBR-INT-009).
    posture: DEFERRED_UNTIL_SHIPPED
    deferral: "internal_diagnostics is DESIGN-COMMITTED and built by WP-KERNEL-012, retrofit by WP-KERNEL-016; per HBR-INT-009 the Studio build records DEFERRED-with-reason plus integration follow-up until the tier ships — never a silent skip."
    proposed_concept: false
    source_ids: [SI-S03, SI-S05]
  - id: studio.integration.pillar-wiring.palmistry-tier3
    pillar: "Palmistry — tier 3 external watcher (product reference pillar 24; HBR-INT-009 tier 3; WIP)"
    wiring: >
      Studio registers Palmistry trackers where liveness/freeze/crash is worth
      surviving-capture: long-running render/export jobs, GPU filter batches, and the
      headless render harness (GPU work is Studio's highest freeze/crash risk — see
      risks record on headless GPU readback crashes). Shared-memory ring-buffer heartbeats,
      minidumps on crash, watchdog on job stalls; no project/sensitive data.
    posture: DEFERRED_UNTIL_SHIPPED
    deferral: "Palmistry is DESIGN-COMMITTED, built by WP-KERNEL-012 and retrofitted by WP-KERNEL-016; Studio records DEFERRED-with-reason plus a registration follow-up per HBR-INT-009."
    proposed_concept: false
    source_ids: [SI-S03, SI-S05]
  - id: studio.integration.pillar-wiring.argus-visual-targets
    pillar: "Argus visual inspection/steering (ARGUS-001..014; HBR-VIS-001..005; WIP in the CKC posekit overhaul worktree)"
    wiring: >
      Every Studio panel, tool, control, canvas region, and inspector carries a stable
      AccessKit/egui author_id so argus.inspect / argus.click / argus.set_value /
      argus.screenshot can identify, steer, and re-observe it (ARGUS-004/007/014). Creating
      or changing any Studio GUI surface is incomplete until the same MT/WP verifies the
      Argus-visible contract for that surface (ARGUS-014). Surfaces Argus cannot see or steer
      are HBR-VIS technical debt with allowed same-MT/WP remediation scope (HBR-VIS-005,
      ARGUS-009..011). Argus is headless and non-intrusive by definition (ARGUS-005).
    posture: WIRED_AT_BUILD
    posture_note: "Argus itself is WIP (native MCP facade exists; durable EventLedger mirroring of ActionLog receipts is not yet wired per ARGUS-013) — Studio must not claim Flight Recorder wiring of Argus receipts until that mirroring exists."
    proposed_concept: false
    source_ids: [SI-S05, SI-S08]
  - id: studio.integration.pillar-wiring.loom-artifact-blocks
    pillar: "Loom — artifact retrieval library (product reference pillar 7)"
    wiring: >
      Studio exports, render artifacts, reusable assets (brushes, styles, palettes, export
      recipes, placed-asset sources), and promoted document snapshots register as Loom
      library blocks with content hashing and relational linking, following the
      block-as-unit-of-meaning pattern the native editors already use (Native Editors × Loom
      interconnection). Loom operations continue to emit FR-EVT-LOOM-* events.
    posture: WIRED_AT_BUILD
    proposed_concept: false
    source_ids: [SI-S03]
  - id: studio.integration.pillar-wiring.fems-memory-records
    pillar: "Front End Memory System (product reference pillar 12; HBR-INT-006)"
    wiring: >
      Studio session decisions, blockers, insights, errors, and session open/close events
      write typed FEMS records with source links and retrieval policy (HBR-INT-006);
      freeform prose logs are insufficient. Examples: proposal-rejection rationale, recurring
      validation-failure patterns per document, operator style decisions a model should
      recall next session. Procedural writes remain review-gated per the FEMS pillar contract.
    posture: WIRED_AT_BUILD
    proposed_concept: false
    source_ids: [SI-S03, SI-S05]
  - id: studio.integration.pillar-wiring.job-runtime-scheduler
    pillar: "Execution / Job Runtime (product reference pillar 13; HBR-SWARM-002)"
    wiring: >
      Every Studio render, export, filter batch, import/format-conversion, preflight run, and
      headless capture executes as a kernel scheduler job with leases, backpressure, and
      cooperative cancellation, with Flight Recorder lifecycle events — the exact posture
      Tailor uses for simulations/refits/exports (IMX-TAILOR-03/04; §13.11 sandbox
      lifecycle). Long renders must honor cancellation within bounded time and loop counters
      must prevent runaway (HBR-SWARM-002).
    posture: WIRED_AT_BUILD
    proposed_concept: false
    source_ids: [SI-S03, SI-S06]
  - id: studio.integration.pillar-wiring.dcc-visibility
    pillar: "Command Center / DCC (product reference pillar 11)"
    wiring: >
      Studio agent sessions appear in the Execution Session Manager; Studio proposals appear
      in the Approval Inbox (the DCC panel already defined for pending reviews); Studio tool
      calls appear in the Tool Call Ledger; Studio job queues surface in Build/Test/Run-style
      queue panels. No Studio-private approval or session surface is invented — DCC is the
      operator visibility home.
    posture: WIRED_AT_BUILD
    proposed_concept: false
    source_ids: [SI-S03]
  - id: studio.integration.pillar-wiring.usermanual-duty
    pillar: "In-product internal UserManual (HBR-MAN-001..004)"
    wiring: >
      Every wired Studio command, panel, tool, workflow, diagnostic surface, storage/event
      contract, and navigation path is mirrored in the in-product internal UserManual in the
      same change, code-truthful (self-consistency tested), including the HBR-INT-009
      three-tier diagnostic posture per behavior (HBR-MAN-004). The existing research-package
      Feature Use Card → UserManual handoff index (file 18) feeds the Studio manual topics.
    posture: WIRED_AT_BUILD
    proposed_concept: false
    source_ids: [SI-S05, SI-S09]
```

### [SFR-STUDIO-INTEGRATION.model-visibility] Model Visibility (Backend and Frontend)

Operator requirement: Studio tools must be visible to models in BOTH backend and frontend.
Backend visibility is the typed document model behind command contracts; frontend visibility is
the Argus/AccessKit surface. Both derive from the same contracts — per the Tailor model-first
governing principle, human-facing affordances are projections of the same typed contracts the
model calls (§13.13 <N>.<i>.1); no separate model shim.

```yaml
model_visibility_records:
  - id: studio.integration.model-visibility.backend-document-read-api
    rule: >
      The Studio document model is queryable through typed read commands: scene/layer-graph
      tree read API (full tree and subtree snapshots with stable node ids), selectors
      (by id, by kind, by name pattern, by bounds, by style ref), story/vector/layout
      subqueries, and document metadata (status, history head, open proposals). Read commands
      are side-effect-free and never require a UI session, pixel coordinates, or drag handles
      (§13.13 model-input rule applied to Studio).
    proposed_concept: true   # the read-command set is new; the posture derives from Tailor 13.13
    source_ids: [SI-S02, SI-S07]
  - id: studio.integration.model-visibility.semantic-diffs-and-receipts
    rule: >
      Every mutating Studio command returns a typed model_receipt (file 10 already requires
      model_receipt on every seed contract) and supports a semantic diff view: before/after
      tree hashes, changed-node lists, and structured render receipts (output hash, bounds,
      render time) so a model can verify effect without prose parsing — the SimulationReceipt
      posture from Tailor (§13.13 <N>.<i>.3) applied to Studio commands.
    proposed_concept: false
    source_ids: [SI-S02, SI-S07]
  - id: studio.integration.model-visibility.frontend-authorid-targets
    rule: >
      Every Studio panel, tool button, slider, inspector field, canvas viewport, and dialog
      exposes a stable AccessKit/egui author_id (ARGUS-007, HBR-VIS-001). Models discover
      frontend state via argus.inspect (AccessKit tree) and argus.screenshot; steering uses
      argus.click / argus.set_value on those stable targets. Request-level agent_label
      attributes parallel clients sharing one binding token (ARGUS-007).
    proposed_concept: false
    source_ids: [SI-S05, SI-S08]
  - id: studio.integration.model-visibility.ui-state-snapshots
    rule: >
      Studio provides machine-readable UI-state snapshots: active document id, active tool,
      selection state, visible panels, zoom/viewport transform, pending-proposal badges, and
      conflict indicators — inspectable through the AccessKit tree and reconstructable from
      durable state per the TraceProjection duty (HBR-INT-007), so no model depends on chat
      history or screen-reading to know Studio's state.
    proposed_concept: true   # the snapshot record shape is new
    source_ids: [SI-S05, SI-S08, SI-S03]
  - id: studio.integration.model-visibility.command-contract-rule
    rule: >
      Every Studio command has a typed contract per the studio.command_contract.v0 schema
      (file 10) — command_id, typed_parameters, output_refs, state_mutations,
      undo_redo_semantics, eventledger_event_family, crdt_scope, diagnostics, model_receipt,
      failure_modes, verification — EXTENDED with three fields this file proposes:
      dry_run (mandatory on mutating commands, per file 10's own ROI item), replay
      (deterministic replay hook: same inputs + same document revision produce an equivalent
      result or a typed nondeterminism declaration, cf. Tailor's per-backend determinism
      posture 11-K-11), and history_entry (whether promotion of the command batch appends a
      per-file history entry — see history-undo below).
    proposed_concept: true   # dry_run/replay/history_entry as required schema fields are new
    source_ids: [SI-S02, SI-S04]
```

### [SFR-STUDIO-INTEGRATION.visual-steerability] Visual Steerability

How a model drives Studio, in strict priority order. OS input injection is never a path
(HBR-QUIET-002; ARGUS-005/006).

```yaml
visual_steerability_records:
  - id: studio.integration.visual-steerability.command-api-first
    rule: >
      The canonical way a model drives Studio is the typed command API (MCP tools over the
      command contracts), exactly as Tailor's MCP gate is the model's sole entry point
      (§13.13 <N>.<i>.1 consequence 5). GUI steering is for GUI-proof and operator-surface
      verification, not for performing document edits a command can perform.
    proposed_concept: false
    source_ids: [SI-S07, SI-S02]
  - id: studio.integration.visual-steerability.argus-gui-steering
    rule: >
      For GUI-proof (verifying panels, tools, controls, and visible state behave as
      specified), models use argus.click / argus.set_value on stable author_id targets with
      before/after argus.inspect or argus.screenshot observation (ARGUS-004/008/012). Mutating
      GUI actions carry explicit leases or receipts (ARGUS-007). Evidence is recorded with
      tool/path, snapshot refs, target author_ids, action sequence, and before/after
      observation (ARGUS-012).
    proposed_concept: false
    source_ids: [SI-S05, SI-S08]
  - id: studio.integration.visual-steerability.headless-render-harness
    rule: >
      Canvas/visual output is captured through a headless render harness: a StudioRenderHarness
      job that renders a document (or region/layer subset) off-screen to an artifact with a
      structured render receipt (output hash, bounds, color profile, backend used), running on
      the Job Runtime with the same GPU-or-CPU-fallback posture Tailor mandates for headless
      environments (11-K-8.5: record which backend was used; CPU fallback where GPU
      unavailable). On headless-GPU hosts where pixel readback is unreliable (known
      egui_kittest Harness::render 0xc0000005 crash), the harness falls back to
      AccessKit-tree/structured assertions per the documented Argus fallback discipline.
    proposed_concept: true   # StudioRenderHarness is a new named concept
    source_ids: [SI-S06, SI-S08, SI-S10]
  - id: studio.integration.visual-steerability.before-after-visual-evidence
    rule: >
      Every command with a visual effect owes before/after visual evidence: the command
      contract's diagnostics include preview refs/hashes (file 10 seed contracts already carry
      preview_hash / visual_order_proof style diagnostics), and the proposal/receipt flow
      stores before and after capture refs so any reviewer (model, validator, operator) can
      compare without re-running the edit.
    proposed_concept: false
    source_ids: [SI-S02, SI-S05]
```

### [SFR-STUDIO-INTEGRATION.parallel-workflows] Parallel Workflows

Operator requirement: multiple files worked by models concurrently, AND parallel models on a
single file or parallel files, all while the operator may be using Handshake on OTHER
projects/modules. Grounded in HBR-SWARM-001..004 and the Tailor CRDT/promotion pattern.

```yaml
parallel_workflow_records:
  - id: studio.integration.parallel-workflows.multi-file
    scenario: "One or more models across many files"
    contract: >
      One CRDT document per file; one job lane per file for render/export work; file-scoped
      leases on destructive or exclusive operations (format re-encode, raster snapshot
      compaction). Leases expire correctly and stale sessions are recoverable (HBR-SWARM-002).
      Cross-file operations (batch export, wardrobe-style grouping) are query-time groupings
      that never block per-file promotion — the Tailor wardrobe posture (11-K-14.3).
    proposed_concept: false
    source_ids: [SI-S05, SI-S04, SI-S06]
  - id: studio.integration.parallel-workflows.multi-model-single-file
    scenario: "Parallel models on a single file"
    contract: >
      CRDT presence with per-model actor identity: every session binds a distinct
      KernelActor::ModelAdapter identity (11-K-2.4) and Argus agent_label (ARGUS-007), so
      actions are observable and attributable. Concurrent edits to distinct subtrees (layers,
      panels, stories) merge via CRDT semantics; concurrent edits to the same node surface a
      typed conflict event requiring decision — the TailorCrdtConflictDetected pattern
      (11-K-12.5), generalized as STUDIO_CRDT_CONFLICT_DETECTED. Promotion ordering is
      serialized through EventLedger idempotency keys (the CPROM-{id}-{val_run_id} pattern,
      11-K-10.4): retrying a promotion returns the original receipt; two proposals promoting
      overlapping state resolve in ledger order with the second re-validated against the new
      authority head. No silent last-writer-wins across actors (HBR-SWARM-001).
    proposed_concept: true   # STUDIO_CRDT_CONFLICT_DETECTED naming is new
    source_ids: [SI-S04, SI-S05, SI-S08]
  - id: studio.integration.parallel-workflows.multi-model-multi-file
    scenario: "Parallel models across parallel files"
    contract: >
      The Job Runtime scheduler arbitrates: lane-based priority, backpressure under load,
      bounded-time cancellation, no lease starvation and no deadlock (HBR-SWARM-002/001).
      GPU-heavy jobs (renders, filter batches) declare resource class so the scheduler can
      serialize GPU contention instead of letting parallel jobs race the device (see risks).
    proposed_concept: false
    source_ids: [SI-S03, SI-S05]
  - id: studio.integration.parallel-workflows.operator-concurrency
    scenario: "Operator working other projects/modules while agents work Studio"
    contract: >
      Operator sessions and agent sessions bind to explicit project/module scopes; agent
      Studio sessions are headless-by-law (HBR-QUIET-001) and never pop windows, steal focus,
      or hijack keyboard/mouse (HBR-QUIET-001/002, ARGUS-005). Background launches record
      ownership metadata (owner_session, owner_wp, owner_role, started_at) in the
      ProcessOwnershipLedger and are reclaimed on close/failure/staleness/cancel
      (HBR-QUIET-003). The operator keeps editing CRDT-backed surfaces while agent sessions
      are active on adjacent state, with presence and conflict state visible in the UI
      (HBR-SWARM-003). Zero focus/Z-order change events during agent windows is the pass bar.
    proposed_concept: false
    source_ids: [SI-S05, SI-S08]
```

### [SFR-STUDIO-INTEGRATION.propose-work-system] Propose-Work System

Operator requirement: models propose edits/operations; operator or governed validation
accepts/rejects before authority mutation. This is the kernel sandbox → validation →
PromotionGate pattern plus the existing kernel ai_edit_proposal CRDT path (11-K-12.4),
specialized for Studio. Deliberately technical acceptance: visual-diff checks and
document-integrity checks — no subjective scoring in the gate.

```yaml
propose_work_records:
  - id: studio.integration.propose-work-system.studio-edit-proposal
    record_type: "StudioEditProposal"
    proposed_concept: true
    shape: >
      A typed proposal record a model authors into sandbox/CRDT draft space: proposal_id
      (prefixed string id, UUID v7 body per HBR-INT-008), document_id, author actor
      (KernelActor::ModelAdapter), command batch (ordered list of typed command invocations
      with parameters — the replayable edit itself), base document revision (history head the
      batch was authored against), preview artifact refs (before/after renders from the
      headless harness), per-command receipts from the sandbox dry-run/execution, and a
      natural_description field for edit coherence (the GarmentSpec natural_description
      posture, §13.13). Emitted as STUDIO_EDIT_PROPOSAL_RECORDED, mirroring
      TailorPanelAiEditProposalRecorded. Models MUST NOT self-approve (11-K-12.4, 11-K-10.5).
    source_ids: [SI-S04, SI-S07, SI-S02]
  - id: studio.integration.propose-work-system.validation-pipeline
    record_type: "proposal validation"
    proposed_concept: true   # the Studio check catalog is new; the descriptor pattern is Tailor's
    shape: >
      Deterministic checks first: document-integrity checks (tree well-formedness, no orphan
      nodes, no cycle parenting, style-ref resolution, locked-layer denial — generalizing the
      failure_modes already on file 10 seed contracts), command-contract checks (parameter
      ranges, dry-run consistency), and visual-diff checks (rendered before/after against the
      proposal's declared intent bounds: changed-region containment, no unexpected pixel delta
      outside declared bounds, output-hash stability of untouched regions). Each check has a
      stable code, Blocking/Advisory severity, and a suggested_fix with an RFC 6901 JSON
      Pointer into the command batch — the Tailor ValidationFinding self-correction contract
      (11-K-9.4). An optional validator-model pass may review preview artifacts and annotate
      findings, but validator-model output is Advisory unless the descriptor promotes specific
      typed checks; the gate decision is driven by
      ValidationReport::aggregate_blocks_promotion() (11-K-9.3).
    source_ids: [SI-S04, SI-S02]
  - id: studio.integration.propose-work-system.operator-approval-surface
    record_type: "approval routing"
    proposed_concept: false
    shape: >
      Pending proposals route to the DCC Approval Inbox (the existing panel for pending
      reviews: memory proposals, capability requests, MT escalations — Studio proposals join
      that queue) with preview artifacts, validation findings, and semantic diff attached.
      Operator approval produces OperatorApprovalEvidence; a governed auto-accept policy for
      low-risk proposal classes is possible only as an explicit operator-configured policy
      surface, never a model decision (11-K-10.5).
    source_ids: [SI-S03, SI-S04]
  - id: studio.integration.propose-work-system.promotion-conversion
    record_type: "PromotionGate binding"
    proposed_concept: false
    shape: >
      PromotionGate::evaluate() converts an accepted proposal into authority: the command
      batch's events append to the EventLedger as authority events
      (STUDIO_EDIT_PROMOTED plus the per-command family events), the per-file history stack
      gains one entry (see history-undo), document status/receipt fields update, and the
      idempotency key STUDIO-PROM-{proposal_id}-{val_run_id} makes retries return the
      original receipt (the CPROM pattern, 11-K-10.4). Rejection emits
      STUDIO_EDIT_PROPOSAL_REJECTED with a typed reason and leaves authority untouched
      (11-K-10.3).
    source_ids: [SI-S04]
  - id: studio.integration.propose-work-system.rejected-proposals-persist
    record_type: "rejection retention"
    proposed_concept: true
    shape: >
      Rejected proposals persist as replayable evidence: the proposal record, its preview
      artifacts, validation findings, and decision receipt are retained (queryable via
      EventLedger and surfaced in DCC), so a later model can learn why an edit was rejected
      (FEMS record for the rationale per HBR-INT-006) and an operator can re-open or replay a
      rejected batch against a newer document revision. Rejection is never silent deletion.
    source_ids: [SI-S04, SI-S05, SI-S03]
```

### [SFR-STUDIO-INTEGRATION.history-undo] Per-File History / Undo / Revert-of-Undo

Operator requirement (verbatim intent): "history/undo/revert undo 1 level deep per file".

**Ambiguity note (explicit).** The phrase has two defensible readings:

- **Reading A — one-level redo:** each file has a history stack with undo; after an undo, ONE
  level of revert-of-undo (redo) is guaranteed per file. Deep undo, shallow (≥1) redo.
- **Reading B — one-level undo and revert:** history is recorded deeply, but the interactive
  undo affordance itself is one level deep per file (undo last promoted batch; revert that
  undo), with deeper restoration done via history replay/snapshot restore rather than a live
  undo stack.

**Recommended interpretation: Reading A**, because (1) file 10's seed contracts already define
per-command `undo_redo_semantics` implying a multi-entry undo model, (2) EventLedger replay
makes deep history cheap while a deep redo *branch* tree is the genuinely expensive part — so
guaranteeing ≥1 redo level per file is the minimal honest contract, and (3) Reading B's
restriction to one undo level would contradict the non-destructive editing posture that
pervades the whole research package. Reading B is preserved below as the minimum floor: the
contract guarantees at least undo-deep/redo-1; deeper redo is permitted, not required. The
spec-enrichment pass must confirm this with the Operator.

```yaml
history_undo_records:
  - id: studio.integration.history-undo.per-file-history-stack
    record_type: "StudioHistoryEntry / per-file history stack"
    proposed_concept: true
    contract: >
      Every promoted command batch appends exactly one history entry to the owning file's
      history stack. Each entry is backed by EventLedger events (the promoted batch's event
      ids), carries entry_id, document revision before/after, semantic label, actor, and
      receipt refs. The stack is per file — history never entangles across files. History is
      durable and replayable: rebuilding the document from the ledger reproduces the same
      revision heads (HBR-INT-001 replay duty).
    source_ids: [SI-S04, SI-S05, SI-S02]
  - id: studio.integration.history-undo.undo-redo-contract
    record_type: "undo/redo semantics"
    proposed_concept: true
    contract: >
      Undo = apply the inverse command batch (preferred, from each command's declared
      undo_redo_semantics per file 10) or snapshot revert where no clean inverse exists.
      Undo itself is a promoted, ledger-recorded operation (STUDIO_HISTORY_UNDONE) — never a
      hidden state pop — so parallel sessions observe it and replay reproduces it.
      Revert-of-undo (redo): after an undo, at least ONE level of redo per file is guaranteed
      (STUDIO_HISTORY_REDONE), restoring the undone entry with stable ids where no conflicting
      edit landed in between; a conflicting intervening edit converts the redo affordance into
      a conflict surface, not a silent overwrite (HBR-SWARM-001). Both undo and redo produce
      receipts. Deeper redo stacks are permitted but not contractually required (see
      recommended interpretation).
    source_ids: [SI-S02, SI-S04, SI-S05]
  - id: studio.integration.history-undo.model-visible-history-api
    record_type: "history query API"
    proposed_concept: true
    contract: >
      Models query per-file history through typed read commands: list entries (with labels,
      actors, revisions), inspect an entry's semantic diff and receipts, and dry-run an
      undo/redo to preview effect before proposing it. Undo/redo requested by a model routes
      through the propose-work system like any other mutating operation.
    source_ids: [SI-S02, SI-S07]
  - id: studio.integration.history-undo.raster-snapshot-semantics
    record_type: "snapshot semantics for raster-destructive ops"
    proposed_concept: true
    contract: >
      Commands that destructively touch raster payloads (flatten, bake, destructive filter
      where no live-filter node is used) must capture a content-addressed raster snapshot
      artifact before mutation; the history entry's undo path is snapshot revert. Snapshot
      artifacts follow the artifact-manifest posture (content hashing, retention under
      artifact_root — the Tailor artifact-bundle pattern 11-K-8.4/17.3). Snapshot compaction
      is a scheduled job and never deletes a snapshot still reachable from the guaranteed
      undo/redo window.
    source_ids: [SI-S04, SI-S02]
```

### [SFR-STUDIO-INTEGRATION.visual-inspection-duty] Visual Inspection Duty

Operator requirement: models must visually inspect BOTH code/structured output AND visual
output before claiming an edit done. This is ARGUS-002/003 and HBR-VIS-001 applied to Studio's
dual nature (typed document + rendered canvas).

```yaml
visual_inspection_records:
  - id: studio.integration.visual-inspection-duty.dual-inspection-rule
    rule: >
      Before claiming any Studio edit done (proposal submission, MT handoff, PASS), the acting
      model must inspect BOTH: (1) structured output — document-model semantic diff, command
      receipts, exported-file integrity (hash + format validation, round-trip receipts for
      compatibility formats per the preamble file-format policy), AND (2) rendered visual
      output — before/after canvas captures from the headless render harness and/or
      argus.screenshot of the operator surface, compared against expected behavior. Uninspected
      screenshots, exit codes, and unit tests alone are not evidence (ARGUS-003, HBR-VIS-001).
    proposed_concept: false
    source_ids: [SI-S08, SI-S05, SI-S01]
  - id: studio.integration.visual-inspection-duty.evidence-rows-in-receipts
    rule: >
      Proposal receipts and MT/WP handoff artifacts carry evidence rows: before capture ref,
      after capture ref, capture tool/path, target author_ids when GUI steering occurred,
      action sequence, and any unremediated HBR-VIS gaps — the ARGUS-012 record shape embedded
      in the StudioEditProposal receipt structure so the evidence travels with the proposal.
    proposed_concept: true   # embedding ARGUS-012 rows into proposal receipts is new
    source_ids: [SI-S08, SI-S02]
  - id: studio.integration.visual-inspection-duty.headless-fallback-discipline
    rule: >
      On hosts where pixel capture is unavailable or unreliable (headless GPU readback crash
      class), the model records the typed fallback used (AccessKit-tree assertions, structured
      render receipts, CPU-backend render) and the reason; visual-evidence duty is degraded
      loudly, never skipped silently — matching the documented run-pixel-screenshots-on-real-GPU
      / fall-back-to-argus.inspect discipline.
    proposed_concept: false
    source_ids: [SI-S10, SI-S08]
```

### [SFR-STUDIO-INTEGRATION.headless-quiet-law] Headless / Quiet Law

Operator requirement: single-file and multi-file work by single or parallel agents runs
headless, never steals keyboard input, never pops windows/apps, never conflicts with the
Operator using the app on OTHER projects/modules at the same time.

```yaml
headless_quiet_records:
  - id: studio.integration.headless-quiet-law.agent-sessions-headless
    rule: >
      Agent-driven Studio work runs headless or hidden-window: no window creation, no
      foregrounding, no Z-order change during agent activity; the operator's foreground
      application stays in front with zero focus-change events during the run window
      (HBR-QUIET-001). This holds identically for one agent on one file and N agents on N
      files.
    proposed_concept: false
    source_ids: [SI-S05]
  - id: studio.integration.headless-quiet-law.no-os-input-injection
    rule: >
      Every Studio automation surface is reachable without OS-level keyboard injection, cursor
      movement, focus stealing, or foregrounding (HBR-QUIET-002); a negative test proves the
      surface cannot be driven by global input hijacking. Argus is the only GUI steering path
      and is itself headless and non-intrusive (ARGUS-005); foreground interaction, if ever
      genuinely unavoidable, is a declared bounded exception with operator-visible warning
      (HBR-QUIET-004, ARGUS-006).
    proposed_concept: false
    source_ids: [SI-S05, SI-S08]
  - id: studio.integration.headless-quiet-law.process-ownership-reclaim
    rule: >
      Every background Studio process (render jobs, harness instances, sandbox runners)
      records ownership metadata (owner_session, owner_wp, owner_role, started_at) and is
      reclaimed at session close, failure, staleness, or operator cancel; no blank terminals,
      no orphan processes after run/failure/kill (HBR-QUIET-003; ProcessOwnershipLedger
      reclaim is already wired per HBR enforcement notes).
    proposed_concept: false
    source_ids: [SI-S05]
  - id: studio.integration.headless-quiet-law.operator-agent-scope-isolation
    rule: >
      Concurrent operator + agent sessions on different projects/modules must not interfere:
      sessions bind to project/module scopes; agent Studio sessions on project A never touch
      the operator's interactive state on project B — no shared focus, no shared modal state,
      no cross-project lease contention; shared infrastructure (scheduler, GPU, PostgreSQL)
      arbitrates via the Job Runtime lanes/backpressure rather than blocking the operator's
      interactive thread (HBR-SWARM-001/002/003 + HBR-QUIET-001 in combination).
    proposed_concept: true   # explicit project/module session-scope binding is a new articulation
    source_ids: [SI-S05, SI-S03]
```

### [SFR-STUDIO-INTEGRATION.operator-unification] Operator Unification Surface

Studio is not model-only tooling. It must work as ONE unified operator-facing creative
application replacing Photoshop, Illustrator, InDesign, Affinity-suite, and Figma-class
workflows for a human operator. The corpus already carries the architecture lesson: Affinity's
persona/StudioLink crossing proves raster, vector, and page-layout tools can operate inside one
document model without launching separate applications, and the cross-app dedupe policy (file
44) mandates that shared capability across source apps maps to ONE Handshake-native Studio
primitive, not duplicate per-vendor implementations. The operator UI must reflect that
single-primitive reality.

```yaml
operator_unification_records:
  - id: studio.integration.operator-unification.unified-document-and-tool-surface
    rule: >
      One unified Studio document model and one unified tool surface span raster, vector,
      layout/page, and design-system domains. There are no per-source-app silos: a shared
      capability (e.g. layers, masks, text, export) exists once as a Studio primitive
      (file 44 core_rule: "Shared capability across source apps maps to one Handshake-native
      Studio primitive, not duplicate Adobe/Affinity/Figma implementations") and every
      operator-facing tool, panel, and command is a projection of that single primitive —
      the same typed contracts the models call (§13.13 model-first governing principle:
      human affordances are projections of the model API, no separate shim).
    proposed_concept: false
    source_ids: [SI-S12, SI-S07, SI-S02]
  - id: studio.integration.operator-unification.task-modes-over-one-document
    rule: >
      Operator workflows are organized as workspace/persona-style TASK MODES over the SAME
      document and primitives — a photo-editing mode, a vector mode, a page-layout mode, and
      a design-system/prototyping mode — never separate apps or separate document states.
      Provenance: Affinity personas and StudioLink are the proven field pattern; the corpus's
      own rebuild lesson states StudioLink "should not be copied as an app-switching UX. It
      should become a shared primitive architecture: the same layer graph, vector path engine,
      layout frame engine, and export system exposed through task-focused work modes"
      (file 02, Affinity rebuild lesson). Switching mode changes tool prominence and panel
      layout only; document state, selection, and history are untouched.
    proposed_concept: true   # StudioTaskMode as a named Studio concept is new (pattern is Affinity provenance)
    source_ids: [SI-S11, SI-S12]
  - id: studio.integration.operator-unification.shared-ux-invariants
    rule: >
      Shared operator UX invariants across all task modes and domains: ONE selection model
      (the StudioSelectionSet primitive regardless of raster/vector/layout context), ONE
      undo/history surface (the per-file history stack of this file's history-undo subtopic —
      no per-mode undo stacks), ONE color pipeline (single color-management surface across
      raster painting, vector fills, and layout swatches), ONE asset/library surface (Loom is
      the library home for brushes, styles, palettes, components, export recipes, and placed
      assets — no Studio-private asset silo), and ONE export surface (StudioExportRecipe for
      raster slices, vector exports, and page/PDF output alike, per file 10's
      studio.export.render_recipe.v0 contract).
    proposed_concept: false
    source_ids: [SI-S02, SI-S03, SI-S12]
  - id: studio.integration.operator-unification.operator-model-cowork
    rule: >
      Operators and models co-work in the SAME unified surface: the per-file CRDT document
      carries presence for operator sessions and model sessions alike, with per-actor
      identity and attribution (KernelActor variants; ARGUS-007 agent_label). Agent activity
      is visible in the operator UI (presence indicators, pending-proposal badges, conflict
      state per HBR-SWARM-003) but never intrusive: agent sessions remain headless-by-law and
      never take focus, pop panes, or move the operator's viewport (see the
      headless-quiet-law subtopic; HBR-QUIET-001/002). The operator keeps editing while
      agents work adjacent state (HBR-SWARM-003).
    proposed_concept: false
    source_ids: [SI-S04, SI-S05, SI-S08]
```

### [SFR-STUDIO-INTEGRATION.usermanual-strategy] Studio UserManual Strategy (Dual-Audience)

Handshake has ONE in-product internal UserManual for no-context models AND operators
(CX-982-001: "Handshake has one in-product internal UserManual for no-context models and
operators; legacy ModelManual identifiers are aliases only"); HBR-MAN-001..004 enforce
same-change currency, no-context usability, code-truthfulness, and diagnostic linkage. The
Operator expects Studio manual coverage to be EXTENSIVE and DETAILED — the manual is how a
no-context model discovers the full Studio tool surface.

```yaml
usermanual_strategy_records:
  - id: studio.integration.usermanual.dual-audience-entry-contract
    rule: >
      Every Studio tool, command, panel, and workflow gets ONE manual entry with two layers
      of the same entry (one surface, never a parallel manual per CX-982-001):
      (a) operator layer — user-friendly, task-oriented, minimal technicality ("how do I
      crop", "how do I mask", "how do I set type on a path"), navigation path, and expected
      result; (b) model layer — technically complete: command_id, typed inputs/outputs,
      dry-run availability, receipt shape, undo semantics, Argus author_id targets for the
      surface, proof/evidence path (what capture or receipt proves the action worked),
      failure modes, and recovery steps. Both layers satisfy CX-982-003 (purpose, usage
      path, invocation, expected I/O, failure modes, recovery, verification proof) and
      CX-982-004 (Flight Recorder/EventLedger linkage + HBR-INT-009 three-tier posture per
      entry).
    proposed_concept: true   # the two-layer entry shape for Studio is a new articulation of the single-manual law
    source_ids: [SI-S14, SI-S05]
  - id: studio.integration.usermanual.full-tool-surface-coverage
    rule: >
      Coverage duty: ALL Studio tools available and how to use them. A no-context model must
      be able to discover the complete Studio tool surface from the manual alone — no chat
      history, no source-app (Photoshop/Affinity/etc.) prior knowledge, no repo reading
      (HBR-MAN-002 no-context operation test; CX-982-003). Coverage completeness is
      checkable: every shipped command contract (file 10 promotion rule: implementation-ready
      only with a contract) must have a matching manual entry, and manual entries for wired
      surfaces must be code-truthful with self-consistency tests (HBR-MAN-003) — drift is a
      build-rule fail. Roadmap/not-yet-wired entries carry the explicit roadmap tag
      (HBR-MAN-003 not_applicable clause).
    proposed_concept: false
    source_ids: [SI-S14, SI-S05, SI-S02]
  - id: studio.integration.usermanual.same-change-currency
    rule: >
      Every implementation MT that adds, changes, wires, exposes, deprecates, or removes a
      Studio behavior updates the UserManual in the SAME change (HBR-MAN-001, CX-982-002),
      with self-consistency verification (HBR-MAN-003), the no-context operation harness
      (HBR-MAN-002), and the HBR-INT-009 diagnostic-posture linkage per entry (HBR-MAN-004,
      CX-982-004/005 — internal_diagnostics/Palmistry absence recorded as
      DEFERRED-with-reason, never silently skipped).
    proposed_concept: false
    source_ids: [SI-S05, SI-S14]
  - id: studio.integration.usermanual.scale-plan-seeded-from-corpus
    rule: >
      Scale plan: the manual topic tree is generated/seeded, not hand-authored from zero.
      The research corpus already groups 2,730 Feature Use Cards into Studio manual surfaces
      via 18-feature-use-card-manual-handoff-index.md (app_counts: affinity 1032,
      indesign 542, illustrator 515, photoshop 441, figma 200; all currently
      planning_only / 0 implemented manual topics). The Studio manual topic tree should be
      generated from that index's manual_topic_groups, then enriched by later deep-delta
      research files. NOTE (honesty record): the deep-delta files referenced as 51-55 by the
      Operator's requirement are NOT present in the research folder at authoring time
      (folder currently ends at 50 plus this file 56); they are recorded here as planned
      seeding inputs, NOT_INSPECTED, to be wired into the generation path when they land.
      Manual entries stay planning_only until command-contract promotion, per the index's
      own handoff rule.
    proposed_concept: true   # the generated-manual-tree pipeline is a new named plan
    source_ids: [SI-S13, SI-S01]
  - id: studio.integration.usermanual.searchability-navigation
    rule: >
      The Studio manual is queryable along at least four axes: (1) tool name ("Crop",
      "Layer Mask"), (2) task intent ("remove background", "set type on a path") — the Feature
      Use Card user-purpose fields in the corpus are the seed data for task-intent aliases,
      (3) command_id (exact studio.<primitive>.<command>.v<N> lookup for models), and
      (4) Argus author_id target (reverse lookup: given a UI target, find the manual entry
      that documents the surface and its steering contract). Search must work for both
      audiences without chat history (CX-982-003; HBR-MAN-002), and navigation-path entries
      keep the manual the single navigation reference (HBR-MAN-001 commandReference/navigation
      duty).
    proposed_concept: true   # the four-axis query contract is new
    source_ids: [SI-S14, SI-S05, SI-S13, SI-S08]
```

### [SFR-STUDIO-INTEGRATION.risks-and-open-questions] Risks and Open Questions

```yaml
risks:
  - id: studio.integration.risks-and-open-questions.crdt-large-raster
    risk: "CRDT for large raster documents: CRDT map subtrees suit structural state (layer graph, vector, text) but multi-hundred-MB raster tiles inside a Yjs document would balloon update logs and reconnect replay."
    mitigation_candidates:
      - "Split state classes: structural state in CRDT; raster payloads as content-addressed artifacts referenced BY the CRDT (tile refs), mutated via promoted command batches, not via CRDT byte-diffs."
      - "Tile-level addressing with copy-on-write so parallel sessions touch disjoint tiles."
      - "Measure reconnect-replay cost with fixture documents before committing the document mapping."
    source_ids: [SI-S04, SI-S02]
  - id: studio.integration.risks-and-open-questions.gpu-contention
    risk: "GPU contention across parallel render jobs: N agents triggering renders/filters race one GPU; device-lost or thrashing stalls the operator's interactive canvas."
    mitigation_candidates:
      - "GPU resource class on scheduler jobs; serialize or bound concurrent GPU jobs via lanes/backpressure (HBR-SWARM-002)."
      - "CPU fallback path for non-interactive jobs (the ClothSolver cpu_fallback posture, 11-K-8.5)."
      - "Palmistry tracker on GPU job liveness once tier 3 ships; frame-time tier-2 counters to detect interactive-canvas starvation."
    source_ids: [SI-S05, SI-S06, SI-S03]
  - id: studio.integration.risks-and-open-questions.undo-across-dual-state
    risk: "Undo semantics across CRDT + EventLedger dual state: an undo entry computed against the ledger head can disagree with unpromoted CRDT draft edits in flight; naive inverse application can corrupt merge state."
    mitigation_candidates:
      - "Undo operates on promoted history entries only; draft-space edits are discarded/rebased explicitly, never silently folded into an undo."
      - "Undo of an entry with intervening promoted edits becomes a conflict surface requiring decision (mirrors 11-K-12.5)."
      - "Property test: ledger replay after any undo/redo sequence reproduces the same revision head (HBR-INT-001)."
    source_ids: [SI-S04, SI-S05]
  - id: studio.integration.risks-and-open-questions.proposal-merge-conflicts
    risk: "Proposal-merge conflicts: two proposals authored against the same base revision both pass validation, but promoting the second against the post-first authority head changes its effect."
    mitigation_candidates:
      - "Every proposal records base revision; PromotionGate re-validates (and re-renders previews) when head != base before accepting."
      - "Promotion ordering serialized via EventLedger idempotency; stale proposals get a typed REBASE_REQUIRED finding instead of silent drift."
    source_ids: [SI-S04]
  - id: studio.integration.risks-and-open-questions.headless-gpu-capture-crash
    risk: "Headless GPU screenshot crashes: egui_kittest Harness::render readback is documented in this repo to crash 0xc0000005 on headless-GPU hosts, threatening the render-harness and Argus screenshot evidence paths."
    mitigation_candidates:
      - "Backend detection + typed degradation: AccessKit-tree/structured-receipt evidence on headless hosts, pixel capture on real-GPU hosts (existing coder-brief discipline)."
      - "CPU-render backend for the harness as the deterministic evidence path on CI."
      - "Palmistry watchdog + minidump on harness crashes once tier 3 ships."
    source_ids: [SI-S10, SI-S08]
  - id: studio.integration.risks-and-open-questions.performance-ceilings
    risk: "Performance ceilings: event-per-mutation and history-entry-per-batch granularity could make brush strokes or drag interactions emit pathological event volume; per-command EventLedger appends are not free."
    mitigation_candidates:
      - "Interaction coalescing: continuous gestures (brush stroke, drag) are ONE command batch with typed parameters, not per-sample events; ephemeral previews stay off-ledger (the ephemeral-selection posture already in file 10's create_subject_selection contract)."
      - "Benchmark event-append and replay throughput against fixture edit sessions before fixing granularity in the spec."
    source_ids: [SI-S02, SI-S04]
  - id: studio.integration.risks-and-open-questions.manual-scale-currency
    risk: "UserManual scale vs same-change currency: the corpus maps 2,730 planning-only feature cards to Studio manual surfaces; if manual entries are hand-authored per MT while the tool surface grows at that scale, HBR-MAN-001 same-change currency becomes the WP bottleneck or drifts."
    mitigation_candidates:
      - "Generate the manual topic tree from the file-18 handoff index and the command-contract registry; MTs fill/verify generated stubs rather than authoring from zero."
      - "HBR-MAN-003 self-consistency tests double as coverage tests: a shipped command contract without a manual entry fails the build rule, keeping currency mechanical."
      - "Dual-audience layers share one entry record so operator and model content cannot drift apart."
    source_ids: [SI-S13, SI-S05]
  - id: studio.integration.risks-and-open-questions.diagnostics-tier-timing
    risk: "Tier 2/3 timing dependency: internal_diagnostics and Palmistry are WIP; if Studio build starts before they ship, deferred wiring can silently rot."
    mitigation_candidates:
      - "Typed DEFERRED-with-reason rows per HBR-INT-009 at build time plus the WP-KERNEL-016 retrofit lane; UserManual entries record the posture (HBR-MAN-004) so drift is testable."
    source_ids: [SI-S05, SI-S03]
open_questions:
  - id: studio.integration.risks-and-open-questions.q-undo-reading
    question: "Confirm the operator's intended reading of 'history/undo/revert undo 1 level deep per file' (Reading A one-level-redo recommended; Reading B one-level-undo floor preserved)."
  - id: studio.integration.risks-and-open-questions.q-event-granularity
    question: "Canonical STUDIO_* event variant list and family registry: which command families get first-slice registration, and what coalescing rule is normative for continuous gestures?"
  - id: studio.integration.risks-and-open-questions.q-auto-accept-policy
    question: "Should a governed auto-accept policy class exist for low-risk proposals (e.g., metadata-only edits), and where is that policy surfaced (DCC panel vs settings authority)?"
  - id: studio.integration.risks-and-open-questions.q-raster-authority-shape
    question: "Authority storage shape for raster payloads: artifact-referenced tiles vs JSONB-adjacent blobs; needs a contracts pass equivalent to Tailor's T-CONTRACTS before tables are named."
  - id: studio.integration.risks-and-open-questions.q-history-redo-depth
    question: "Is redo depth >1 per file worth the branch-tree complexity, or is linear redo with history-replay restoration sufficient for the first slice?"
  - id: studio.integration.risks-and-open-questions.q-fr-evt-family
    question: "Flight Recorder family naming: FR-EVT-STUDIO-* proposed here needs registration against the existing FR event-family table during spec enrichment."
  - id: studio.integration.risks-and-open-questions.q-deep-delta-51-55
    question: "Deep-delta files 51-55 (referenced by the Operator as parallel-authored manual/topic seeding inputs) are not present in the research folder at authoring time; confirm their ids/titles when they land and wire them into the manual topic-tree generation path alongside file 18."
  - id: studio.integration.risks-and-open-questions.q-task-mode-set
    question: "Canonical StudioTaskMode set and switching semantics: are photo/vector/page-layout/design-system the first-slice modes, and does mode state persist per file, per workspace, or per operator profile?"
```

### [SFR-STUDIO-INTEGRATION.sources] Sources

```yaml
sources:
  - id: SI-S01
    path: ".GOV/reference/studio_app_feature_research/00-preamble.md"
    note: "Scope, non-authority stance, naming and file-format policy, schema, risks, build approach."
  - id: SI-S02
    path: ".GOV/reference/studio_app_feature_research/10-studio-command-contracts.md"
    note: "studio.command_contract.v0 schema, seed contracts with eventledger_event_family / crdt_scope / model_receipt / undo_redo_semantics, dry-run and receipt ROI items."
  - id: SI-S03
    path: ".GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md"
    note: "Pillar inventory (Flight Recorder #1, Loom #7, DCC #11, FEMS #12, Execution/Job Runtime #13, PostgreSQL+CRDT authority #15, Studio #18, ACE #21, internal_diagnostics #23, Palmistry #24), FR event families, DCC panels, Stage/Studio boundary, Tailor kernel-integration summary. Navigation aid; spec sections are canonical."
  - id: SI-S04
    path: ".GOV/spec/master-spec-v02.198/spec-modules/13-tailor-cloth-garment-engine.md"
    note: "§13.11 Kernel Integration (PostgreSQL sole authority, event-before-row CTE, KernelActor variants, TAILOR_* event families, sandbox adapter, validation descriptor + check catalog, PromotionGate idempotency + no self-approval, MeshComparator determinism, CRDT kernel reuse + yjs_bridge + ai_edit_proposal + conflict events, model lanes) and §13.13 Model-First API (model API = product API, SimulationReceipt self-correction, MCP consent gate, JSON Pointer suggested_fix, iteration caps). Canonical creative-module binding pattern."
  - id: SI-S05
    path: ".GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json"
    note: "HBR pillars and rules cited: INT-001/003/004/006/007/008/009, SWARM-001..004, VIS-001..005, QUIET-001..004, MAN-001..004; ProcessOwnershipLedger reclaim; three-tier diagnostics deferral typing."
  - id: SI-S06
    path: ".GOV/spec/master-spec-v02.198/spec-modules/13-tailor-cloth-garment-engine.md"
    note: "§13.11 11-K-8 sandbox capability policy and cpu_fallback backend recording; 11-K-14 wardrobe non-blocking grouping; 11-K-17 portability. (Same file as SI-S04; split for citation clarity.)"
  - id: SI-S07
    path: ".GOV/spec/master-spec-v02.198/spec-modules/13-tailor-cloth-garment-engine.md"
    note: "§13.13 model-first governing principle and MCP tool table (author/simulate/edit/promote + consent column). (Same file as SI-S04.)"
  - id: SI-S08
    path: ".GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md"
    note: "ARGUS-001..014: canonical methods, headless/non-intrusive law, stable author_id targets, parallel-agent leases/agent_label, evidence record shape, HBR-VIS debt handling, EventLedger-mirroring caveat (ARGUS-013)."
  - id: SI-S09
    path: ".GOV/reference/studio_app_feature_research/19-studio-local-first-rust-posture.md"
    note: "Local-first Rust posture, engine map (studio_collaboration, studio_model_tools, studio_extensibility), command-contract + UserManual promotion rule."
  - id: SI-S10
    path: ".GOV/roles/coder/docs/CODER_STARTUP_BRIEF.md"
    note: "Documented egui_kittest Harness::render readback 0xc0000005 crash on headless-GPU hosts and the real-GPU-or-argus.inspect/AccessKit fallback discipline."
  - id: SI-S11
    path: ".GOV/reference/studio_app_feature_research/02-affinity-suite-feature-map.md"
    note: "Affinity personas + StudioLink provenance rows and the rebuild lesson: persona/studio crossing as shared primitive architecture exposed through task-focused work modes, not app-switching UX."
  - id: SI-S12
    path: ".GOV/reference/studio_app_feature_research/44-cross-app-overlap-and-affinity-dedupe-map.md"
    note: "Overlap policy: shared capability across source apps maps to one Handshake-native Studio primitive, not duplicate Adobe/Affinity/Figma implementations; source variants preserved for provenance/compatibility."
  - id: SI-S13
    path: ".GOV/reference/studio_app_feature_research/18-feature-use-card-manual-handoff-index.md"
    note: "2,730 Feature Use Cards grouped into Studio manual surfaces (affinity 1032 / indesign 542 / illustrator 515 / photoshop 441 / figma 200; all planning_only, 0 implemented manual topics); same-change manual handoff rule."
  - id: SI-S14
    path: ".GOV/codex/Handshake_Codex_v1.4.md"
    note: "CX-982 HARD_INTERNAL_USER_MANUAL_CURRENCY: one in-product internal UserManual for no-context models and operators (CX-982-001), same-change updates (CX-982-002), no-context entry content (CX-982-003), FR/EventLedger + three-tier diagnostic linkage (CX-982-004/005)."
```
