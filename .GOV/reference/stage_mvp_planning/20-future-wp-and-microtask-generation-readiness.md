---
file_id: stage-future-wp-and-microtask-generation-readiness
file_kind: reference-future-authoring-readiness
updated_at: "2026-07-19"
status: hardened-planning-input-authority-work-deferred
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-future-authority-sequence" status="hardened-planning-input-authority-work-deferred" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Future Stage authority sequence

This document prepares the later authoring pass but does not perform it. The Master Spec, existing Stage stubs, taskboard, build order, traceability registry, refinement, future WP contract, and official MT files remain unchanged.

When the operator explicitly starts the authority pass, use this order:

1. Lock the decisions and blockers in `planning-readiness.yaml` against the then-current integration baseline.
2. Validate `requirements-and-traceability.yaml` for complete mapping from operator directions, `STAGE-DEC`, `STAGE-PRES`, `STAGE-SPEC`, active-WP interfaces, risks, release gates, non-goals, and explicit reaffirmed-or-superseded source dispositions.
3. Author the Master Spec Stage topic from the approved product/technical contracts and pass the canonical spec workflow.
4. Author a full refinement with source anchors, operator request, research basis, architecture, scope edges, assumptions, non-goals, red team, acceptance gates, rollout, and microtask plan.
5. Create the single replacement stub/packet from the current approved corpus. Retain complete source lineage, but do not inherit any older Stage requirement, implementation, adapter, connector, route, pane, schema, mockup, or compatibility behavior unless the current contract independently selects it.
6. Archive superseded Stage stubs/projections in per-WP archival folders only after traceability and operator approval prove the replacement.
7. Generate official MT contracts from a machine-readable allocator after the requirement graph, exact files, dependency graph, gate bindings, and conflict groups pass validation.
8. Update taskboard/build-order/traceability only through the canonical workflow and only after actual authority changes.

</topic>

<topic id="stage-future-wp-contract-schema" status="hardened-planning-input-authority-work-deferred" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Future WP/refinement contract requirements

The consolidated Stage WP must contain:

- stable WP ID/version/title and exact taskboard/traceability links;
- operator request and locked decision IDs;
- exact superseded source stubs and requirement-by-requirement current dispositions;
- approved Master Spec anchors and spec-conflict resolutions;
- complete current scope, non-goals, deliberately selected deferred items, dependencies, consumers, blockers, supersession boundaries, and ownership boundaries;
- versioned record/event/tool/capability/error/integration registries;
- product topology with exact owned/shared/forbidden/reference files at the approved baseline;
- release slices and non-deceptive closure semantics;
- P0/P1 requirements with stable acceptance and gate IDs;
- legacy Stage removal, optional real-data one-way import, current public compatibility, feature-flag, rollout, rollback, backup/restore, update, support, and incident contracts;
- security threat model, prompt-injection/data-flow controls, parser boundaries, and red-team scenarios;
- exact validation commands, fixture IDs, environment requirements, evidence manifests, freshness, and existing validator/operator closure surfaces;
- official MT index, DAG, conflict groups, merge owners, critical path, integration checkpoints, and status-sync transaction.

The refinement must let a no-context model understand why Stage exists, how it fits Handshake, how the production bootstrap differs from Servo promotion, which current systems it reuses, which active-WP snapshots are not yet proven, and what evidence closes each boundary.

</topic>

<topic id="stage-future-microtask-schema" status="hardened-planning-input-authority-work-deferred" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Future microtask schema and allocator

The future allocator is specified in `microtask-allocator-contract.yaml`. It is a
planning compiler contract, not a second microtask authority. Its only valid
output is an official `MT-###.json` contract that passes
`.GOV/roles_shared/schemas/MICRO_TASK_CONTRACT.schema.json`, starts from
`.GOV/templates/MICRO_TASK_CONTRACT_TEMPLATE.json`, and is registered by the
approved replacement packet. A generated Markdown projection is never authority.

The canonical output shape includes the following required and extended fields:

```yaml
schema_id: hsk.microtask_contract@1
schema_version: microtask_contract_v1
contract_authority: PRIMARY_MACHINE_READABLE
wp_id: WP-1-Handshake-Stage-MVP-v1
mt_id: MT-###
authority_files: {}
markdown_projection: {}
lifecycle:
  status: PENDING
  depends_on: []
  blocks: []
  active: false
scope:
  summary: one externally provable closure unit
  allowed_paths: []
  forbidden_paths: []
  acceptance_criteria: []
  proof_targets: []
file_ownership: {}
traceability: {}
external_dependency_gates: []
verification: {}
rollback_or_compensation: {}
diagnostic_obligations: []
gui_obligation: {}
user_manual_obligation: {}
hbr_obligations: []
hbr_int_009_tier_obligations: []
handoff: {}
status_sync: {}
red_team:
  required: true
  profile: DETERMINISTIC_CONTRACT_MIGRATION_V1
```

`microtask-allocator-contract.yaml` provides the deterministic field map from
the planning graph into that shape, required input states, split triggers,
conflict groups, status-sync transaction, and twenty fail-closed rejection
conditions. Those conditions include duplicate/gapped/reversed IDs, orphan
traceability, uncovered source/spec/Pillar dispositions, dependency cycles,
undeclared overlap, shared-file ownership without one merge owner, migration
collisions, unbound external gates, commands without zero-match guards,
acceptance without positive/negative/failure proof, stale evidence, legacy
Stage leakage, non-atomic closure units, projection drift, and canonical schema
or packet-validator failure.

Shared conflict groups must include at least native application/shell registry, backend `lib.rs`/API routing, command/event/capability/error registries, database migrations, workspace/Cargo manifests, UserManual registry, diagnostics/health registry, fixture/evidence registry, and release packaging. One integration/merge owner serializes each group.

## Status synchronization transaction

An MT state change names the evidence/validator verdict that authorizes it and updates official MT, `mt_index`, WP aggregate, taskboard row, build order, traceability registry, dependency gates, and validator projections through the existing canonical workflow. Failed partial synchronization is detected and repaired; no projection is treated as canonical. Completion cannot be inferred from code, a local note, or a planning gate alone.

</topic>

<topic id="stage-future-dag-waves" status="hardened-planning-input-authority-work-deferred" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Future DAG waves and integration checkpoints

The expanded lane blueprint in `10-microtask-lane-and-dag-seed.md` is not a fixed MT count. The future allocator creates tasks only after exact source ownership is known.

Required checkpoints:

1. topology, source lineage/supersession dispositions, current requirements, threat model, and registry schemas;
2. persistence/event kernel plus fake-adapter conformance and diagnostics spine;
3. native shell/browser product services plus WebView2 bootstrap;
4. sessions/auth/network/permissions and Windows security containment;
5. WP-1 agent control plus prompt-injection/non-exfiltration proof;
6. downloads/capture/archive/artifact/ASR/project intake plus WP-12 legacy Stage removal and optional real-data import;
7. Loom/search/translation/export/accessibility/localization/manual;
8. high-volume/resource, fault, migration, backup, packaging/update/support evidence;
9. WebView2 Windows production promotion;
10. Servo restricted alpha and later arbitrary-web security promotion;
11. unified WP/spec/task closure.

Diagnostics, fixtures, security, supersession/removal and optional data-import proof, UserManual, and visual/accessibility proof start with the first relevant implementation task; they are not postponed to a final audit lane.

</topic>
