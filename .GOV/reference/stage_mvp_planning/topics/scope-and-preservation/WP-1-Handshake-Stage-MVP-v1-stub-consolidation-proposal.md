---
file_id: wp-1-handshake-stage-mvp-v1-stub-consolidation-proposal
file_kind: work-packet-stub-consolidation-proposal
updated_at: "2026-07-19"
status: deferred-requires-regeneration-from-hardened-corpus
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="wp-1-handshake-stage-mvp-v1-stub-consolidation-proposal" status="deferred-requires-regeneration-from-hardened-corpus" version="v1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# WP-1-Handshake-Stage-MVP-v1 stub consolidation proposal

This early consolidation proposal is retained as historical planning evidence but is not ready for execution or confirmation. The operator deferred stub folding and replacement-stub/refinement/official-MT creation until the hardened planning corpus is reviewed, and has since locked that the current Stage direction supersedes all older Stage-specific requirements and implementations regardless of build status. Regenerate this proposal from files `14` through `20`, all machine-readable registers, and later operator decisions before changing any source stub.

## Proposed transaction

Use `WP-1-Handshake-Stage-MVP-v1` as the single replacement Stage-owned stub ID and fold the older IDs into it supersession-first and lineage-complete:

- `WP-1-Stage-Media-Artifact-Portability-v1`;
- `WP-1-Stage-ASR-Transcript-Lineage-v1`.

The two absorbed stubs are archived in per-WP archival folders and marked superseded; they are not deleted. This proposal changes neither packet nor task state until the operator approves the transaction and the required authority workflow runs.

## Current replacement scope

The replacement stub is authored from the current approved Stage corpus. It does not automatically retain any prior scope, non-goal, acceptance criterion, implementation, adapter, connector, route, pane, schema, or mockup. Similar outcomes survive only where current `STAGE-REQ-*` requirements or later operator decisions independently select them.

The current corpus presently selects media portability and ASR lineage outcomes as current lanes. Their old packet shapes are still superseded:

### Media artifact portability lane

- shared portable artifact manifests for Stage capture/import/session outputs;
- stable semantic identity across storage backends;
- capture/import/session bundle indexes;
- retention, export, materialization, replay, and backend-swap invariants;
- Media Downloader session/materialization interoperability;
- canonical ArtifactStore handles, hashes, provenance, and cleanup behavior;
- current non-goals, risks, dependencies, blockers, and acceptance criteria from the hardened corpus.

### ASR transcript lineage lane

- Stage media selection/capture to ASR job lineage;
- exact source hash and media-probe facts;
- timing anchors and transcript artifact linkage;
- progress, failure, retry, and correlation receipts;
- downstream Loom, Lens, Atelier, CKC, and archive assumptions;
- current non-goals, risks, dependencies, blockers, and acceptance criteria from the hardened corpus.

The requirement-by-requirement source-lineage and disposition ledger is `.GOV/reference/stage_mvp_planning/01-source-preservation-register.md` and must be used as a consolidation validator input. Its `STAGE-PRES-*` IDs are provenance, not legacy survival guarantees.

## Dependency rewrite

Remove dependencies from the surviving umbrella to the two absorbed Stage stubs and remove any absorbed-stub dependency on the umbrella that would become a self-dependency. Retain or restate every external relationship, including:

- Artifact System Foundations;
- Storage Trait Purity;
- Media Downloader;
- ASR Transcribe Media;
- Media Downloader Loom Bridge;
- Video Archive Loom Integration;
- Lens/Atelier/CKC intake;
- Loom;
- WP-1 Multi-Model Orchestration/Lifecycle/Telemetry;
- WP-12 Native Editors as an external editor authority and legacy Stage supersession boundary; any current editor integration is newly specified.

The exact dependency IDs must come from the current machine-readable contracts and registries at transaction time. No dependency is inferred from the old prose projections alone.

## Consolidated stub requirements

The surviving contract must:

1. remain `PRIMARY_MACHINE_READABLE_STUB`, `NON_EXECUTION_STUB`, and `STUB (NOT READY FOR DEV)`;
2. use the current Master Spec pointer/path, not stale `.GOV/roles_shared/SPEC_CURRENT.md` or v02.131/v02.150/v02.158 references;
3. describe the single large Stage WP and its 100-plus-microtask intent without pretending a final packet/refinement exists;
4. carry source lineage to all three original contracts and the source-disposition register;
5. state an explicit current disposition for each source item; only current-corpus requirements and later operator decisions define scope, rationale, constraints, non-goals, acceptance criteria, risks, mitigations, dependencies, blockers, and unresolved decisions;
6. clearly distinguish folded Stage-owned lanes from adjacent work packets and consumers;
7. reference the Stage planning workspace, research basis, active-WP integration/legacy-Stage-supersession analysis, and later approved Master Spec anchors;
8. retain the required refinement, red-team, research, signature, activation, taskboard, build-order, traceability, and validator gates;
9. resolve the current contradiction between `user_signature_required: false` and an activation step that requires a user signature, according to the repository's current activation authority;
10. remain non-executable until the later refinement and full packet are approved and validated.

## Coordinated authority surfaces

The consolidation must be one coordinated transaction across:

- the three machine-readable stub contracts and their projections;
- archive folders and supersession metadata;
- taskboard projection;
- build-order projection;
- work-packet traceability registry;
- source/spec/refinement links;
- any ID/dependency registry entries;
- validators or generated projections that reference either absorbed ID.

Partial consolidation is rejected because it would leave duplicate active intent and conflicting dependency edges.

## Acceptance checks

- Every `STAGE-PRES-*` source row has exactly one explicit reaffirmed, superseded, external-boundary, or operator-decision disposition; only reaffirmed outcomes map into current scope.
- The surviving stub contains no self-dependency and no dangling absorbed-stub dependency.
- Both absorbed contracts and projections are archived, discoverable by their original IDs, and point to the surviving stub.
- Repository searches find no active taskboard/build-order/traceability reference treating an absorbed ID as independently executable.
- Every external dependency and downstream consumer remains represented.
- The consolidated stub points only to the current approved Master Spec version after the Stage spec proposal is applied.
- Machine-readable contract validation, projection generation/checks, taskboard checks, build-order checks, traceability checks, and hard gates pass.
- A no-context model can explain what is current, what was superseded, what was folded only as lineage, what remains adjacent, and why implementation is still blocked.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| A current portability or ASR requirement disappears inside the large umbrella. | Keep named current lanes, source-lineage/disposition fields, lane-specific ACs, and automated current-requirement coverage. |
| Old built Stage code silently constrains the replacement. | Require a legacy-surface inventory, explicit removal/replacement/salvage/data-import disposition, and a proof scan showing no compatibility alias or old runtime authority remains. |
| Old IDs remain active in projections. | Transactional registry/projection scan and validator pass before claiming consolidation. |
| External dependencies are mistaken for folded scope. | Explicit ownership table and unchanged adjacent packet IDs. |
| Consolidation activates implementation prematurely. | Preserve non-execution/stub state and activation gates; no packet/refinement claim from this transaction. |
| Stale spec references survive. | Resolve through `SPEC_CURRENT.md` and the approved new Stage spec version, then search for old paths/versions. |
| Dirty active WP branches invalidate integration assumptions. | Treat current analyses as dated interface snapshots; re-open active packets and diffs before activation and merge planning. |

## Operator decision requested

Confirm this consolidation proposal as written, or identify changes. Confirmation authorizes the coordinated stub/archive/registry update through the repository workflow. It does not activate development.

</topic>
