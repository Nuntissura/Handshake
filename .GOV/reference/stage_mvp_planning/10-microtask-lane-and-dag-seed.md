---
file_id: stage-mvp-microtask-lane-and-dag-seed
file_kind: reference-execution-decomposition
updated_at: "2026-07-19"
status: expanded-blueprint-non-execution
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-microtask-lane-and-dag-seed" status="expanded-blueprint-non-execution" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage microtask lane and DAG seed

## Status and use

This is a non-execution decomposition for hardened planning. The earlier seed proposed 186 future slots across 18 lanes. The full-feature and production audit proved that count and grouping were too coarse, so the count is no longer frozen. This version defines 23 future lanes and generation rules without creating official microtasks, granting execution authority, setting official file ownership, or satisfying refinement/spec approval gates.

The current product and active-WP worktrees were inspected read-only and mapped in `15-product-topology-and-active-wp-migration.md`. Before official generation, that snapshot must be rebound to exact approved commits and every generated MT must include exact owned/shared/forbidden/reference files, dependencies, constraints, expected behavior, acceptance criteria, verification commands, status synchronization, and evidence outputs.

## DAG spine

```text
L00 topology/evidence + L01 source-disposition/current-requirements + L02 threat model/registries
  -> L03 data/events/kernel + L04 diagnostics/fixtures spine
     -> L05 native shell/browser chrome
     -> L06 history/bookmarks/import + L07 high-volume scheduler
     -> L08 WebView2 production adapter + L09 Servo strategic adapter
     -> L10 network/permissions/security + L11 sessions/auth/secrets
     -> L12 agent control + L13 Stage Apps
     -> L14 capture/archive/parsers + L15 downloads/intake/ASR
     -> L16 Loom/search + L17 translation/export
     -> L18 accessibility/localization/manual
     -> L19 legacy Stage removal/optional data import + active-WP integration
     -> L20 packaging/update/supply chain + L21 backup/support/operations
     -> L22 integration/promotion/authority closure
```

Security, diagnostics, and product-topology reconciliation are continuous dependencies, not end-only audits.

## Expanded proposed lanes

| Lane | Planning scope | Mandatory dependency/proof |
|---|---|---|
| `L00` | approved integration baseline, product/active-WP topology, exact public APIs/schemas/migrations/tests/manual/packaging, dependency pins, allowed/shared/forbidden files | signed source map and runnable baseline commands |
| `L01` | all operator decisions, three-stub lineage/supersession dispositions, current requirements, spec conflicts, non-goals, active-WP obligations, archive/supersession plan | zero-orphan requirement and disposition coverage |
| `L02` | threat model; command/query/tool/capability/error/event/config/action/permission/artifact/evidence registries; adapter invariants | schema validators and threat-entrypoint coverage |
| `L03` | Stage records, CAS revisions, fencing leases, outbox/EventLedger, projectors, history, recently closed, bulk targets, cleanup, database indexes | persistence/replay/property tests |
| `L04` | fake-adapter harness, controlled sites, evidence manifests, structured health/log/trace/screenshot spine, redaction and zero-test guards | harness self-tests and evidence validation |
| `L05` | canonical native Stage module, omnibox, browser chrome, windows/tabs/sidebar, settings, context menus, permissions/download UI, focus/DPI/input/accessibility seams | browser-product and visual-debug journeys |
| `L06` | joint session history, visit history, recently closed, independent bookmarks, folders/order/undo, import/export/onboarding, canonical queries/bulk operations | history/bookmark/import round trip and 3,000-tab import |
| `L07` | hierarchical live budgets, lifecycle facets, protection reasons/expiry, service-worker truth, lazy restore, memory/network/disk/cache quotas, scheduler fairness | bounded renderer count, offscreen bulk, no restore storm |
| `L08` | direct WebView2 adapter, COM/STA ownership, callback cleanup, profiles/process events, browser services, runtime feature detection/update, Windows integration | complete Windows adapter/browser-service and failure corpus |
| `L09` | exact Servo pin/fork/toolchain, embedding queues, process/storage/request policy, restricted alpha, compatibility/packaging/rollback, later Windows sandbox promotion | restricted-alpha proof; arbitrary web remains security-blocked |
| `L10` | renderer/host/site isolation, request policy, proxy/TLS/DNS/private network, permissions, popups/schemes, ad/tracker/reputation, parser/security corpus | default-deny negative and containment suites |
| `L11` | session classes/materialization, auth compatibility, external OAuth, cookie editor/import/export, credential lease, clear data, encryption/retention/cleanup/cross-engine loss | cross-profile secret/isolation plus authenticated-site matrix |
| `L12` | versioned observations/actions, durable intent/receipt/reconciliation, source-sink control, prompt injection, parallel leases/budgets, operator watch/confirmation/takeover, WP-1 consumer | non-exfiltration/concurrent-agent/quiet-operation proof |
| `L13` | Stage App manifest/origin/signing/trust, install/update/revoke/rollback/migration, bridge methods, navigation reauthorization, outbound and diagnostics | bridge-origin/confusion/bypass and package lifecycle proof |
| `L14` | renderer capture vs WARC/archive, page/selection/readable/screenshot/PDF/media/document/3D, manifests, staging/finalization, sanitizers/parsers/fuzzing, portability | artifact/hash/provenance/partial/failure and hostile-input proof |
| `L15` | browser download manager, Media Downloader, ArtifactStore, project destination, Lens/Atelier/CKC, captions/ASR, partial results, retry/resume/cancel, credential leases | source-tab-to-project and renderer-kill continuation proof |
| `L16` | Loom ownership, folders/tags/relations, metadata/full-text/semantic search boundaries, URL normalization, ranking, saved queries, index lifecycle/rebuild/retention | graph consistency and zero-page-activity search proof |
| `L17` | local/cloud translation package lifecycle/evaluation, readable Markdown, print/captured PDF, Export/Materialize, provenance/egress/sanitization | offline/egress/immutability/export recovery proof |
| `L18` | AccessKit/UIA/browser focus, keyboard/screen reader/high contrast/scale/IME, localization/RTL/pseudo-locale, action/settings registry, operator/model manual | assistive-technology, localization, and no-context manual journeys |
| `L19` | removal/replacement of WP-12 Stage routes/panes/storage/capability/wire/adapters/connectors/mocks, optional one-way real-data import, 0341/0346/0348/0349 collisions and cross-table guards, new WP-1/editor/CKC contracts | legacy-surface absence, optional real-data reconciliation, and frozen current-consumer proof |
| `L20` | Windows packaging/installer baseline, WebView2/Servo/helper updates, signatures, SBOM/notices/advisories, component register, state-compatible restart/rollback/quarantine | clean-machine package/update/supply-chain matrix |
| `L21` | PostgreSQL/ArtifactStore backup, Stage portable payload, clean-machine restore, safe mode, recovery center, support bundle, incident/drill lifecycle | restore/count/hash, redaction, support, and fault-drill proof |
| `L22` | wave integration, WebView2 production promotion, Servo restricted/arbitrary-web promotion, spec/refinement/WP/MT/taskboard/traceability synchronization and closure | fresh production-gate registry and operator/validator verdicts |

Official generation must use one machine-readable allocator and reject overlaps, reversals, gaps, duplicate IDs, orphan requirements, cycles, file conflicts, migration collisions, missing commands/evidence, and incomplete status synchronization.

## Historical 18-lane seed crosswalk

The earlier 186-slot plan is retained only as historical planning evidence. This crosswalk explains how prior analysis informed the expanded lanes; it does not preserve old Stage scope or constrain the current DAG:

- old `L00/L01/L02` map into new `L00..L03`;
- old `L03/L04` map into new `L05..L07`;
- old Servo/Chromium `L05/L06` map into new `L08/L09`;
- old security/session/privacy `L07..L09` map into new `L10/L11` plus parts of `L20/L21`;
- old agent/Stage App `L10/L11` map into new `L12/L13`;
- old capture/intake `L12/L13` map into new `L14/L15` and migration `L19`;
- old Loom/search/translation/export/manual `L14/L15` map into new `L16..L18`;
- old diagnostics/integration `L16/L17` map into continuous `L04` plus `L19..L22`.

Only features independently present in the current requirement register survive. Stable official MT IDs are intentionally deferred until the expanded requirement graph and supersession dispositions are validated.

## Lane-level microtask shaping rules

Each official microtask should normally close one externally provable unit, such as:

- one schema/trait plus contract tests;
- one UI component plus visual/accessibility proof;
- one lifecycle transition plus resource/negative-path proof;
- one renderer capability plus shared conformance fixture;
- one integration hop plus artifact/job/receipt evidence;
- one failure/recovery scenario plus reproducible test;
- one documentation workflow plus executable fixture.

Avoid microtasks that merely say `implement browser`, `add tests`, `integrate Loom`, or `improve performance`. Those hide dependencies and cannot be completed reliably by a no-context model.

## Cross-lane invariants

Every applicable microtask must preserve:

- Servo strategic product direction, production-qualified bounded WebView2 Windows slice, and distinct promotion/closure gates;
- no silent renderer fallback;
- hostile-content/host authority separation;
- Workflow Engine/Mechanical Tool Bus routing for privileged/background work;
- ArtifactStore authority for bytes and Export/Materialize authority for external paths;
- Loom authority for knowledge relationships;
- canonical full-set operations rather than visible-row subsets;
- actor attribution, capability checks, receipts, and recoverability;
- quiet model operation without focus theft;
- disk-agnostic paths and project-local configuration;
- operator and no-context-model manual updates for shipped actions;
- visual-debug use for every GUI change;
- product/spec/task state synchronization only after actual proof.

## Dependency cautions

- New `L02/L04` threat modeling, registries, diagnostics, and fixtures begin before either real adapter.
- New `L08` cannot own shared Stage features; it supplies the complete production adapter/browser-service implementation for the Windows slice.
- New `L09` Servo work begins early enough to preserve the strategic architecture, but its arbitrary-web Windows gate cannot block a separately approved WebView2 production release.
- New `L10` security gates block arbitrary-web Servo regardless of functional compatibility.
- New `L14` capture does not depend on Stage Apps; both depend on the common kernel/security contracts.
- New `L15` reuses Media Downloader v2 and ArtifactStore; schema duplication is a failed task.
- New `L16` references Loom identities rather than introducing a Stage graph authority.
- New `L19` owns explicit legacy Stage removal/replacement, collision cleanup, optional one-way real-data import, and new current-contract integration; it does not create a WP-12 compatibility wave.
- New `L20/L21` make packaging, updates, backup, safe mode, and support first-class implementation scope rather than hidden closure work.
- New `L22` cannot close the WP while Servo strategic gates, current approved scope, historical-source dispositions, active-WP contracts, or operator workflows remain incomplete.

## Promotion to official microtasks

1. Obtain operator decisions recorded in `planning-readiness.yaml` and revalidate exact integration baselines.
2. Validate complete current requirements, source-lineage/supersession dispositions, spec conflicts, interfaces, risks, releases, and gate traceability.
3. Rewrite/validate the master-spec Stage topic.
4. Finalize and sign the Stage refinement and machine-readable WP contract.
5. Generate official microtasks from a machine-readable seed allocator with stable IDs and no range defects.
6. Validate every contract for closure unit, exact paths, dependencies, conflicts, acceptance, negative paths, commands, evidence, rollback, manual/diagnostics, parallel claims, and status synchronization.

</topic>
