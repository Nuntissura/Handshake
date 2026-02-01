# Task Packet: WP-1-Atelier-Collaboration-Panel-v1

## METADATA
- TASK_ID: WP-1-Atelier-Collaboration-Panel-v1
- WP_ID: WP-1-Atelier-Collaboration-Panel-v1
- BASE_WP_ID: WP-1-Atelier-Collaboration-Panel (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-31T18:03:48.721Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (planned Coder-A)
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** Validated (PASS)
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja310120261839

## USER_CONTEXT (Non-Technical Explainer) [CX-654]
- You highlight a piece of text in the document and click "Collaborate on selection".
- The system shows multiple "roles" (like different specialists) and each role can propose one or more edits for ONLY the highlighted text.
- You choose which suggestions to apply.
- Safety rule: anything outside your highlighted selection must stay exactly the same (byte-identical), and out-of-scope edits are rejected and logged for audit.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Atelier-Collaboration-Panel-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the Atelier Collaboration Panel workflow for the Docs editor (Tiptap) to collaborate on a bounded selection (per-role suggestions + apply), with strict selection-bounded patch application.
- Why: Enable safe, auditable in-editor collaboration without allowing silent edits outside the operator's selected span.
- IN_SCOPE_PATHS:
  - docs/task_packets/WP-1-Atelier-Collaboration-Panel-v1.md
  - docs/refinements/WP-1-Atelier-Collaboration-Panel-v1.md
  - docs/WP_TRACEABILITY_REGISTRY.md
  - docs/TASK_BOARD.md
  - app/src/components/DocumentView.tsx
  - app/src/components/TiptapEditor.tsx
  - app/src/components/DocumentView.test.tsx
  - app/src/lib/api.ts
  - app/src/App.css
  - app/src/components/AtelierCollaborationPanel.tsx
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/atelier_scope.rs
  - src/backend/handshake_core/tests/atelier_collaboration_panel_tests.rs
- OUT_OF_SCOPE:
  - True multi-user collaboration (CRDT/presence/conflict resolution).
  - Monaco editor integration (Monaco surface is not present in app today; handle in a future WP when Monaco exists).
  - Boundary-normalization (explicitly disabled for v1; any out-of-selection changes are rejected).
  - Implementing other Atelier/Lens validators besides ATELIER-LENS-VAL-SCOPE-001.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Atelier-Collaboration-Panel-v1

# Frontend tests (UI + selection apply behavior):
cd app; pnpm test

# Backend tests:
cd src/backend/handshake_core; cargo test

# Hygiene / CI parity:
just lint
just validator-spec-regression
just validator-scan
just validator-error-codes

just cargo-clean
just post-work WP-1-Atelier-Collaboration-Panel-v1
```

### DONE_MEANS
- Docs editor supports "collaborate on selection": operator selects a bounded span and opens a side panel showing all roles (per spec 14.2.1).
- Each role can emit 0..n suggestions; when multiple suggestions exist, UI supports selecting and applying one or more.
- Application rules are enforced:
  - doc_patchset is selection-bounded; any attempt to change outside the selection is rejected (boundary-normalization disabled in v1).
  - Non-selected text remains byte-identical after applying suggestions.
- Validator `ATELIER-LENS-VAL-SCOPE-001` exists and blocks out-of-scope patch application attempts.
- Flight Recorder / evidence:
  - FR-EVT-002 editor_edit is emitted on applying a suggestion, with ops strictly within the selection.
  - FR-EVT-006 llm_inference is emitted for role suggestion generation (trace_id and model_id required).
  - FR-EVT-003 diagnostic is emitted when an out-of-scope patch is rejected (links to Diagnostic.id).
- `just pre-work WP-1-Atelier-Collaboration-Panel-v1` and `just post-work WP-1-Atelier-Collaboration-Panel-v1` pass on the WP branch worktree.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.123.md (recorded_at: 2026-01-31T18:03:48.721Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md Addendum: 14.2.1 (Atelier Collaboration Panel, selection-scoped) + 14.3 (ATELIER-LENS-VAL-SCOPE-001) + 11.5.1 (FR-EVT-002/003) + 11.5.2 (FR-EVT-006)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md
- Approval: Task packet creation authorized by USER_SIGNATURE `ilja310120261839` (refinement approved; ENRICHMENT_NEEDED=NO)

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- N/A (first activated packet for BASE_WP_ID; prior artifact is a non-executable stub: docs/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md).

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.123.md (Addendum: 14.2.1, 14.3; Flight Recorder: 11.5.1/11.5.2)
  - docs/task_packets/WP-1-Atelier-Collaboration-Panel-v1.md
  - docs/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md
  - docs/refinements/WP-1-Atelier-Collaboration-Panel-v1.md
  - app/src/components/DocumentView.tsx
  - app/src/components/TiptapEditor.tsx
  - app/src/lib/api.ts
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/api/workspaces.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/diagnostics/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
- SEARCH_TERMS:
  - "### 14.2.1 Atelier Collaboration Panel"
  - "ATELIER-LENS-VAL-SCOPE-001"
  - "doc_edit"
  - "JobKind::DocEdit"
  - "editor_edit"
  - "FlightRecorderEventType::EditorEdit"
  - "validate_editor_edit_payload"
  - "llm_inference"
  - "FR-EVT-006"
  - "selectionUpdate"
  - "write_context_from_headers"
  - "HSK-403-SILENT-EDIT"
  - "record_diagnostic"
- RUN_COMMANDS:
  ```bash
  rg -n "14\\.2\\.1|14\\.3|ATELIER-LENS-VAL-SCOPE-001|FR-EVT-002|FR-EVT-006" Handshake_Master_Spec_v02.123.md
  just pre-work WP-1-Atelier-Collaboration-Panel-v1
  cd src/backend/handshake_core; cargo test
  cd app; pnpm test
  just lint
  just validator-scan
  just validator-error-codes
  ```
- RISK_MAP:
  - "Selection boundary bug (off-by-one) allows out-of-range mutation" -> "Spec violation; must be blocked by ATELIER-LENS-VAL-SCOPE-001 and covered by tests"
  - "DocEdit job produces edits outside selection" -> "Must be rejected; emit diagnostic + FR-EVT-003"
  - "Missing editor_edit event on apply" -> "Auditability gap; Validator should fail"
  - "TipTap selection mapping differs from stored blocks" -> "Non-selected text may change; must keep byte-identical outside selection"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `app/src/App.css`
- **Start**: 1
- **End**: 1573
- **Line Delta**: 146
- **Pre-SHA1**: `70ed6ee27f439ec5cb4b1c3833601b2e116a38c3`
- **Post-SHA1**: `2c0c011c75e7b09edbb0d07b4b7ee31554bf2468`
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

- **Target File**: `app/src/components/AtelierCollaborationPanel.tsx`
- **Start**: 1
- **End**: 409
- **Line Delta**: 409
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `2424899d24f9a2875b04af3bc495fec84f5d1994`
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

- **Target File**: `app/src/components/DocumentView.test.tsx`
- **Start**: 1
- **End**: 175
- **Line Delta**: 43
- **Pre-SHA1**: `29521a691459b2baeda1225de955bf614fa33aea`
- **Post-SHA1**: `3d121e47660ff9cd93a59861995ee57d7a9107ff`
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

- **Target File**: `app/src/components/DocumentView.tsx`
- **Start**: 1
- **End**: 537
- **Line Delta**: 51
- **Pre-SHA1**: `c83d4ebd0807390ccfabc85850a87d269b27325f`
- **Post-SHA1**: `5fcb5695548c452feb772e6b67f3b0fcd37735e3`
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

- **Target File**: `app/src/components/TiptapEditor.tsx`
- **Start**: 1
- **End**: 178
- **Line Delta**: 25
- **Pre-SHA1**: `da0ac8abeda52344e2f78450950be5c4c92f787d`
- **Post-SHA1**: `df8c5f5cde7dc330472f2ed9af883e8f47099404`
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

- **Target File**: `app/src/lib/api.ts`
- **Start**: 1
- **End**: 756
- **Line Delta**: 78
- **Pre-SHA1**: `45af6603fa4578d99c451f17959576a3b27edb11`
- **Post-SHA1**: `14a9485f1f6cff0203f8d592f2f3f855ffb80062`
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

- **Target File**: `src/backend/handshake_core/src/ace/validators/atelier_scope.rs`
- **Start**: 1
- **End**: 250
- **Line Delta**: 250
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `99a51b3c8c590fd5484cfe0ef7d3abd328260a7e`
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

- **Target File**: `src/backend/handshake_core/src/ace/validators/mod.rs`
- **Start**: 1
- **End**: 1270
- **Line Delta**: 1
- **Pre-SHA1**: `8ded388435c549b63390171146faefd2bcd79d4b`
- **Post-SHA1**: `25b80de87f0e8ae1c028b55e092a0e53ee008375`
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

- **Target File**: `src/backend/handshake_core/src/api/workspaces.rs`
- **Start**: 1
- **End**: 1757
- **Line Delta**: 934
- **Pre-SHA1**: `78ef525e962e60d81e73c2379b9f16d4df676791`
- **Post-SHA1**: `71260615581b0699e5671160bbf3727837dfd08b`
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

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 5465
- **Line Delta**: 129
- **Pre-SHA1**: `86cc7746ad4c75429b6b8cfd7351b92b71e8d159`
- **Post-SHA1**: `6650634199179fdead7b86d80d05fe3284f7110a`
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

- **Target File**: `src/backend/handshake_core/tests/atelier_collaboration_panel_tests.rs`
- **Start**: 1
- **End**: 99
- **Line Delta**: 99
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `cf6c074130775033c97fb2c7607de394062a49ef`
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

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update: Added non-spoofable applied-suggestion provenance (contract_id + source_job_id) and recorded applied provenance in FR-EVT-002; tightened UI apply rules (stale/selection mismatch prevention) + suggestion previews.
- Next step / handoff hint: Stage/commit in-scope files and notify Validator; see `## EVIDENCE` for command outputs.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

### 2026-02-01

#### Frontend
```text
$ NO_COLOR=1 pnpm -C app test

> app@0.1.0 test D:\Projects\LLM projects\wt-WP-1-Atelier-Collaboration-Panel-v1\app
> vitest run


 RUN  v4.0.15 D:/Projects/LLM projects/wt-WP-1-Atelier-Collaboration-Panel-v1/app

 \u2713 src/components/CanvasSerialization.test.ts (1 test) 5ms
 \u2713 src/components/DebugPanel.test.tsx (3 tests) 460ms
     \u2713 renders healthy system status and recent events  420ms
 \u2713 src/components/WorkspaceSidebar.test.tsx (2 tests) 215ms
 \u2713 src/components/DocumentView.test.tsx (2 tests) 296ms
 \u2713 src/App.test.tsx (3 tests) 256ms

 Test Files  5 passed (5)
      Tests  11 passed (11)
   Start at  03:29:27
   Duration  2.71s (transform 2.00s, setup 739ms, import 2.84s, tests 1.23s, environment 4.61s)
```

```text
$ pnpm -C app lint

> app@0.1.0 lint D:\Projects\LLM projects\wt-WP-1-Atelier-Collaboration-Panel-v1\app
> eslint src --ext .ts,.tsx
```

#### Backend
```text
$ cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib --tests -q

running 154 tests
....................................................................................... 87/154
...................................................................
test result: ok. 154 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.78s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
..
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
.......
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s


running 3 tests
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 13 tests
iiii.........
test result: ok. 9 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 1.04s


running 5 tests
.....
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.33s


running 4 tests
....
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 3 tests
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.53s


running 6 tests
......
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s


running 4 tests
....
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s


running 2 tests
..
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 13 tests
.............
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s


running 5 tests
.....
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s


running 4 tests
....
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s


running 5 tests
.....
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s

warning: value assigned to `timed_out` is never read
    --> src\\workflows.rs:3003:17
     |
3003 |         let mut timed_out = false;
     |                 ^^^^^^^^^
     |
     = help: maybe it is overwritten before being read?
     = note: `#[warn(unused_assignments)]` (part of `#[warn(unused)]`) on by default

warning: value assigned to `stdout_rel` is never read
    --> src\\workflows.rs:3009:17
     |
3009 |         let mut stdout_rel: Option<String> = None;
     |                 ^^^^^^^^^^
     |
     = help: maybe it is overwritten before being read?

warning: value assigned to `stderr_rel` is never read
    --> src\\workflows.rs:3010:17
     |
3010 |         let mut stderr_rel: Option<String> = None;
     |                 ^^^^^^^^^^
     |
     = help: maybe it is overwritten before being read?

warning: method `report_kind` is never used
   --> src\\mex\\supply_chain.rs:223:8
    |
194 | impl ScanJobKind {
    | ---------------- method in this implementation
...
223 |     fn report_kind(&self) -> Option<SupplyChainReportKind> {
    |        ^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

#### Determinism gate
```text
$ just post-work WP-1-Atelier-Collaboration-Panel-v1 --range 17608f01a0f1a966e65cc677e63be025c7f2118c..HEAD

Checking Phase Gate for WP-1-Atelier-Collaboration-Panel-v1...
? GATE PASS: Workflow sequence verified.

Post-work validation for WP-1-Atelier-Collaboration-Panel-v1 (deterministic manifest + gates)...

Check 1: Validation manifest present

Diff selection: range (explicit --range)
Git range: 17608f01a0f1a966e65cc677e63be025c7f2118c..77c7aeb682e1fb5c2b2bec869f97ca106f1ea779

Check 2: Manifest fields

Check 3: File integrity (per manifest entry)

Check 4: Git status

==================================================
Post-work validation PASSED with warnings

Warnings:
  1. Manifest[2]: Could not load 17608f01a0f1a966e65cc677e63be025c7f2118c version (new file or not tracked at 17608f01a0f1a966e65cc677e63be025c7f2118c): app\\src\\components\\AtelierCollaborationPanel.tsx
  2. Manifest[7]: Could not load 17608f01a0f1a966e65cc677e63be025c7f2118c version (new file or not tracked at 17608f01a0f1a966e65cc677e63be025c7f2118c): src\\backend\\handshake_core\\src\\ace\\validators\\atelier_scope.rs
  3. Manifest[11]: Could not load 17608f01a0f1a966e65cc677e63be025c7f2118c version (new file or not tracked at 17608f01a0f1a966e65cc677e63be025c7f2118c): src\\backend\\handshake_core\\tests\\atelier_collaboration_panel_tests.rs

You may proceed with commit.
? ROLE_MAILBOX_EXPORT_GATE PASS
warning: in the working copy of 'docs/task_packets/WP-1-Atelier-Collaboration-Panel-v1.md', CRLF will be replaced by LF the next time Git touches it
fatal: path 'app/src/components/AtelierCollaborationPanel.tsx' exists on disk, but not in '17608f01a0f1a966e65cc677e63be025c7f2118c'
fatal: path 'src/backend/handshake_core/src/ace/validators/atelier_scope.rs' exists on disk, but not in '17608f01a0f1a966e65cc677e63be025c7f2118c'
fatal: path 'src/backend/handshake_core/tests/atelier_collaboration_panel_tests.rs' exists on disk, but not in '17608f01a0f1a966e65cc677e63be025c7f2118c'
```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### 2026-02-01 Validator Report (PASS)

Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Atelier-Collaboration-Panel-v1.md
- Spec: Handshake_Master_Spec_v02.123.md

Repo Context:
- Worktree: D:\Projects\LLM projects\wt-WP-1-Atelier-Collaboration-Panel-v1
- Branch: feat/WP-1-Atelier-Collaboration-Panel-v1
- Validated HEAD: b417500936615db2aed7a92118a6ac8e2cb776bb
- Merge-base (main..HEAD): 9bc9299e32323af73489191eb3862bec53f28826

Evidence Mapping:
- Spec 14.2.1 (selection-scoped collaboration): selection-bounded apply enforced by apply_selection_bounded_patchsets in src/backend/handshake_core/src/ace/validators/atelier_scope.rs.
- Spec 14.3 (validator name): out-of-selection mutations rejected as ATELIER-LENS-VAL-SCOPE-001 in src/backend/handshake_core/src/api/workspaces.rs and recorded as a Diagnostic with job_id when available.
- Non-spoofable provenance: apply requests are verified against server-stored job outputs via verify_atelier_applied_suggestion_v1 in src/backend/handshake_core/src/api/workspaces.rs.
- FR-EVT-002 editor_edit: applied_suggestions include role_id/contract_id/suggestion_id/source_job_id/source_trace_id/source_model_id plus before_span/after_span in src/backend/handshake_core/src/api/workspaces.rs.
- Doc-edit job outputs include contract_id + source_model_id in src/backend/handshake_core/src/workflows.rs.

Checks Run (results):
- just validator-scan: PASS
- just validator-spec-regression: PASS
- just validator-git-hygiene: PASS
- just validator-error-codes: PASS
- just validator-traceability: PASS
- just validator-dal-audit: PASS
- just validator-coverage-gaps: PASS
- pnpm -C app lint: PASS
- pnpm -C app test: PASS (11 tests)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml verify_atelier_apply_provenance_ -q: PASS (3 tests)
- just post-work WP-1-Atelier-Collaboration-Panel-v1 --range 9bc9299e32323af73489191eb3862bec53f28826..b417500936615db2aed7a92118a6ac8e2cb776bb: PASS (warnings only for files not present at base)

Notes:
- Removed forbidden panic! in test-only code under src/backend/handshake_core/src/api/workspaces.rs to satisfy just validator-scan; no production behavior change.

REASON FOR PASS:
- Selection-bounded apply is enforced at the validator layer, violations are blocked with a stable diagnostic code, apply provenance is server-verified (non-spoofable), and required hygiene/tests are green.
