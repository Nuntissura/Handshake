---
file_id: mt-007-008-ckc-spec-taskboard-inventory
file_kind: ckc_spec_taskboard_inventory
updated_at: 2026-05-16
wp_id: WP-1-Atelier-Lens-Consolidation-v1
mt_ids:
  - MT-007
  - MT-008
status: complete
---

# MT-007/008 CKC Spec And Taskboard Inventory

<topic id="scope-and-guardrails" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Scope

Inventory source:
- `D:\Projects\LLM projects\CastKit-Codex\CKC_GOV\spec\CastKit_Codex_Spec_v00.075.md`
- `D:\Projects\LLM projects\CastKit-Codex\CKC_GOV\taskboard\TASK_BOARD.md`

This artifact covers:
- MT-007: CKC spec headings and requirement sections.
- MT-008: CKC taskboard statuses and delivery signals.

## Guardrails

- CKC is source evidence only for Handshake fold-in.
- Do not create CKC rebuild stubs from this inventory.
- Do not modify CKC files from this inventory lane.
- SQLite is absolutely rejected for Handshake: no runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths may use SQLite in Handshake.
- CKC SQLite references are recorded only as source evidence of CKC history or CKC technical debt; they are not accepted implementation patterns for Handshake.

</topic>

<topic id="source-anchors" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Primary Source Anchors

| Source | Anchor | Signal |
|---|---:|---|
| CKC spec v00.075 | lines 11-13 | Current spec entry records WP-0122..WP-0137 review-batch hardening and code-truth consistency across code, manual, WPs, taskboard, and tests. |
| CKC spec v00.075 | lines 15-20 | Governance and verification rules require LLM visibility, parallel-operation audit, viewport captures, current value/reset visibility, and sample-corpus use. |
| CKC spec v00.075 | lines 21-35 | Intake, Inbox, Collections, contact sheets, sidecars, PoseKit contexts, OpenPose sidecars, parallel editing, and recoverable deletion are current CKC behavior. |
| CKC spec v00.075 | lines 37-46 | Manual/automation hardening and PostgreSQL-compatible regression tests are part of the latest hardening gate. |
| CKC spec v00.075 | lines 422-432 | Non-negotiables include byte-for-byte user text preservation, adult/explicit fields first-class, template integrity, same-line descriptors, minimal UI, and 2-panel/3-panel layouts. |
| CKC spec v00.075 | lines 502-517 | PostgreSQL is CKC's default/current provider; SQLite remains only as CKC legacy/test fallback. |
| CKC spec v00.075 | lines 519-554 | CKC has automation/debugger IPC, internal manual formats, multi-agent sessions/leases, hidden background startup, renderer/backend commands, and file capture. |
| CKC spec v00.075 | lines 556-569 | Intake Sorter supports folder-only and linked CKC profile modes, pending review, and linked metadata writeback. |
| CKC spec v00.075 | lines 571-579 | CKC libraryRoot contains legacy/test SQLite file path only when explicitly configured; PostgreSQL lives outside libraryRoot. |
| CKC spec v00.075 | lines 1036-1068 | Next-generation global search requirements mention SQLite FTS5; this is CKC evidence only and must translate to non-SQLite Handshake search/indexing. |
| CKC taskboard | lines 5-9 | Taskboard is CKC authoritative work status and must be read with CKC governance surfaces; no product-doc mirror. |
| CKC taskboard | lines 137-155 | WP-0119 through WP-0137 show PostgreSQL-first/SQLite-removal planning, review-batch work, one blocked PoseKit interaction item, and raster contact sheet planning. |
| CKC taskboard | lines 157-184 | Current focus records WP-0137 in REVIEW, full test pass, GUI/sample-corpus review evidence still pending, plus open findings and deferred gates. |

## Heading Inventory Summary

The CKC spec is a mixed source-of-truth document: recent changelog entries, normative sections, roadmap requirements, and an appendix session dump. Relevant headings for Handshake fold-in:
- `v00.075 - WP-0122..WP-0137 review-batch hardening`
- `Governance And Verification`
- `Intake, Inbox, Collections, Contact Sheets, And Sidecars`
- `PoseKit Workbench And OpenPose Artifacts`
- `Parallel Editing And Recoverable Deletion`
- `Automation And Manual Hardening`
- `Non-negotiables`
- `Decisions`
- `Implementation mapping`
- `Data layout`
- `LibraryRoot diagnostics`
- `Media pane chrome`
- `Docs mode`
- `Sheet ingest/merge + versioning`
- `High-ROI roadmap`
- `Next-Generation Features`
- `Appendix A - SESSION_DUMP_2026-02-10`

</topic>

<topic id="capability-clusters" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Capability Clusters

| Cluster | CKC Evidence | Requirement Signals | Handshake Fold-in Implication |
|---|---|---|---|
| Governance and verification | Spec lines 15-20; taskboard lines 5-9, 170-176 | LLM visibility, parallel-operation audit, visual captures, sample corpus, research-first WP planning, build rules. | Fold into Handshake as governance requirements for agent-readable receipts, visual verification evidence, source-backed research summaries, and taskboard/refinement traceability. |
| Automation/manual/debugger | Spec lines 37-46, 519-554; taskboard lines 155, 157-158 | Manual-command-dispatch consistency, automation command map, sessions, leases, logs, captures, background-safe mode. | Translate to Handshake diagnostics and operator/model control surfaces. Preserve the consistency-test concept; do not import CKC APIs as product shape. |
| Intake/Inbox/import gate | Spec lines 21-25, 556-569; taskboard lines 142-145, 172 | Persistent batches, loose/linked classification, pending lifecycle, filesystem health, hidden sidecar/deleted rows. | Useful for Handshake Atelier/Lens intake lanes, review queues, and recoverability. Must use Handshake data contracts and PostgreSQL/EventLedger storage only. |
| Collections/contact sheets | Spec lines 21-25; taskboard lines 146-147, 154, 173 | Collections with notes/tags/links, contact-sheet SVG+manifest, raster export planned, no-space artifact paths. | Fold as evidence for collection artifacts and preview/export surfaces; preserve manifest/provenance patterns, translate storage/export to Handshake ArtifactStore. |
| PoseKit/OpenPose/workbench | Spec lines 27-30; taskboard lines 149-151, 162, 165-167 | Blank/single/collection contexts, `getPoseKitState`, deterministic no-space sidecars, blocked calibration/history interaction pass. | Relevant to Atelier/Lens visual/pose workbench. Treat CKC PoseKit as evidence; create Handshake-native interaction WPs only after consolidation/research. |
| Parallel editing/recoverable deletion | Spec lines 32-35; taskboard lines 152-153, 174 | PostgreSQL source of truth, sessions/leases, optimistic revisions, append-only product events, limited CRDT merge shapes, archive/restore impact checks. | Strong fit for Handshake EventLedger and CRDT/workspace contracts. Reuse policy shape, not CKC table/API details. |
| Storage/runtime | Spec lines 502-517, 571-579; taskboard lines 137, 169 | PostgreSQL-first default/current provider; SQLite only as CKC legacy/test fallback; SQLite removal/quarantine planned. | Handshake must go further: SQLite is forbidden everywhere, including tests, fixtures, mocks, examples, caches, compatibility adapters, temporary harnesses, imports, exports, and product paths. |
| Product UX baseline | Spec lines 422-432, 655-727 | Byte-for-byte user text preservation, explicit fields first-class, no silent template drops, minimal UI, 2-panel/3-panel layouts, media pane chrome, docs mode, sheet merge/versioning. | Relevant to Atelier/Lens usability and data preservation. Translate into Handshake UX acceptance criteria and source-preservation tests. |

</topic>

<topic id="taskboard-state-signals" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Taskboard State Signals

Parsed CKC taskboard work-packet rows: 135.

| Status | Count | Interpretation |
|---|---:|---|
| DONE | 113 | Most historical CKC features are marked complete and can be used as evidence of shipped behavior or intended delivered behavior. |
| REVIEW | 14 | WP-0122..WP-0137 review-batch items are implemented but still in review state; use as strong evidence, not as fully closed CKC precedent. |
| BLOCKED | 1 | WP-0133 PoseKit calibration, markers, and history has unresolved interaction work. |
| CONCEPT (future; not current) | 1 | WP-0118 prompt-response matrix is parked concept work. |
| PLANNED variants | 6 | Planned or low-priority future work remains, including WP-0114, WP-0116, WP-0117, WP-0119, and WP-0136. |

## Delivery Signals

- WP-0137 is in REVIEW with product/manual dispatch drift fixed, PostgreSQL-compatible backend regression, current spec v00.075, and full `npm test` pass of 206 passed and 1 skipped (taskboard lines 157-158).
- WP-0122..WP-0135 implementation pass is complete but review evidence remains before DONE promotion (taskboard line 172).
- Raster contact sheet export is planned as WP-0136, not shipped in current CKC evidence (taskboard lines 154, 173).
- PostgreSQL-first testing and SQLite removal/quarantine are planned in CKC WP-0119 and reinforced as a current rule (taskboard lines 137, 169).
- Open minor findings remain for React 19 Save button click behavior and occluded operator-mode captures (taskboard lines 178-180).
- Deferred validation remains for packaged smoke, NAS mirror backup, DB-backed suite verification against live PostgreSQL, and validation pass for WP-0092 through WP-0095 (taskboard lines 181-183).

</topic>

<topic id="unresolved-work-and-risks" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Unresolved Work

- CKC review-batch GUI/sample-corpus captures still need review evidence before DONE promotion.
- WP-0133 deeper PoseKit interaction model remains blocked: draggable calibration overlay, missing-marker placement flow, 3D/live split editing, and forked history.
- WP-0136 raster contact sheet export is planned and not yet delivered.
- WP-0119 indicates CKC still has SQLite remediation work outstanding.
- Historical roadmap/spec sections still contain SQLite-centric implementation language, especially global search FTS5 and planned cache/index fields.
- Some older delivery gates remain deferred or pending, including packaged smoke and DB-backed verification against a live PostgreSQL container.

## Risks And Mitigations

| Risk | Mitigation For Handshake |
|---|---|
| CKC mixes delivered, review, blocked, planned, and appendix/history requirements in one spec. | Treat CKC as evidence; require Handshake work packets to cite taskboard status and classify evidence maturity before implementation. |
| CKC contains SQLite language in current and historical sections. | Enforce the absolute SQLite rejection in Handshake packet acceptance criteria and validation grep checks. |
| CKC automation APIs are product-specific. | Translate capability intent into Handshake-native diagnostics, EventLedger receipts, and governed command surfaces. |
| Review-batch evidence is not all DONE. | Use review-batch items as strong but non-final evidence; require fresh Handshake verification rather than importing CKC closure state. |
| PoseKit calibration/history is blocked in CKC. | Split Handshake pose/workbench interaction scope into dedicated research-backed work rather than folding blocked CKC behavior silently. |

</topic>

<topic id="handshake-fold-in-implications" status="complete" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Fold-in Implications

1. Preserve CKC capability intent, not CKC implementation shape. CKC provides evidence for intake, collections, contact sheets, sidecars, PoseKit contexts, automation/manual consistency, and parallel editing policies.
2. Handshake should translate storage and collaboration to PostgreSQL, EventLedger, ArtifactStore, CRDT/workspace contracts, and existing governed runtime boundaries.
3. The SQLite rejection must be a hard gate in downstream Handshake packets and validators. CKC's SQLite fallback, FTS5, fixtures, migration language, and compatibility paths must not be carried forward.
4. CKC taskboard statuses should drive evidence maturity: DONE is historical delivery evidence, REVIEW is implemented-but-not-final evidence, BLOCKED/PLANNED/CONCEPT are backlog or risk signals.
5. Do not create CKC rebuild stubs from this inventory. Downstream stubs should be Handshake-native only after consolidation and research closure.
6. High-ROI additions while this area is already touched: add a Handshake no-SQLite validator, add evidence-maturity fields to consolidation JSON, and require each future fold-in packet to cite CKC source anchors plus Handshake ownership boundaries.

</topic>
