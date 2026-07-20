---
file_id: stage-current-spec-reconciliation
file_kind: reference-spec-audit-and-reconciliation-plan
updated_at: "2026-07-19"
status: ready-for-operator-review
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-current-spec-authority-map" status="verified" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Current Stage specification authority

`.GOV/spec/SPEC_CURRENT.md` resolves authority to `master-spec-v02.201`. The current Stage topic is physically located in `.GOV/spec/master-spec-v02.201/spec-modules/11-shared-dev-platform-and-oss-foundations.md`, lines 1035-2669.

The Stage topic remains `Stage Spec v0.6 (Draft)`, dated 2026-02-19, and its own closeout text requires revalidation after v02.130. The current bundle is v02.201. Stage is therefore not production-current even before applying the operator's Servo-primary direction.

The bundle `INDEX.json` registers the physical file as module `11`, but does not independently index the contained Stage section. A future Stage rewrite must add a machine-resolvable Stage topic/anchor entry rather than relying on a large module scan.

## Cross-module authority that the rewrite must compile against

| Contract | Current authority |
|---|---|
| Native application boundary | `01-vision-and-context.md`: native Rust shell; controlled browser/App islands only. |
| Stage worksurface identity | `01-vision-and-context.md`: first-class worksurface bound to Workspace Graph and AI Job Model. |
| Canonical governance types | `02-system-architecture.md`: canonical AutomationLevel, GovernanceDecision, AutoSignature, and Flight Recorder families override imported aliases. |
| Canonical artifacts | `02-system-architecture.md`: structured `ArtifactHandle`, not a URI string. |
| Unified Tool Surface | `06-mechanical-integrations.md`: stable versioned tool IDs; WRITE/EXECUTE actions compile through AI Jobs. |
| Session correlation | `06-mechanical-integrations.md`: unified-tool `session_id` means `ModelSession.session_id`; it cannot mean a Stage browser session. |
| Model orchestration | `07-user-experience-and-development.md` and active WP-1: ModelSession/ModelLane execution, scheduling, provenance, and diagnostics. |
| Stage topical authority | `11-shared-dev-platform-and-oss-foundations.md`: legacy trust zones, browser workflows, Stage Apps/Bridge, capture/import, 3D, security, and test harness. |
| Capability registry | `11-shared-dev-platform-and-oss-foundations.md`: unknown capability IDs fail closed. |
| Feature/runtime registry | `12-end-of-file-appendices.md`: currently stale `TECH-TAURI`, `OPERATOR_ONLY`, `SQLITE_NOW_POSTGRES_READY`, and `Md*`-only primitive exposure. |

</topic>

<topic id="stage-current-spec-conflict-register" status="ready-for-review" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Conflict and remediation register

| Conflict ID | Verified defect | Required resolution in the proposal | Decision state |
|---|---|---|---|
| `STAGE-SPEC-001` | Stage is a stale v0.6 draft and names v02.130-era files/anchors. | Rewrite against v02.201 topical modules and update every Main Body, roadmap, Appendix, and traceability anchor. | required |
| `STAGE-SPEC-002` | Stage is not independently machine-resolvable in `INDEX.json`. | Add a stable Stage topic entry/range or split module that the bundle tooling can resolve and validate. | required |
| `STAGE-SPEC-003` | Native-Rust law conflicts with Tauri/Wry/Electron guidance and Appendix `TECH-TAURI`. | Define a native Rust `StageHost` with renderer adapters; remove Tauri/Electron as runtime authority and replace obsolete paths/technology rows. | operator direction already locks native/Servo direction; exact Chromium substrate remains open |
| `STAGE-SPEC-004` | Stage Bridge redefines `ArtifactHandle` as a URI string. | Use the canonical structured `ArtifactHandle`; URI is only one field. Reject parallel Stage-only artifact authority. | required |
| `STAGE-SPEC-005` | `stage.jobs.enqueue(job_kind)` conflicts with merge law treating `stage.*` values as protocol/profile IDs under `job_kind=workflow_run`. | Specify the exact compile-down envelope: `job_kind=workflow_run`, versioned Stage `protocol_id`/`profile_id`, idempotency key, capability decision, actor, and correlation fields. | required |
| `STAGE-SPEC-006` | Generic `session_id` is used for both Stage browser sessions and ModelSession. | Require distinct `model_session_id`, `stage_session_id`, `stage_tab_id`, `model_lane_run_id`, and generation-scoped engine IDs. | required |
| `STAGE-SPEC-007` | Stage App manifests use bridge method names as capabilities; Appendix registers no Stage tools. | Register capabilities and tool/method IDs separately. ToolGate checks a central capability ID; unknown IDs fail closed. Add versioned Stage tools and Appendix rows. | required |
| `STAGE-SPEC-008` | `artifact.clip` conflicts with topical `artifact.document`/DerivedContent. | Define the current canonical capture/selection representation from current requirements and supersede the legacy identifier; no alias is required solely because it existed. | current contract required |
| `STAGE-SPEC-009` | Topical and roadmap 3D protocol IDs disagree; one patchset ID exists only in roadmap. | Select current canonical IDs in topical authority and supersede conflicting legacy IDs; give every selected protocol an authoritative schema and acceptance test. | proposal required |
| `STAGE-SPEC-010` | Cold tabs and sessions lack typed schemas, state machines, authority, migrations, replay, retention, and concurrency. | Define PostgreSQL/EventLedger `StageWindow`, `StageFolder`, `StageTab`, `StageSession`, observation/action receipt, lifecycle, revision, retention, and restart contracts. | required |
| `STAGE-SPEC-011` | Model-operated browsing/testing is promised but has no model-visible tools or observation/action contract; Appendix says `OPERATOR_ONLY`. | Add BiDi-shaped engine-neutral browser tools, semantic snapshots, geometry/actionability, screenshots, network/console state, stable Stage IDs, receipts, postconditions, and operator takeover. Replace `OPERATOR_ONLY` with explicit model exposure and ToolGate posture. | required |
| `STAGE-SPEC-012` | ModelSession integration exists only for Prompt Playground. | Add Stage-to-Dexterity/ModelSession joins, ContextBundle/artifact handoff, DCC projection keys, control leases, cancellation/recovery, and promotion rules. | required; active WP-1 blockers remain a release gate |
| `STAGE-SPEC-013` | Stage interaction edges omit AI Job Model, Workflow Engine, Capability/Consent, EventLedger/Flight Recorder, ModelSession/Dexterity, ASR, Studio, and active editor/CKC consumers. | Publish a complete Stage ownership and interaction matrix with reciprocal consumer contracts. | required |
| `STAGE-SPEC-014` | Trusted HTML is a headline feature with no trust-zone definition. | Define artifact-origin classification, script/active-content posture, offline resolver, CSP, bridge denial, provenance banner, sanitization mode, and explicit re-enable policy. | required |
| `STAGE-SPEC-015` | Capture sends a URL to Archivist but does not define current authenticated page state, frames, dynamic DOM, scroll, canvas/media, or secret handling. | Split `current-renderer evidence capture` from `independent acquisition/archive`. Record capture facets and limitations; prohibit silent cookie copying. | required |
| `STAGE-SPEC-016` | Local web testing conflicts with the default localhost/private-network deny policy. | Add a separate development-test profile with explicit capability, loopback/host binding, target allowlist, TTL, visible state, and durable evidence. No global network-policy weakening. | required |
| `STAGE-SPEC-017` | Stage App hashing exists, but install/update/revoke/rollback, trust-root rotation, dependencies, compatibility, migrations, and crash isolation are missing. | Define a full package lifecycle and fail-closed compatibility negotiation. | required |
| `STAGE-SPEC-018` | Observability is generic; there are no Stage EventLedger state events, Stage diagnostics, process ownership, crash/hang receipts, UserManual, or three-tier outcomes. | Define EventLedger authority events plus Flight Recorder projection, internal diagnostics, Palmistry, ProcessOwnershipLedger integration, Argus projection, and code-truthful UserManual entries. | required |
| `STAGE-SPEC-019` | Security/resource budgets are placeholders and open questions. | Replace invented thresholds with target-hardware baselines, Stage journey suites, hostile fixtures, soak tests, and operator-approved promotion thresholds. | measurement required |
| `STAGE-SPEC-020` | Recommended components are unpinned and current references do not prove browser security/update posture. | Record exact selected versions/commits, SBOM, update cadence, advisories, rollback, and rejected alternatives. | research in progress |
| `STAGE-SPEC-021` | Stage-to-Studio handoff is one-sided and Stage-to-ASR is implicit. | Add reciprocal Studio and ASR contracts without absorbing those products into Stage. | required |

## Supersession and lineage rule

The rewrite makes the current Stage direction authoritative and supersedes all older Stage-specific requirements and implementations. Legacy trust-zone, capture/import, 3D, private-network, hostile-content, and security-harness material remains source evidence only; an outcome survives only when the current requirement register independently reaffirms it. Platform-WebView-specific knobs remain only as non-authoritative Chromium-bootstrap evidence where the selected substrate uses them.

</topic>

<topic id="stage-current-spec-proposed-structure" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Proposed Stage topic structure

The later Spec Proposal should replace the monolithic imported draft with independently indexed, cross-linked sections in this order:

1. Purpose, scope, non-goals, and ownership boundaries.
2. Native Stage worksurface and renderer strategy.
3. Typed Stage data model and multidimensional lifecycle.
4. Sessions, profiles, storage, credentials, retention, and portability.
5. Browser-engine adapter and capability negotiation.
6. External Web, trusted HTML, Stage App, and privileged-host trust zones.
7. Navigation, network, permission, download, privacy, and request policy.
8. Stage Apps package lifecycle and privileged Bridge compile-down.
9. Model operation through Dexterity, browser-control leases, and operator takeover.
10. Capture/import, ArtifactStore, Media Downloader, ASR lineage, and downstream intake.
11. Search, translation, Markdown/PDF export, Loom projection, and any newly specified current editor integration.
12. Diagnostics, UserManual, recovery, security harness, and acceptance gates.
13. Chromium bootstrap promotion/retirement and Servo promotion gates.
14. Canonical Appendix runtime, primitive, tool, capability, and interaction-edge rows.

The Stage section must be directly resolvable from the bundle index, and every roadmap/Appendix reference must target one of these topical anchors.

</topic>
