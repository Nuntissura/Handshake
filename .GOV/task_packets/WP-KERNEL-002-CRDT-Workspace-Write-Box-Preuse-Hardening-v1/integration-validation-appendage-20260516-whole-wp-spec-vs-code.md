---
file_id: integration-validation-appendage-20260516-whole-wp-spec-vs-code
file_kind: validation_appendage
updated_at: 2026-05-16T18:10:00Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
candidate_worktree: D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: 208e5503
baseline_main_sha: e11ba597
spec_target: .GOV/spec/master-spec-v02.185/indexed-spec-manifest.json
spec_target_sha1: 29ae893608ccb3d9ba2bd9fc84a3eca8887de295
mt_batch_verdict: PASS
whole_wp_master_spec_validation: PARTIAL_DCC_DEPTH_FAIL
overall_merge_readiness: NOT_READY_FOR_MERGE_DCC_DEPTH_REMEDIATION_REQUIRED
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T18:10:00Z" ingestable="true" summary="Whole-WP code-vs-Master-Spec validation against v02.185 anchors for WP-KERNEL-002.">

Handshake (Product): whole-WP coding-vs-current-Master-Spec validation across all backend kernel surfaces (KernelActionCatalog, WriteBox family, DirectEditGuard, CRDT/PromotionBridge, CRDT storage) and DCC frontend projection viewer. Backend implementation is spec-compliant. DCC frontend renders all required projection types but with insufficient field depth on Action Catalog Viewer, Write Box Queue, and Promotion Preview vs. Section 10.11.5.28's explicit "MUST show by X, Y, Z" lists.

Repo Governance: Operator-waived governance paperwork. No merge, sync, staging, commit, or push performed. This is an advisory verdict pending Operator decision on DCC depth remediation.

</topic>

<topic id="spec-anchors-tested" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T18:10:00Z" ingestable="true" summary="Spec anchors evaluated for this WP.">

Spec target resolved: `.GOV/spec/SPEC_CURRENT.md` -> `.GOV/spec/master-spec-v02.185/indexed-spec-manifest.json` (v02.185, sha1 `29ae893608ccb3d9ba2bd9fc84a3eca8887de295`).

Anchors tested:
- A1: `02-system-architecture.md` Section 2.3.13.10 "Kernel V1 CRDT Workspace, Write Box, and Promotion Bridge [ADD v02.185]"
- A2: `02-system-architecture.md` changelog rows confirming v02.185 primitive set
- A3: `03-local-first-infrastructure.md` "Kernel V1 CRDT workspace addendum [ADD v02.185]"
- A4: `10-product-surfaces.md` Section 10.11.5.28 "Kernel Action Catalog and Write Box Projections [ADD v02.185]"
- A5: `12-end-of-file-appendices.md` FEAT-KERNEL-WORKSPACE-WRITE-BOX and the 8 v02.185 primitives

</topic>

<topic id="backend-verdicts" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T18:10:00Z" ingestable="true" summary="Backend clause-by-clause verdicts against Section 2.3.13.10 and the CRDT addendum.">

BACKEND_VERDICT: PASS.

R1 — KernelActionCatalogV1 contract: **MET**.
Evidence:
- `src/backend/handshake_core/src/kernel/action_catalog.rs:49-54` declares `KernelActionCatalogV1` with `schema_id: "hsk.kernel_action_catalog@1"`, `catalog_id`, `version`, `actions`.
- `KernelCatalogActionV1` (line 33-46) carries every required field: `action_id`, `input_schema_id` + `result_schema_id` (schema version anchored), `role_eligibility` (actor eligibility), `capability_requirements` + `approval_posture` (capability/approval posture), `authority_effect` (target authority class), `validation_hooks` (validation checks), `promotion_path` with `event_kind`/`receipt_kind` (resulting event/receipt type), `dcc_preview` (preview behavior).
- Idempotency key carried at the action envelope (`action_envelope.rs:65, 79, 144`) and write-box replay (`write_boxes.rs:66`).
- `kernel002_action_catalog()` (line 75-133) enumerates 50 catalog actions covering catalog view, CRDT propose patch, write-box promote, mirror advisory capture/normalize, direct-edit deny, runtime-truth projection, workflow transition preview, governance overlay/coordination/lifecycle, Postgres residual, locus work tracking, DCC viewer/registry, role mailbox suite (contract/loop/triage/claim/handoff/inbox), FEMS suite, role turn isolation, work profiles, local-first MCP posture, git decision gate, anti-pattern registry, governance pack instantiation, session spawn DCC/distillation, screenshot capture project/execute, visual debugging loop, markdown mirror sync drift, task/work-packet/microtask contracts, local model microtask loop, generated-doc status projection, coder/validator handoff and verdict mediation, remediation work generation, MT loop scheduler, MT validation work graph.
- `validate_kernel_action_catalog` enforces uniqueness and required fields (line 135).

R2 — WriteBoxV1 family: **MET**.
Evidence:
- `src/backend/handshake_core/src/kernel/write_boxes.rs:100-122` `WriteBoxCommon` carries every required field: `write_box_id` (stable), `workspace_id`, `owner` (actor_id + actor_kind + role_id), `crdt_site_id`, `target_refs` (vec of `WriteBoxTargetRef` with authority_class), `base_snapshot_refs` (state vector source), `intent_summary`, `operation_payload_refs` (vec with sha256 provenance), `schema_version`, `validation_status` (validation state + check_ids), `denial_receipt_refs`, `promotion_receipt_refs`, `replay_metadata` (replay plan ref + replay order key + idempotency key + source event refs).
- 9 typed boxes (`DraftBox`, `CRDTWorkspaceBox`, `ProposalBox`, `PatchBox`, `ArtifactBox`, `MirrorAdvisoryBox`, `MemoryBox`, `ExecutionBox`, `PromotionBox`) on lines 124-179 wrap `WriteBoxCommon`. `CRDTWorkspaceBox` adds explicit `state_vector` and `update_refs` (line 131-135). `PromotionBox` adds `promotion_target_ref` and `event_ledger_ref` (line 174-179).
- `kernel002_write_box_schema_family()` (line 198-277) registers all 9 kinds with allowed_transitions, authority_effect, required_evidence_refs, validation_requirements, projection_rules. `validate_write_box_schema_family` enforces presence (line 279-319).

R3 — Direct edit denial: **MET**.
Evidence:
- `src/backend/handshake_core/src/kernel/direct_edit_guard.rs:56-68` `WriteBoxDirectEditDeniedV1` carries every required field: `actor` (KernelActorRef), `target` (target_ref + target_class + authority_class), `attempted_action`, `denial_reason`, `recovery_instruction`, `ui_response_ref`, `api_response_ref`, plus `receipt_refs` and `event_ledger_refs`.
- `guard_direct_edit_attempt` (line 100-145) is real product code, routing each `DirectEditTargetClass` to a deny/wrap/allow decision. AuthorityArtifact denies; CRDT/GeneratedFile/GeneratedMirror/RoleMailboxReply/GitAction wrap into lawful catalog actions; ProductCode allows.
- `run_direct_edit_regression_harness` (line 147-) exercises real product path on a set of `DirectEditRegressionCaseV1` and reports `unguarded_case_ids` to surface gaps.

R4 — CRDT-to-EventLedger promotion bridge: **MET**.
Evidence:
- `src/backend/handshake_core/src/kernel/crdt/promotion_bridge.rs:118-131` `bridge_crdt_state_to_promotion` reads validated write box (`validation_report.promotion_allowed && decision == Allowed`), computes state hash, and routes accepted vs rejected.
- `accepted_bridge_result` (line 133-187) emits both `KernelCrdtPromotionRequestedV1` and `KernelCrdtPromotionAcceptedV1` event mappings with distinct idempotency keys (`promotion_idempotency_key(bridge_id, "requested"|"accepted")`).
- `rejected_bridge_result` (line 189-225) emits `KernelCrdtPromotionRequestedV1` + `KernelCrdtPromotionRejectedV1`.
- `promote_crdt_state_through_event_ledger` (line 227-290) atomically appends the request + decision event pair to Postgres EventLedger via `db.append_kernel_event_pair_atomic_with_causation`. Storage failure produces typed `CrdtPromotionFailureReceiptV1` with replay instructions.
- `required_crdt_promotion_failure_receipts` (line 292-333) enumerates 6 standard failure receipts: duplicate_promotion_request, stale_state_vector, simultaneous_operator_model_promotion, validation_failed_after_merge, postgres_write_failed, projection_rebuild_failed, each replayable with idempotency key.
- CRDT merge does NOT mutate EventLedger directly — bridge is the only path; this matches spec.

R5 — CRDT storage (Module 03 addendum): **MET**.
Evidence:
- `src/backend/handshake_core/migrations/0020_kernel_crdt_storage.sql:4-26` defines Postgres `kernel_crdt_updates` table with `update_seq` (ordered replay), `update_sha256`, `state_vector_before/after`, `replay_metadata_json`, `event_ledger_stream_id`, `event_ledger_event_id` FK to `kernel_event_ledger(event_id)`, `storage_authority`.
- `kernel_crdt_snapshots` table (line 43-62) snapshot-safe with `covered_update_seq`, `state_vector`, `snapshot_sha256`, `promotion_evidence_update_ids` (joinable to write-box/promotion ids), `event_ledger_event_id` FK.
- Unique indexes prevent duplicate `update_seq` per crdt_document (line 28-29) and unique snapshot per event (line 64-65). Replay index (line 37-38) supports ordered restart replay.
- Storage is Postgres-backed, not SQLite — matches addendum "MUST NOT become a hidden SQLite authority path".

R6 — Eight v02.185 primitives present: **MET**.
- PRIM-KernelActionCatalogV1 → `KernelActionCatalogV1` (action_catalog.rs:49)
- PRIM-KernelActionDescriptorV1 → `KernelCatalogActionV1` (action_catalog.rs:33) — descriptor for each catalog entry
- PRIM-WriteBoxV1 → `WriteBoxCommon` + 9 typed boxes (write_boxes.rs:100-179)
- PRIM-WriteBoxDirectEditDeniedV1 → `WriteBoxDirectEditDeniedV1` (direct_edit_guard.rs:56)
- PRIM-WriteBoxPromotionRequestV1 → `CrdtPromotionGateInputV1` + KernelCrdtPromotionRequested event mapping (promotion_bridge.rs:54, 172)
- PRIM-WriteBoxPromotionReceiptV1 → `CrdtPromotionBridgeResultV1` + `CrdtPromotionFailureReceiptV1` (promotion_bridge.rs:89, 79)
- PRIM-CrdtWorkspaceDraftV1 → `CrdtUpdateRecordV1` (crdt/persistence.rs:29) + `CRDTWorkspaceBox` + `kernel_crdt_updates` table
- PRIM-CrdtWorkspaceSnapshotV1 → `CrdtSnapshotRecordV1` (crdt/snapshot.rs:16) + `kernel_crdt_snapshots` table

Field-level naming differs from the literal spec primitive names (e.g., `KernelCatalogActionV1` vs `KernelActionDescriptorV1`); the spec sets the contract (fields + behavior), not the literal Rust type name, and every required field is present.

</topic>

<topic id="dcc-frontend-verdicts" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T18:10:00Z" ingestable="true" summary="DCC frontend verdicts against Section 10.11.5.28.">

DCC_FRONTEND_VERDICT: PARTIAL.

R1 — Typed product projections (not raw transcripts): **MET**.
- `app/src/lib/api.ts:424-449` `KernelDccProjectionSurfaceV1` exposes 16 typed projection rows/collections.
- `KernelDccProjectionView.tsx` renders each as discrete tables, never raw JSON.
- Backend type mirror in `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs:68-199` with schema validation in tests.

R2 — Action catalog viewer: **PARTIAL**.
- MET: lists `KernelActionCatalogV1` action ids (`KernelDccProjectionView.tsx:181-183`, `catalog_action_refs` field).
- MISSING in UI: target authority class per row, input schema version per row, actor eligibility, approval/capability requirements matrix, preview behavior flag, allowed output receipt types. Spec text: "list ... by stable action id, target authority class, input schema version, actor eligibility, approval or capability requirements, preview behavior, and allowed output receipt types."
- Backend types contain these fields; the projection contract `catalog_action_refs: string[]` flattens to id only — depth must be expanded in both the projection contract and the React component.

R3 — Write box queue: **PARTIAL**.
- MET: shows write_box_id, actor, CRDT site id (work_id), target_refs, validation_state, lifecycle_state (`KernelDccProjectionView.tsx:282-305`).
- MISSING in UI: denial receipt refs (type field `api.ts:331 denial_receipt_refs` exists but not rendered), promotion receipt refs (`api.ts:332`), linked EventLedger events when promoted (no EventLedger ref column in table), stale-state-vector posture (no per-row freshness flag — only global badge section).

R4 — Direct-edit denial view: **MET**.
- Evidence: `KernelDccProjectionView.tsx:307-331` renders denial_id, attempted_actor, target, action, recovery_instruction.
- `DccDirectEditDenialRowV1` (api.ts:336-347) carries all required fields.
- Minor: spec also mentions "whether the blocked edit can be normalized into an advisory write box"; the recovery_instruction text covers this loosely but no explicit `normalization_eligible` flag exists. Treated as acceptable since recovery_instruction is the surface.

R5 — Promotion preview: **PARTIAL**.
- MET: shows preview_id, work_id, write_box_id, promotion_target_ref, request_event_ref (`KernelDccProjectionView.tsx:333-357`).
- MISSING in UI: current state vector per preview row, validation checks summary, idempotency key (no `idempotency_key` field in `DccPromotionPreviewRowV1`), expected EventLedger event types list, stale/duplicate risk indicator, accepted_event_ref/rejected_event_ref (fields exist on type `api.ts:356-357` but not rendered).

R6 — Projection freshness badges: **MET**.
- Evidence: `KernelDccProjectionView.tsx:359-372` renders source_projection_id, state_vector, stale boolean with CSS `fresh|stale` class. Stable IDs via `data-stable-id`.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T18:10:00Z" ingestable="true" summary="Single Kernel Builder remediation plan combining all current gaps.">

Kernel Builder remediation plan (single combined plan for all open items across MT batch + whole-WP spec validation):

1. **DCC-R2 — Action Catalog Viewer field depth (Master Spec 10.11.5.28):**
   Replace flat `catalog_action_refs: string[]` with a typed row collection. Add `DccCatalogActionRowV1` to `app/src/lib/api.ts` carrying `{ action_id, target_authority_class, input_schema_id, actor_eligibility: string[], capability_requirements: string[], approval_posture, preview_behavior_summary, result_schema_id }`. Project these from `kernel002_action_catalog()` via the backend DCC surface in `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs`. Render the new fields as columns or an expandable detail row in `app/src/components/KernelDccProjectionView.tsx`. Add `app/src/components/KernelDccProjectionView.test.tsx` assertions for each new column.

2. **DCC-R3 — Write Box Queue field depth (Master Spec 10.11.5.28):**
   Extend the rendered Write Box Queue table in `KernelDccProjectionView.tsx:282-305` to include `denial_receipt_refs`, `promotion_receipt_refs`, `event_ledger_event_refs` (new field on `DccWriteBoxQueueRowV1`), and a `stale_state_vector` boolean (derived from CRDT state vector freshness per row). Wire the new boolean from the backend projection. Update tests.

3. **DCC-R5 — Promotion Preview field depth (Master Spec 10.11.5.28):**
   Extend `DccPromotionPreviewRowV1` in `app/src/lib/api.ts` and the rendered table in `KernelDccProjectionView.tsx:333-357` to include: `state_vector` (string), `validation_check_summaries: string[]`, `idempotency_key`, `expected_event_kinds: string[]`, `stale_risk` enum/flag, `accepted_event_ref`, `rejected_event_ref`. Existing accepted/rejected refs already on the type — render them. Compute `stale_risk` in the backend projection from current vs. preview state vector. Update tests.

4. **MT-043 hardening (non-blocking) — runtime-derivation test coverage:**
   Add three Rust tests in the kernel session-spawn test surface:
   (a) feed `FlightRecorderEventType::SessionSpawnAnnounceBack` events through `session_spawn_runtime_evidence_from_state` and assert badges appear with correct payload fields;
   (b) populate `capability_grants` on a model session and assert `cascade_cancel_session_ids` is populated by `session_has_explicit_cascade_cancel_capability`;
   (c) feed a `FlightRecorderEventType::SessionCascadeCancel` event and assert cascade-cancel is surfaced. These prove "from runtime records" and would catch a regression in the new derivation helpers.

5. **MT-045 hardening (non-blocking) — end-to-end capture proof + dep checks:**
   (a) Add a smoke integration test that boots a local HTML fixture (Tauri test or local HTTP), invokes `handshake screenshot capture --scope full-app` and again for `--scope panel` + `--scope module`, and asserts a non-empty PNG with metadata `width`/`height` matching the captured image header. (b) Add an explicit Node/Playwright pre-flight check in `capture_product_screenshot_from_browser_adapter` with an actionable error when the adapter binary or `playwright` package is missing.

6. **MT-049 hardening (non-blocking) — wrap justfile recipes:**
   Update `justfile` so `gov-check`, `build-order-sync`, and `task-packet-stub-contracts` each wrap their underlying node invocation in `handshake command receipt run --command-line "..." --workdir "$(pwd)" --artifact-root ../Handshake_Artifacts/handshake-product/command-receipts`. This makes receipt artifacts a side effect of every recipe invocation, not a separate step. Add an integration test that runs `just gov-check` and asserts the receipt file is present and binds the current candidate SHA.

Notes:
- Items 1–3 are spec-compliance blockers for whole-WP PASS at strict reading of Section 10.11.5.28. Operator decision can downgrade them to follow-up if MVP DCC viewer depth is acceptable as v1.
- Items 4–6 are hardening of MT-043, MT-045, MT-049 that I accepted as PASS this round; they prevent silent regressions in the new derivation, capture, and receipt paths.
- After remediation: commit on the candidate branch, then re-request integration validation. Worktree must remain `wtc-preuse-hardening-v1` on `feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`; do not rebase or squash without operator instruction.
- Backend (R1–R6) is fully spec-compliant; the remediation surface is concentrated in the DCC viewer + test depth.

</topic>

<topic id="merge-readiness" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T18:10:00Z" ingestable="true" summary="Overall merge readiness assessment.">

MERGE_READINESS: NOT_READY_PENDING_OPERATOR_DECISION.

Reason: backend is spec-complete (all 6 backend requirements MET). DCC frontend renders all required projection types but with insufficient field depth on 3 of 6 viewers vs. the explicit "MUST show by X, Y, Z" lists in Section 10.11.5.28. Two operator decision paths:

PATH A (strict spec compliance): block merge, return WP to Kernel Builder for items 1–3 of the combined remediation plan. After commit, request integration re-validation. Items 4–6 are hardening that may be deferred or bundled.

PATH B (MVP acceptance + scheduled follow-up): merge now on backend strength; create a follow-up WP for DCC viewer depth (items 1–3) with explicit anchor refs to 10.11.5.28. This trades strict spec compliance now for delivery momentum, and creates spec-vs-code drift until the follow-up lands.

Recommendation: PATH A is the cleaner option since the backend types already carry the missing fields — the DCC viewer extension is a focused UI/projection-shape change, not new backend authority. Estimated scope ~3 files in `app/src/`, ~1 file in `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs`, plus test extensions.

If Operator chooses PATH B, I require explicit instruction before merging and will record the merge under that decision in this appendage.

</topic>
