---
file_id: stage-master-spec-overhaul-proposal-v1
file_kind: spec-proposal
updated_at: "2026-07-19"
status: deferred-requires-regeneration-from-hardened-corpus
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-master-spec-overhaul-proposal" status="deferred-requires-regeneration-from-hardened-corpus" version="v1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Spec Proposal: Stage topic overhaul

## Confirmation gate

This early proposal is retained as historical planning evidence but is not ready for confirmation. The operator deferred Master Spec authoring until full-feature planning/research is complete and has since locked that the current Stage direction supersedes all older Stage-specific requirements and implementations. Regenerate this proposal from files `14` through `20`, all machine-readable registers, and future operator decisions before using the required `CX-405`/`CX-406` confirmation gate. No Master Spec file has been changed.

## What changes

Replace the stale imported `Stage Spec v0.6 (Draft)` material currently embedded in module 11 with an independently indexed, cross-linked Stage specification in the next Master Spec version. Preserve its still-valid requirements while resolving conflicts against the current v02.201 architecture and the operator-locked Stage direction.

The new Stage topic will define:

1. product purpose, supported operator/model workflows, scope, and Stage/Studio boundary;
2. native-shell and canonical Stage-domain architecture;
3. browser-engine adapter and capability manifest;
4. WebView2 minimal Chromium bootstrap, optional Chrome-for-Testing validation worker, conditional CEF escalation, and Servo strategic adapter;
5. platform- and capability-qualified Servo promotion, including the current Windows arbitrary-web sandbox block;
6. durable windows, sessions, tabs, attachments, lifecycle facets, lazy restore, and 3,000-plus-tab resource scheduling;
7. external-web, Stage-App, trusted-host, sanitized-capture, profile, network, and automation trust boundaries;
8. WebDriver-BiDi-shaped observation/action protocol, durable receipts, operator takeover, reconciliation, and no-focus-theft behavior;
9. capture/import, artifact portability, Downloader handoff, ASR transcript lineage, PDF/media/3D validation, and archive semantics;
10. Loom/tab-organization, search, translation, Markdown/PDF export, Lens/Atelier/CKC intake, and UserManual requirements;
11. diagnostics, Argus/Flight-Recorder/EventLedger integration, compatibility corpus, red team, promotion, rollback, and production acceptance gates;
12. explicit integration ownership with active WP-1, WP-12, and CKC work.

## Why

- The current Stage text is dated 2026-02-19, labels itself `v0.6 (Draft)`, and says it must be revalidated after a much older Master Spec version.
- Stage is not independently machine-resolvable from the current bundle index.
- The current text mixes native Rust authority with Tauri/Wry/Electron-era terms and does not reflect the operator-locked minimal-Chromium/strategic-Servo direction.
- It conflicts with current structured ArtifactHandle, workflow-run, session, capability/method, artifact, 3D, persistence, and diagnostic contracts.
- It does not define a model-control protocol, high-volume tab lifecycle, production persistence, process recovery, Stage/WP-1 integration, current-renderer versus archival capture, or the required operator/model manual.
- Two Stage-owned stubs add media-portability and ASR-lineage requirements that belong inside one coherent Stage contract.

## Architectural impacts

| Area | Proposed impact |
|---|---|
| Spec topology | Stage becomes an independent indexed module or module group with stable cross-links, rather than a stale imported subsection hidden inside shared platform foundations. |
| Native UI | Stage remains a native egui/winit product surface. Browser engines are native-host adapters; no Tauri/Electron product architecture is introduced. |
| Engine strategy | Direct WebView2 is the Windows bootstrap; Chromium stays minimal. Servo remains strategic but promotion is capability/platform gated. Arbitrary-web Windows Servo is blocked until sandbox proof exists. |
| Domain model | Revisioned `StageWindow`, `StageSession`, `StageTab`, `BrowserAttachment`, action/observation receipts, capture, and bulk-run records replace engine-native or string-only authority. |
| Lifecycle | Persistent record, runtime attachment, page lifecycle, navigation, and control lease are independent dimensions. |
| Automation | WebDriver BiDi shapes the public command/event model; CDP and engine APIs remain private adapters. Pre-side-effect intents and uncertain-outcome reconciliation are normative. |
| Security | Hostile external web never receives the privileged bridge. Stage Apps, sanitized captures, profiles, networks, and automation endpoints receive explicit trust and isolation rules. |
| Persistence | PostgreSQL/Storage Trait/EventLedger integration becomes normative; browser profile directories remain runtime materialization, not authority. |
| Artifacts | Current structured `ArtifactHandle` and ArtifactStore authority are reused. `artifact.document`/derived-content forms replace the legacy unaligned `artifact.clip` assumption. |
| Workflow | `workflow_run` plus profile/protocol merge replaces the old `stage.jobs.enqueue(job_kind)` authority. Stage tool/method IDs remain distinct from granted capabilities. |
| WP-1 | Stage consumes model lanes, ToolGate, process ownership/recovery, promotion, Flight Recorder, EventLedger, and Argus. It does not duplicate model orchestration. |
| WP-12 | All existing WP-12 Stage-specific UI/API/storage/routes/adapters/connectors/schemas/mockups are superseded. Any current editor integration is specified anew; real operator data receives a one-way import only if present. |
| CKC | CKC remains a downstream governed-artifact/lineage consumer; pose/character domain logic is not folded into Stage. |
| Stage/Studio | The established browsing/capture versus editing/authoring boundary remains, with typed route-to-editor/embed-back handoffs. |
| Operations | Runtime/adaptor/build versions, process/profile mapping, receipts, screenshots, health, crash/restart, rollback, and no-context diagnostic paths become required. |

## Current-direction and source-lineage commitments

The rewrite will preserve, not silently narrow:

- every item from the three Stage stubs receives an explicit reaffirmed-or-superseded disposition; no legacy item survives solely because it existed or was built;
- current Stage/Studio, trust-zone, session, bridge, jobs/capture/import, 3D, network, and hostile-content requirements come from the current approved register rather than legacy compatibility;
- the operator-locked workflows and UI requirements recorded in this workspace;
- separate adjacent ownership for Downloader, Loom bridge, ASR execution, Video Archive, Artifact System, Storage Trait, Lens/Atelier intake, WP-1 orchestration, WP-12 editors, and CKC.

The requirement-level lineage/dispositions are recorded in `01-source-preservation-register.md`; conflicts are recorded in `11-current-spec-reconciliation.md`; active-WP boundaries and legacy Stage removal are recorded in `12-active-wp-compatibility.md`.

## Proposed Stage spec shape

1. Stage purpose, terminology, and boundary.
2. Operator and model workflows.
3. Native shell and domain architecture.
4. Engine adapter and capability contract.
5. Chromium bootstrap.
6. Servo strategic implementation and promotion.
7. Windows/platform support matrix.
8. Security, trust zones, request policy, profiles, and bridge.
9. Durable data model and lifecycle.
10. Agent control, receipts, takeover, replay, and WP-1 integration.
11. Capture/import/archive and ArtifactStore lineage.
12. Media portability, Downloader, ASR, and downstream consumers.
13. High-volume tabs, organization, Loom, search, translation, and export.
14. Stage/Studio and any newly specified current editor integration.
15. Diagnostics, UserManual, testing, production gates, rollback, and red team.
16. Dependency, traceability, and supersession map.

## Explicit removals or replacements

- Remove Tauri/Wry/Electron as normative product architecture. Wry may appear only as a rejected/prototype bootstrap option.
- Replace string `ArtifactHandle` examples with the current structured contract.
- Replace `stage.jobs.enqueue(job_kind)` authority with canonical workflow-run/profile/protocol semantics.
- Rename/disambiguate Stage browser session versus WP-1 model session.
- Separate capability grants from tool/method identifiers.
- Replace or map `artifact.clip` to current document/derived-content artifact forms.
- Normalize 3D IDs and validation references to current contracts.
- Remove the implication that model operation is `OPERATOR_ONLY`; define ToolGate-governed model actions and operator takeover.
- Replace ambiguous `trusted HTML`, capture semantics, and local/private-network exceptions with typed policies.
- Replace placeholder resource budgets with measurement procedures and target-hardware acceptance records.

## Files affected after approval

The exact new spec version and module paths must be produced by the repository's spec-versioning workflow. Expected authority surfaces include:

- a new version derived from `.GOV/spec/master-spec-v02.201/`;
- the bundle `INDEX.md` and any module index/links;
- the Stage module(s) and cross-links from modules that own artifacts, workflow, UI, data, diagnostics, and security;
- `.GOV/spec/SPEC_CURRENT.md` only after the new bundle validates;
- downstream traceability/refinement/packet references only through the activation/consolidation workflow.

## Validation required after approval

- source-range and old-to-new anchor map;
- explicit current dispositions for all `STAGE-PRES-*` entries and proof that legacy Stage runtime surfaces are absent;
- bundle link/index validation and spec validators;
- terminology check for StageSession/ModelSession, capabilities/tool IDs, ArtifactHandle, workflow_run, and 3D IDs;
- cross-module ownership and dependency review;
- no stale `.GOV/roles_shared/SPEC_CURRENT.md` references;
- no Tauri/Electron/Wry normative architecture leakage;
- diff proof that v02.201 remains untouched and the new version is the only promoted authority;
- operator review of the complete generated diff before the final pointer update.

## Operator decision requested

Confirm this Spec Proposal as written, or identify adjustments. Confirmation authorizes drafting the new Master Spec version through the repository workflow; it does not by itself activate the Stage WP or approve implementation.

</topic>
