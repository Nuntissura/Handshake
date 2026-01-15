# Task Packet: WP-1-Role-Mailbox-v1

## METADATA
- TASK_ID: WP-1-Role-Mailbox-v1
- WP_ID: WP-1-Role-Mailbox-v1
- BASE_WP_ID: WP-1-Role-Mailbox (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-15T02:24:12.241Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja150120260254

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Role-Mailbox-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Role Mailbox threads/messages with deterministic repo export to `docs/ROLE_MAILBOX/`, dedicated Flight Recorder event schemas (`FR-EVT-GOV-MAILBOX-001/002/003`) with strict payload-shape validation, and a leak-safe mechanical gate (`RoleMailboxExportGate`).
- Why: Reduce role-to-role coordination friction while preserving "chat is not state" governance, auditability, and secret-leak prevention.
- IN_SCOPE_PATHS:
  - app/.dependency-cruiser.cjs
  - app/.gitignore
  - app/.vscode/extensions.json
  - app/eslint.config.js
  - app/index.html
  - app/package.json
  - app/pnpm-lock.yaml
  - app/public/tauri.svg
  - app/public/vite.svg
  - app/README.md
  - app/src/App.css
  - app/src/App.test.tsx
  - app/src/App.tsx
  - app/src/assets/react.svg
  - app/src/components/CanvasHeader.tsx
  - app/src/components/CanvasSerialization.test.ts
  - app/src/components/CanvasView.tsx
  - app/src/components/DebugPanel.test.tsx
  - app/src/components/DebugPanel.tsx
  - app/src/components/DocumentView.test.tsx
  - app/src/components/DocumentView.tsx
  - app/src/components/ExcalidrawCanvas.tsx
  - app/src/components/FlightRecorderView.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/components/operator/DebugBundleComplete.tsx
  - app/src/components/operator/DebugBundleExport.tsx
  - app/src/components/operator/DebugBundleProgress.tsx
  - app/src/components/operator/EvidenceDrawer.tsx
  - app/src/components/operator/index.ts
  - app/src/components/operator/JobsView.tsx
  - app/src/components/operator/ProblemsView.tsx
  - app/src/components/operator/TimelineView.tsx
  - app/src/components/SystemStatus.tsx
  - app/src/components/TiptapEditor.tsx
  - app/src/components/WorkspaceSidebar.test.tsx
  - app/src/components/WorkspaceSidebar.tsx
  - app/src/lib/api.ts
  - app/src/main.tsx
  - app/src/setupTests.ts
  - app/src/state/debugEvents.ts
  - app/src/vite-env.d.ts
  - app/src-tauri/.gitignore
  - app/src-tauri/build.rs
  - app/src-tauri/capabilities/default.json
  - app/src-tauri/Cargo.lock
  - app/src-tauri/Cargo.toml
  - app/src-tauri/icons/128x128.png
  - app/src-tauri/icons/128x128@2x.png
  - app/src-tauri/icons/32x32.png
  - app/src-tauri/icons/icon.icns
  - app/src-tauri/icons/icon.ico
  - app/src-tauri/icons/icon.png
  - app/src-tauri/icons/Square107x107Logo.png
  - app/src-tauri/icons/Square142x142Logo.png
  - app/src-tauri/icons/Square150x150Logo.png
  - app/src-tauri/icons/Square284x284Logo.png
  - app/src-tauri/icons/Square30x30Logo.png
  - app/src-tauri/icons/Square310x310Logo.png
  - app/src-tauri/icons/Square44x44Logo.png
  - app/src-tauri/icons/Square71x71Logo.png
  - app/src-tauri/icons/Square89x89Logo.png
  - app/src-tauri/icons/StoreLogo.png
  - app/src-tauri/src/lib.rs
  - app/src-tauri/src/main.rs
  - app/src-tauri/tauri.conf.json
  - app/tsconfig.json
  - app/tsconfig.node.json
  - app/vite.config.ts
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/artifact.rs
  - src/backend/handshake_core/src/ace/validators/boundary.rs
  - src/backend/handshake_core/src/ace/validators/budget.rs
  - src/backend/handshake_core/src/ace/validators/cache.rs
  - src/backend/handshake_core/src/ace/validators/compaction.rs
  - src/backend/handshake_core/src/ace/validators/determinism.rs
  - src/backend/handshake_core/src/ace/validators/drift.rs
  - src/backend/handshake_core/src/ace/validators/freshness.rs
  - src/backend/handshake_core/src/ace/validators/injection.rs
  - src/backend/handshake_core/src/ace/validators/leakage.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/payload.rs
  - src/backend/handshake_core/src/ace/validators/promotion.rs
  - src/backend/handshake_core/src/api/bundles.rs
  - src/backend/handshake_core/src/api/canvases.rs
  - src/backend/handshake_core/src/api/diagnostics.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/api/logs.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/api/paths.rs
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/mod.rs
  - src/backend/handshake_core/src/bundles/redactor.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/bundles/validator.rs
  - src/backend/handshake_core/src/bundles/zip.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/diagnostics/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/logging.rs
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/mex/envelope.rs
  - src/backend/handshake_core/src/mex/gates.rs
  - src/backend/handshake_core/src/mex/mod.rs
  - src/backend/handshake_core/src/mex/registry.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/src/terminal/config.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/redaction.rs
  - src/backend/handshake_core/src/terminal/session.rs
  - src/backend/handshake_core/src/tokenization.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/mex_tests.rs
  - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/tests/terminal_guards_tests.rs
  - src/backend/handshake_core/tests/terminal_session_tests.rs
  - src/backend/handshake_core/tests/tokenization_service_tests.rs
  - src/backend/handshake_core/tests/tokenization_tests.rs
  - src/backend/handshake_core/src/role_mailbox.rs
  - src/backend/handshake_core/src/api/role_mailbox.rs
  - src/backend/handshake_core/tests/role_mailbox_tests.rs
  - docs/ROLE_MAILBOX/index.json
  - docs/ROLE_MAILBOX/export_manifest.json
  - scripts/validation/role_mailbox_export_check.mjs
  - justfile
  Note: app changes are optional; keep them minimal and prefer backend-only in this WP.
- OUT_OF_SCOPE:
  - Full mail client features (see Master Spec 10.3 / 11.7.3)
  - Any use of mailbox messages as authoritative state (must transcribe decisions into signed governance artifacts)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff (gates + hygiene):
just pre-work WP-1-Role-Mailbox-v1

# Backend checks
just test
just lint

just cargo-clean

# Run after implementation (before PR/merge)
just post-work WP-1-Role-Mailbox-v1
```

### DONE_MEANS
- Flight Recorder mailbox events are dedicated schemas and ingestion rejects invalid payload shape and forbidden fields (no inline body/unbounded text) per Master Spec 11.5.3.
- RoleMailbox message creation/export/transcription emit required `FR-EVT-GOV-MAILBOX-001/002/003` and append Spec Session Log entries with the required event_type values.
- Repo export `docs/ROLE_MAILBOX/` is deterministic and leak-safe: no inline body fields, bounded redacted subject/note fields, and manifest hashes verify.
- `RoleMailboxExportGate` exists and fails on out-of-sync export, schema violations, missing transcription links for governance-critical message types, or forbidden fields in export files.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.112.md (recorded_at: 2026-01-15T02:24:12.241Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md \u00a72.6.8.10 (Role Mailbox) + \u00a72.6.8.8 (Spec Session Log) + \u00a711.5.3 (FR-EVT-GOV-MAILBOX-001/002/003 schemas)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - docs/refinements/WP-1-Role-Mailbox-v1.md
  - docs/task_packets/WP-1-Role-Mailbox-v1.md
  - Handshake_Master_Spec_v02.112.md
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "FR-EVT-GOV-MAILBOX"
  - "gov_mailbox_message_created"
  - "FlightRecorderEventType"
  - "validate_event_payload"
  - "RoleMailboxExportGate"
  - "docs/ROLE_MAILBOX"
  - "Secret Redactor"
  - "ArtifactHandle"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Role-Mailbox-v1
  just test
  just lint
  just post-work WP-1-Role-Mailbox-v1
  ```
- RISK_MAP:
  - "secret leakage via repo export" -> "credential exposure / audit contamination"
  - "event schema drift" -> "audit/search failures; validator gate false negatives"
  - "non-deterministic export" -> "byte-diff noise; flaky gates; unreliable handoff"
  - "path traversal via ids" -> "write outside docs/ROLE_MAILBOX/; repo corruption"

## SKELETON
- Proposed interfaces/types/contracts:
  - RoleMailboxThread, RoleMailboxMessage, TranscriptionLink (per Master Spec 2.6.8.10)
  - FR event types: gov_mailbox_message_created / gov_mailbox_exported / gov_mailbox_transcribed (per 11.5.3)
  - RoleMailboxExportGate: validates export_manifest + JSONL schema + forbidden field scan
- Open questions:
  - Is RoleMailbox implemented as a backend-only primitive first (preferred), with UI/API surfaces added in a follow-up WP?
  - Where should "Secret Redactor" live in code (existing module vs new)?
- Notes:
  - Keep mailbox bodies as artifacts only; never inline body in events or repo export.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
