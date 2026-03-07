# Task Packet: WP-1-Locus-Phase1-Integration-Occupancy-v1

## METADATA
- TASK_ID: WP-1-Locus-Phase1-Integration-Occupancy-v1
- WP_ID: WP-1-Locus-Phase1-Integration-Occupancy-v1
- BASE_WP_ID: WP-1-Locus-Phase1-Integration-Occupancy (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-03-06T19:48:58.823Z
- MERGE_BASE_SHA: 21ee7c29d34a1f0e5a22f989756973aca15e65fc (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator ilja
- AGENT_ID: Codex-Orchestrator
- ROLE: Orchestrator
- AGENTIC_MODE: NO
<!-- Allowed: YES | NO -->
- ORCHESTRATOR_MODEL: N/A
<!-- Required if AGENTIC_MODE=YES -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required if AGENTIC_MODE=YES -->
- CODER_MODEL: Coder-A
- CODER_REASONING_STRENGTH: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- **Status:** In Progress
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Micro-Task-Executor, WP-1-Spec-Router-SpecPromptCompiler, WP-1-Workflow-Engine, WP-1-Flight-Recorder, WP-1-Storage-Abstraction-Layer
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Locus-Phase1-QueryContract-Autosync, WP-1-Locus-Phase1-Medallion-Search
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-1-Locus-Phase1-QueryContract-Autosync-v1, WP-1-Locus-Phase1-Medallion-Search-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- USER_SIGNATURE: ilja060320261915
- PACKET_FORMAT_VERSION: 2026-03-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: IN_PROGRESS
Blockers: Skeleton approval required before implementation
Next: Operator/Validator reviews the drafted `## SKELETON` and runs `just skeleton-approved WP-1-Locus-Phase1-Integration-Occupancy-v1`

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: Signature bundle selected Coder-A execution lane for WP-1-Locus-Phase1-Integration-Occupancy-v1
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- NOTE: `AGENTIC_MODE: YES` means the Orchestrator owns the run; `AGENTIC_MODE: NO` still allows coder-side sub-agents if Operator approval evidence is recorded here.
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat. The WP signature bundle execution lane may serve as that approval evidence when it explicitly authorizes agent use for the run.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Locus-Phase1-Integration-Occupancy-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-TrackedWorkPacket
  - PRIM-TrackedMicroTask
  - PRIM-LocusCreateWPJob
  - PRIM-FlightRecorder
- PRIMITIVE_INDEX_ACTION: NO_CHANGE (UPDATED | NO_CHANGE)
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND (OK | NEEDS_STUBS | NONE_FOUND)
- STUB_WP_IDS: WP-1-Locus-Phase1-QueryContract-Autosync-v1, WP-1-Locus-Phase1-Medallion-Search-v1 (comma-separated WP-... IDs | NONE)

## SCOPE
- What: Wire Locus into the Spec Router and Micro-Task Executor paths and add replay-safe micro-task occupancy tracking so routed work packets and MT lifecycle events materialize inside Locus.
- Why: The spec already defines Locus as the authoritative work-tracking plane for routed WPs, MT lifecycle state, and occupancy. The current product code still treats those paths as local-only artifacts, which leaves Locus under-integrated and parallel ModelSession occupancy untracked.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/sqlite_store.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/mcp_e2e_tests.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- OUT_OF_SCOPE:
  - PostgreSQL parity or backend migration strategy work
  - Task Board autosync/query-contract work
  - Medallion or search/query retrieval surfaces
  - UI/product-console changes

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Locus-Phase1-Integration-Occupancy-v1
cargo test -p handshake_core micro_task_executor_tests
cargo test -p handshake_core mcp_e2e_tests
cargo test -p handshake_core model_session_scheduler_tests
just post-work WP-1-Locus-Phase1-Integration-Occupancy-v1 --range 21ee7c29d34a1f0e5a22f989756973aca15e65fc..HEAD
```

### DONE_MEANS
- Routed prompts that create a task packet also submit a `locus_create_wp_v1` job with `task_packet_path` and `spec_session_id`, and the resulting Locus write still emits the canonical work-packet Flight Recorder event.
- The MT executor loop calls `locus_register_mts_v1`, `locus_start_mt_v1`, `locus_record_iteration_v1`, and `locus_complete_mt_v1` at the spec-defined lifecycle points without bypassing the existing Locus job dispatcher.
- Tracked micro-tasks persist `active_session_ids`, plus bind/unbind occupancy updates, with deterministic add/remove semantics that survive retries and replay without duplicating sessions or iteration history.
- Capability mapping and regression tests cover the new Locus lifecycle/occupancy path so the router/executor integration is mechanically provable before validation.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.141.md (recorded_at: 2026-03-06T19:48:58.823Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.15.2-2.3.15.6 (TrackedMicroTask.active_session_ids; locus_bind_session/locus_unbind_session; Spec Router auto locus_create_wp; MT Executor lifecycle integration; Flight Recorder event catalog FR-EVT-WP-001 and FR-EVT-MT-001..004)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.
- N/A

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Locus-Phase1-Integration-Occupancy-v1.md
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/sqlite_store.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/tests/micro_task_executor_tests.rs
  - src/backend/handshake_core/tests/mcp_e2e_tests.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- SEARCH_TERMS:
  - "locus_create_wp_v1"
  - "locus_register_mts_v1"
  - "locus_start_mt_v1"
  - "locus_record_iteration_v1"
  - "locus_complete_mt_v1"
  - "active_session_ids"
  - "emit_locus_operation_event"
- RUN_COMMANDS:
  ```bash
  rg -n "locus_create_wp_v1|locus_register_mts_v1|locus_start_mt_v1|locus_record_iteration_v1|locus_complete_mt_v1" src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/capabilities.rs
  rg -n "active_session_ids|TrackedMicroTask|LocusBind|LocusUnbind" src/backend/handshake_core/src/locus src/backend/handshake_core/src/storage/locus_sqlite.rs
  cargo test -p handshake_core micro_task_executor_tests
  ```
- RISK_MAP:
  - "dispatcher bypass" -> "router/executor write directly to storage and lose capability + FR enforcement"
  - "occupancy replay drift" -> "duplicate session ids or iteration rows strand MT capacity after retries/crashes"
  - "provenance loss" -> "routed WPs miss task_packet_path/spec_session_id linkage expected by the spec"
  - "concurrent metadata overwrite" -> "occupancy update clobbers iteration history or MT status under parallel sessions"

## SKELETON
- Proposed interfaces/types/contracts:
  - `src/backend/handshake_core/src/locus/types.rs`
    - Extend `TrackedMicroTask` with `active_session_ids: Vec<String>` so MT occupancy lives in the canonical tracked-MT shape rather than ad-hoc metadata keys.
    - Extend `LocusStartMtParams` to carry `model_id: String`, `lora_id: Option<String>`, and `escalation_level: u32` to match the spec-defined start payload.
    - Add `LocusBindSessionParams { wp_id, mt_id, session_id, model_id: Option<String>, lora_id: Option<String>, escalation_level: u32 }`.
    - Add `LocusUnbindSessionParams { wp_id, mt_id, session_id, reason: Option<String> }`.
    - Extend `LocusOperation` plus protocol parsing for `locus_bind_session_v1` and `locus_unbind_session_v1`.
  - `src/backend/handshake_core/src/capabilities.rs`
    - Map `locus_bind_session_v1` and `locus_unbind_session_v1` to `locus.write`.
    - Preserve the current dispatcher/capability boundary; Spec Router and MT Executor remain producers of Locus jobs, not direct storage writers.
  - `src/backend/handshake_core/src/storage/locus_sqlite.rs` and `src/backend/handshake_core/src/locus/sqlite_store.rs`
    - Keep `micro_tasks.metadata` as the durable tracked-MT envelope, but update the stored JSON transactionally during register/start/record/complete/bind/unbind so `active_session_ids` stays replay-safe and deduplicated.
    - `register_mts` seeds `active_session_ids: []`.
    - `start_mt`, `record_iteration`, `complete_mt`, `bind_session`, and `unbind_session` all update the stored tracked-MT JSON alongside the existing scalar status/current-iteration columns.
  - `src/backend/handshake_core/src/workflows.rs`
    - `run_spec_router_job` submits a follow-on `JobKind::LocusOperation` request for `locus_create_wp_v1` when routing yields a task packet/WP, carrying `wp_id`, title, description, `task_packet_path`, and `spec_session_id`.
    - `run_micro_task_executor_v1` submits `locus_register_mts_v1` after MT generation, `locus_start_mt_v1` when an MT becomes active, `locus_record_iteration_v1` after each completed iteration, and `locus_complete_mt_v1` on terminal completion.
    - Occupancy binding uses the active ModelSession context when present: bind on MT entry, unbind on completion/failure/pause/cancel so stranded session IDs cannot accumulate.
  - Tests
    - Extend the scoped regression suites to assert canonical Locus job submission, persisted `active_session_ids`, and preserved FR events rather than only local progress-artifact behavior.
- Open questions:
  - Confirm the exact source object inside `run_spec_router_job` that already has `wp_id`, title/description, and `task_packet_path`; reuse that payload instead of re-deriving fields in a second place.
  - Confirm which MT executor boundary exposes the canonical `session_id` for occupancy updates when a ModelSession is not yet registered at MT-generation time.
  - Confirm whether paused/governance-gated MTs should unbind immediately or remain bound until human intervention resolves; default assumption for this WP is fail-safe unbind on any non-running state.
- Notes:
  - No new Locus tables are planned in this packet; occupancy remains in the tracked-MT JSON already persisted in `micro_tasks.metadata`.
  - All writes stay on the existing `JobKind::LocusOperation` -> capability gate -> sqlite dispatcher -> Flight Recorder path.
  - Query/autosync/search/Postgres parity remain deferred to the stub WPs listed in packet metadata.

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - <fill; screens/panels/dialogs/menus>
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: <fill> | Type: <fill> | Tooltip: <fill> | Notes: <fill>
- UI_STATES (empty/loading/error):
  - <fill>
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - <fill>
- UI_ACCESSIBILITY_NOTES:
  - <fill>

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: workflow producer -> locus operation dispatcher -> sqlite persistence + flight recorder
- SERVER_SOURCES_OF_TRUTH:
  - Locus operation payloads persisted through `JobKind::LocusOperation`
  - SQLite Locus work-packet / micro-task rows in `src/backend/handshake_core/src/storage/locus_sqlite.rs`
  - Flight Recorder locus event payloads emitted through `emit_locus_operation_event`
- REQUIRED_PROVENANCE_FIELDS:
  - wp_id
  - mt_id
  - task_packet_path
  - spec_session_id
  - model_id
  - escalation_level
  - iteration / token / duration counters
- VERIFICATION_PLAN:
  - Prove the router submits the canonical `locus_create_wp_v1` payload instead of writing local-only artifacts.
  - Prove MT lifecycle hooks pass through the locus dispatcher and still emit the FR-EVT-WP/MT catalog events.
  - Prove occupancy add/remove semantics are replay-safe via targeted tests around repeated start/iteration/completion and session rebinding.
- ERROR_TAXONOMY_PLAN:
  - missing_router_submission vs missing_mt_lifecycle_submission
  - stale_occupancy_state vs duplicate_session_binding
  - provenance_field_missing vs capability_mapping_missing
- UI_GUARDRAILS:
  - NONE; backend-only packet with no direct UI surface
- VALIDATOR_ASSERTIONS:
  - Prove Spec Router and MT Executor produce canonical Locus jobs at the spec anchors above.
  - Prove `active_session_ids` plus bind/unbind semantics persist without duplicate-session drift.
  - Prove capability checks + Flight Recorder emission remain on the write path and were not bypassed by direct SQLite writes.

## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (BOOTSTRAP complete; `## SKELETON` drafted; awaiting approval)
- What changed in this update:
  - Updated the packet state from Ready for Dev to In Progress.
  - Drafted the interface-first skeleton for Spec Router -> Locus submission, MT lifecycle submissions, occupancy bind/unbind contracts, and scoped regression coverage.
- Next step / handoff hint:
  - Run `just coder-skeleton-checkpoint WP-1-Locus-Phase1-Integration-Occupancy-v1`, then STOP for Operator/Validator approval via `just skeleton-approved WP-1-Locus-Phase1-Integration-Occupancy-v1`.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Locus-Phase1-Integration-Occupancy-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
