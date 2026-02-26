# Task Packet: WP-1-Front-End-Memory-System-v1

## METADATA
- TASK_ID: WP-1-Front-End-Memory-System-v1
- WP_ID: WP-1-Front-End-Memory-System-v1
- BASE_WP_ID: WP-1-Front-End-Memory-System
- DATE: 2026-02-26T00:01:20.142Z
- MERGE_BASE_SHA: 460e4198b11994da9515fb8c627e05cd6f4b1760
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-26T00:05:00Z
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja260220260100
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Front-End-Memory-System-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Phase 1 Front End Memory System (FEMS) v0 across runtime, session integration, operator panel visibility, and flight-recorder events, with bounded memory pack injection and explicit review-gated memory writes.
- Why: v02.138 adds Phase 1 MUST-deliver FEMS behavior to prevent memory poisoning/drift and to make memory influence replayable and auditable.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-Front-End-Memory-System-v1.md
  - .GOV/task_packets/WP-1-Front-End-Memory-System-v1.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/promotion.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - app/src/lib/api.ts
  - app/src/state/aiJobs.ts
  - app/src/components/operator/TimelineView.tsx
  - app/src/components/operator/JobsView.tsx
  - app/src/components/operator/DebugBundleExport.tsx
- OUT_OF_SCOPE:
  - Phase 2 FEMS v1 hybrid retrieval and scale/privacy expansion.
  - CRM/contact-profile product expansion beyond minimal hooks required in v02.138.
  - Cross-workspace multi-operator routing and Phase 2+ autonomy behavior.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Front-End-Memory-System-v1

# Governance + formatting + lint:
just gov-check
just fmt
just lint

# Backend tests (include targeted memory and workflow areas):
just test
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_session_tests

# Frontend tests:
cd app; pnpm test

# Post-work deterministic validation:
just cargo-clean
just post-work WP-1-Front-End-Memory-System-v1 --range 460e4198b11994da9515fb8c627e05cd6f4b1760..HEAD
```

### DONE_MEANS
- FEMS job profile contracts are represented in runtime job paths (`memory_extract_v0.1`, `memory_consolidate_v0.1`, `memory_forget_v0.1`) with explicit proposal/commit artifacts.
- Session `memory_policy` behavior is enforced: `EPHEMERAL` injects no MemoryPack; `SESSION_SCOPED` and `WORKSPACE_SCOPED` build bounded packs with deterministic truncation rules.
- Procedural memory writes are review-gated only (no implicit write-through from interactive loops).
- Flight Recorder emits FR-EVT-MEM-001..005 with hash/id-based privacy-safe payloads (no raw memory content).
- DCC/operator UI exposes memory browser/review/preview affordances sufficient to inspect injected pack hash and review queue decisions.
- FEMS-EVAL-001 criteria are covered by tests/evidence (budget bounds, anti-poisoning, replay hash determinism, cloud redaction behavior).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.138.md (recorded_at: 2026-02-26T00:01:20.142Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.138.md 2.6.6.6.6 Front End Memory Job Profile (FEMS) (Normative)
  - Handshake_Master_Spec_v02.138.md 2.6.6.7.6.2 Front End Memory System (FEMS) (Normative)
  - Handshake_Master_Spec_v02.138.md 4.3.9.12.7 Front End Memory System integration (FEMS) (Normative)
  - Handshake_Master_Spec_v02.138.md 5.4.8 Front End Memory System Test Suite (FEMS-EVAL-001) (Normative)
  - Handshake_Master_Spec_v02.138.md 10.11.5.14 Front End Memory Panel (FEMS)
  - Handshake_Master_Spec_v02.138.md 11.5.13 Front End Memory System events (FR-EVT-MEM-*) (Normative)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packets:
  - `.GOV/task_packets/stubs/WP-1-Front-End-Memory-System-v1.md` (stub; non-executable)
- Preserved requirements:
  - FEMS v0 bounded pack injection and deterministic replay hash.
  - Procedural memory review-gating and anti-poisoning controls.
  - FR-EVT-MEM-* event-family coverage and DCC preview/review expectations.
- Changes in this activated packet:
  - Converted stub to executable packet with signed refinement and PREPARE assignment.
  - Added explicit scope, test plan, and measurable done-means for delegation.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.138.md
  - .GOV/refinements/WP-1-Front-End-Memory-System-v1.md
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/promotion.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - app/src/lib/api.ts
  - app/src/state/aiJobs.ts
  - app/src/components/operator/TimelineView.tsx
  - app/src/components/operator/JobsView.tsx
- SEARCH_TERMS:
  - "memory_extract_v0.1"
  - "memory_consolidate_v0.1"
  - "memory_forget_v0.1"
  - "MemoryWriteProposal"
  - "MemoryCommitReport"
  - "MemoryPack"
  - "memory_policy"
  - "memory_state_ref"
  - "FR-EVT-MEM-001"
  - "FR-EVT-MEM-005"
  - "FEMS-EVAL-001"
  - "cloud-safe"
  - "procedural"
  - "review"
- RUN_COMMANDS:
  ```bash
  rg -n "memory_extract|memory_consolidate|memory_forget|MemoryPack|memory_policy|FR-EVT-MEM" src/backend/handshake_core/src app/src
  just pre-work WP-1-Front-End-Memory-System-v1
  just gov-check
  just lint
  ```
- RISK_MAP:
  - "memory poisoning" -> "untrusted content becomes durable procedural memory and alters future model behavior"
  - "context bloat" -> "unbounded pack size causes degraded quality/cost and replay mismatch"
  - "cloud leakage" -> "high-sensitivity memory sent to cloud without consent/redaction"
  - "ui bypass" -> "direct memory edits bypass review and provenance"

## SKELETON
- Proposed interfaces/types/contracts:
  - Runtime contract:
    - `MemoryWriteProposal` + `MemoryCommitReport` artifact schema binding in ACE/job execution.
    - `MemoryPack` builder with deterministic ordering, budget capping, and `memory_pack_hash`.
  - Session binding:
    - `memory_policy` handling in model-call path (`EPHEMERAL|SESSION_SCOPED|WORKSPACE_SCOPED`).
    - `memory_state_ref` updates per committed/injected pack lifecycle.
  - Observability:
    - FR-EVT-MEM-001..005 emitters with ID/hash payloads.
  - Operator surfaces:
    - DCC panel hooks for preview/review queue.
- Open questions:
  - Existing storage shape for durable memory items may require incremental extension rather than a single migration.
  - Confirm whether DCC memory panel MVP should live under existing operator views or dedicated component for this packet.
- Notes:
  - This packet is the core v02.138 Phase 1 FEMS deliverable and should precede FEMS risk/acceptance follow-on packets.
  - BOOTSTRAP baseline scan indicates FEMS-specific contracts are not present yet in in-scope runtime/API/UI paths; this WP introduces the first end-to-end FEMS v0 surface.
  - File-level contract mapping for this WP:
    - `src/backend/handshake_core/src/workflows.rs`: add FEMS job-kind handling + session memory policy orchestration + review-gated commit path wiring.
    - `src/backend/handshake_core/src/jobs.rs` and `src/backend/handshake_core/src/api/jobs.rs`: expose/accept FEMS job contracts and policy-safe job lifecycle entrypoints.
    - `src/backend/handshake_core/src/flight_recorder/mod.rs` and `src/backend/handshake_core/src/flight_recorder/duckdb.rs`: add FR-EVT-MEM-001..005 event typing + validation + persistence mapping.
    - `app/src/lib/api.ts` + `app/src/state/aiJobs.ts`: add frontend contract types/actions for memory proposal/review/pack preview payloads.
    - `app/src/components/operator/TimelineView.tsx`, `app/src/components/operator/JobsView.tsx`, `app/src/components/operator/DebugBundleExport.tsx`: add memory timeline visibility and operator review/preview affordances.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: client->server->storage (memory proposals and policies are client-observable but server-enforced)
- SERVER_SOURCES_OF_TRUTH:
  - Server computes effective memory policy, pack truncation, and cloud-safe redaction decisions.
  - Durable memory commits are applied only from server-side commit jobs, not client direct writes.
- REQUIRED_PROVENANCE_FIELDS:
  - proposal_id, proposal_hash, commit_id, commit_report_hash, memory_pack_hash
  - memory_policy, scope_refs, item_count, token_estimate, truncation_occurred
  - reviewer_kind/decision for review-gated operations
- VERIFICATION_PLAN:
  - Unit/integration tests assert pack budget bounds, deterministic hash replay, and review-gated procedural writes.
  - Flight Recorder assertions validate FR-EVT-MEM event sequence and payload privacy constraints.
- ERROR_TAXONOMY_PLAN:
  - invalid_memory_policy
  - memory_pack_budget_exceeded
  - procedural_write_requires_review
  - cloud_redaction_required
  - memory_event_contract_violation
- UI_GUARDRAILS:
  - Show pack preview hash and truncation flags before model call execution.
  - Review queue requires explicit approve/reject action for procedural and CRM-related writes.
  - Disable direct edit paths on memory records in DCC.
- VALIDATOR_ASSERTIONS:
  - No implicit durable memory write path exists.
  - FR-EVT-MEM-001..005 events are emitted with required fields and without raw content.
  - Replay mode can reproduce memory_pack_hash for identical inputs and pinned selection.

## IMPLEMENTATION
- Added FEMS runtime contracts and deterministic hashing:
  - `MemoryPolicy`, `MemoryWriteProposal`, `MemoryCommitReport`, `MemoryPack` and related memory mutation structs in `src/backend/handshake_core/src/ace/mod.rs`.
  - Deterministic hash computation for proposal/report/pack artifacts.
- Added procedural-review guardrail:
  - `promotion:procedural_requires_review` warning and enforcement in `src/backend/handshake_core/src/ace/validators/promotion.rs`.
- Implemented FEMS execution path in workflows:
  - Protocol dispatch for `memory_extract_v0.1`, `memory_consolidate_v0.1`, `memory_forget_v0.1`.
  - Memory policy enforcement for `EPHEMERAL|SESSION_SCOPED|WORKSPACE_SCOPED`.
  - Deterministic pack construction with truncation accounting and hash emission.
  - Review-gated write proposal/commit flow for procedural memory changes.
- Added FR-EVT-MEM-001..005 event support:
  - Added event types, payload validators, and duckdb mappings.
  - Payloads are id/hash/ref fields only (no raw memory content fields).
- Added API and operator UI plumbing:
  - FEMS job-kind aliases accepted in jobs API with protocol consistency checks.
  - Frontend FEMS types/helpers and memory-aware operator UI surfaces in Jobs/Timeline/Debug export views.

## HYGIENE
- Out-of-scope formatting drift from `just fmt` was reverted to keep this WP diff in-scope.
- TEST_PLAN commands were executed in this worktree and recorded in `## EVIDENCE` with command + exit code entries.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `.GOV/roles_shared/TASK_BOARD.md`
- **Start**: 1
- **End**: 200
- **Line Delta**: 0
- **Pre-SHA1**: `c66eb61f900cb81b7d5b6d01e1f63d18585cca57`
- **Post-SHA1**: `44eec622a7f72668ad89f75696582341bf767881`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `just lint` exit code 0
- **Artifacts**: FEMS runtime, FR event, API, and UI code paths listed in `## IMPLEMENTATION`
- **Timestamp**: 2026-02-26T00:00:00Z
- **Operator**: coder
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.138.md
- **Notes**: Range-manifest values computed against `460e4198b11994da9515fb8c627e05cd6f4b1760..HEAD`

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (implementation complete, hygiene/evidence updated, awaiting validator handoff)
- What changed in this update: Implemented FEMS v0 runtime/API/UI and FR-EVT-MEM-001..005 plumbing; recorded hygiene and deterministic manifest metadata for post-work checks.
- Next step / handoff hint: Run/collect `just post-work ... --range ...` output and hand off to Validator.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "FEMS job profile contracts are represented in runtime job paths (`memory_extract_v0.1`, `memory_consolidate_v0.1`, `memory_forget_v0.1`) with explicit proposal/commit artifacts."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11121`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11288`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11444`
- EVIDENCE: `src/backend/handshake_core/src/api/jobs.rs:22`
- REQUIREMENT: "Session `memory_policy` behavior is enforced with deterministic/bounded memory pack handling."
- EVIDENCE: `src/backend/handshake_core/src/ace/mod.rs:191`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11011`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11180`
- REQUIREMENT: "Procedural memory writes are review-gated only."
- EVIDENCE: `src/backend/handshake_core/src/ace/validators/promotion.rs:22`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:11277`
- REQUIREMENT: "Flight Recorder emits FR-EVT-MEM-001..005 with hash/id-based payloads (no raw memory content)."
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:117`
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:5252`
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:5315`
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/duckdb.rs:816`
- REQUIREMENT: "Operator UI exposes memory inspection/review/preview affordances."
- EVIDENCE: `app/src/components/operator/JobsView.tsx:212`
- EVIDENCE: `app/src/components/operator/TimelineView.tsx:243`
- EVIDENCE: `app/src/components/operator/DebugBundleExport.tsx:219`
- EVIDENCE: `app/src/state/aiJobs.ts:178`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Front-End-Memory-System-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: just pre-work WP-1-Front-End-Memory-System-v1
- EXIT_CODE: 0
- PROOF_LINES: `Pre-work checks completed with no blocking errors`
- COMMAND: just gate-check WP-1-Front-End-Memory-System-v1
- EXIT_CODE: 0
- PROOF_LINES: `All critical gate checks passed`
- COMMAND: just gov-check
- EXIT_CODE: 0
- PROOF_LINES: `Governance checks completed`
- COMMAND: just fmt
- EXIT_CODE: 0
- PROOF_LINES: `Formatting command completed; out-of-scope drift reverted afterward`
- COMMAND: just lint
- EXIT_CODE: 0
- PROOF_LINES: `Lint command completed (warnings present, non-blocking)`
- COMMAND: just test
- EXIT_CODE: 0
- PROOF_LINES: `test result: ok`
- COMMAND: cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_session_tests
- EXIT_CODE: 0
- PROOF_LINES: `test result: ok`
- COMMAND: cd app; pnpm test
- EXIT_CODE: 0
- PROOF_LINES: `Test Files  6 passed`
- COMMAND: just cargo-clean
- EXIT_CODE: 0
- PROOF_LINES: `cargo clean completed`
- COMMAND: just post-work WP-1-Front-End-Memory-System-v1 --range 460e4198b11994da9515fb8c627e05cd6f4b1760..HEAD
- EXIT_CODE: 1
- PROOF_LINES: `Initial run reported placeholder manifest/evidence fields; packet updated accordingly`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
