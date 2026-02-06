# Task Packet: WP-1-Product-Governance-Snapshot-v3

## METADATA
- TASK_ID: WP-1-Product-Governance-Snapshot-v3
- WP_ID: WP-1-Product-Governance-Snapshot-v3
- BASE_WP_ID: WP-1-Product-Governance-Snapshot (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-06T13:40:41.412Z
- MERGE_BASE_SHA: bbf2a67308cd3706056db2b7e3e74cd21c07dbe7
- REQUESTOR: ilja
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-06T13:40:41.412Z
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja060220260923
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Product-Governance-Snapshot-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the deterministic "Product Governance Snapshot" generator + validator command surface per the Master Spec, producing `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json` derived ONLY from canonical `.GOV/**` governance inputs.
- Why: Provides a leak-safe, deterministic snapshot of governance state so a fresh agent/auditor can reconstruct "what is true" without chat history; enables mechanical validation of scope/approvals.
- IN_SCOPE_PATHS:
  - Handshake_Master_Spec_v02.125.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - justfile
  - .GOV/scripts/governance-snapshot.mjs
  - .GOV/scripts/validation/validator-governance-snapshot.mjs
  - .GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json
- OUT_OF_SCOPE:
  - src/
  - app/
  - tests/
  - scripts/ (top-level; do not add new generic scripts here)
  - docs/ (compatibility only; not canonical governance)
  - any file not listed in IN_SCOPE_PATHS

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- Waiver ID: WAIVER-2026-02-06-POSTWORK-REGEX
  - Date: 2026-02-06
  - Scope: Permit editing `.GOV/scripts/validation/post-work-check.mjs` to fix a SyntaxError blocking `just post-work`.
  - Justification: `just post-work` is mandatory in TEST_PLAN; gate tooling regex was invalid and prevented execution.
  - Approver: ilja (user)
  - Expiry: WP-1-Product-Governance-Snapshot-v3 validation complete
  - Ref: CX-573F
- Waiver ID: WAIVER-2026-02-06-ROLEMAILBOX-MANIFEST
  - Date: 2026-02-06
  - Scope: Permit editing `.GOV/ROLE_MAILBOX/export_manifest.json` to make the file canonical (single trailing LF).
  - Justification: `just post-work` runs `just role-mailbox-export-check`; export_manifest.json had an extra trailing newline causing a nonzero exit.
  - Approver: ilja (user)
  - Expiry: WP-1-Product-Governance-Snapshot-v3 validation complete
  - Ref: CX-573F

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Product-Governance-Snapshot-v3

# Implement + verify (determinism + schema + leak-safety):
just governance-snapshot
just governance-snapshot
just validator-governance-snapshot

just cargo-clean
just post-work WP-1-Product-Governance-Snapshot-v3 --range bbf2a67308cd3706056db2b7e3e74cd21c07dbe7..HEAD
```

### DONE_MEANS
- `just governance-snapshot` writes `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json` with bytes exactly `JSON.stringify(obj, null, 2) + "\\n"` (forced LF newline; 2-space indentation).
- Generator reads ONLY the whitelist inputs from Handshake_Master_Spec_v02.125.md `#### 7.5.4.10` (hard-fail on any attempt to read non-whitelisted paths; no repo scan).
- Snapshot conforms to the minimum schema in Handshake_Master_Spec_v02.125.md `#### 7.5.4.10`, including `schema_version: \"hsk.product_governance_snapshot@0.1\"`, stable-sorted collections, and `git: {}` by default (omit `head_sha` unless explicitly enabled).
- Snapshot contains NO wall-clock timestamps and omits raw logs/bodies (validator gate summaries are list-only; no timestamps; no raw logs).
- `just validator-governance-snapshot` runs generator twice, byte-compares outputs, validates schema + whitelist + "no timestamps" invariant, and exits nonzero on any mismatch.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-06T13:40:41.412Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md `#### 7.5.4.10 Product Governance Snapshot (HARD)` + `#### 7.5.4.3 Canonical governance artifacts (kernel)`
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.
- `.GOV/task_packets/stubs/WP-1-Product-Governance-Snapshot-v1.md` (stub)
  - Preserved: original intent (governance snapshot generator/validator).
  - Changed: activated into canonical `.GOV/**` contract with deterministic output + strict whitelist (no repo scan).
- Prior activation attempt: `feat/WP-1-Product-Governance-Snapshot-v2` (worktree `wt-WP-1-Product-Governance-Snapshot-v2`) is superseded and must not merge.
  - Preserved (carried forward): output path `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`; omit `git.head_sha` by default; validator runs generator twice + byte-compare; no timestamps/raw logs; whitelist-only inputs.
  - Changed: canonical governance artifacts now live under `.GOV/**` (kernel rule in v02.125 `#### 7.5.4.3`) and snapshot definition is now explicit in v02.125 `#### 7.5.4.10`.
- Prior attempt: `feat/WP-1-Product-Governance-Snapshot-v3` (worktree `wt-WP-1-Product-Governance-Snapshot-v3`) is docs-based and is superseded by this `.GOV`-canonical activation on `feat/WP-1-Product-Governance-Snapshot-v3-gov`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/SIGNATURE_AUDIT.md
  - .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json
  - .GOV/validator_gates/README.md
  - .GOV/refinements/WP-1-Product-Governance-Snapshot-v3.md
  - .GOV/task_packets/WP-1-Product-Governance-Snapshot-v3.md
  - Handshake_Master_Spec_v02.125.md
  - justfile
  - .GOV/scripts/governance-snapshot.mjs
  - .GOV/scripts/validation/validator-governance-snapshot.mjs
  - .GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json
- SEARCH_TERMS:
  - "Product Governance Snapshot"
  - "hsk.product_governance_snapshot@0.1"
  - "PRODUCT_GOVERNANCE_SNAPSHOT"
  - "governance-snapshot"
  - "validator-governance-snapshot"
  - "wp_gate_summaries"
- RUN_COMMANDS:
  ```bash
  git diff --name-only bbf2a67308cd3706056db2b7e3e74cd21c07dbe7..HEAD
  rg -n "Product Governance Snapshot" Handshake_Master_Spec_v02.125.md
  rg -n "PRODUCT_GOVERNANCE_SNAPSHOT|governance-snapshot|validator-governance-snapshot" -S .GOV
  ```
- RISK_MAP:
  - "nondeterministic ordering" -> "validator byte-compare fails; governance snapshot becomes non-auditable"
  - "implicit repo scan / extra inputs" -> "leak risk + non-reproducible snapshot; violates whitelist HARD rule"
  - "timestamps/locale formatting" -> "nondeterministic bytes and false drift in audits"
  - "path separator differences (Windows vs POSIX)" -> "inconsistent `inputs.path` fields; breaks stable ordering"
  - "validator gates include raw logs/timestamps" -> "snapshot leaks non-deterministic or sensitive content"

## SKELETON
- Proposed interfaces/types/contracts:
- Generator CLI contract:
  - `node .GOV/scripts/governance-snapshot.mjs [--out <path>] [--include-head-sha]`
  - Default output path: `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`
  - Default git provenance: `git: {}` (omit `head_sha` unless `--include-head-sha` is set)
- Generator module skeleton (no logic yet; names subject to implementation):
  - `const SCHEMA_VERSION = "hsk.product_governance_snapshot@0.1"`
  - `const DEFAULT_OUTPUT_PATH = ".GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json"`
  - `const FIXED_INPUT_WHITELIST = [\n    ".GOV/roles_shared/SPEC_CURRENT.md",\n    ".GOV/roles_shared/TASK_BOARD.md",\n    ".GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md",\n    ".GOV/roles_shared/SIGNATURE_AUDIT.md",\n    ".GOV/roles/orchestrator/ORCHESTRATOR_GATES.json"\n  ]` (+ dynamic resolved spec path; + `.GOV/validator_gates/*.json` if present)
  - `async function resolveSpecPathFromSpecCurrent(specCurrentPath): Promise<string>`
  - `async function listValidatorGateJsonPaths(): Promise<string[]>` (readdir + filter `*.json` + sort)
  - `async function sha256File(path): Promise<string>`
  - `function normalizeSnapshotPath(path): string` (use forward slashes; repo-relative)
  - `async function parseTaskBoardEntries(path): Promise<Array<{ wp_id: string, status_token: string }>>` (extract `**[WP_ID]** - [TOKEN]` lines)
  - `async function parseTraceabilityMappings(path): Promise<Array<{ base_wp_id: string, active_packet_path: string }>>` (extract registry table rows)
  - `async function parseSignatureAudit(path): Promise<Array<{ signature: string, purpose: string, wp_id?: string }>>` (extract markdown table rows)
  - `async function summarizeOrchestratorGates(path): Promise<{ last_refinement?: string, last_signature?: string, last_prepare?: string }>`
  - `async function summarizeValidatorGates(paths): Promise<Array<{ wp_id: string, verdict?: string, status?: string, gates_passed?: string[] }>>` (list-only; omit timestamps/log bodies)
  - `async function buildSnapshot(opts): Promise<ProductGovernanceSnapshot>`
  - `async function writeDeterministicJson(path, obj): Promise<void>` (bytes = `JSON.stringify(obj, null, 2) + \"\\n\"`)
- Validator CLI contract:
  - `node .GOV/scripts/validation/validator-governance-snapshot.mjs`
  - Must (a) enforce whitelist-only reads, (b) generate twice and byte-compare, (c) validate minimum schema + stable sorting, (d) assert no timestamps in snapshot text, (e) exit nonzero on any violation.
- Determinism contract (must be enforced in code + validator):
  - Stable sorting per spec: inputs by path, task_board.entries by wp_id, traceability.mappings by base_wp_id, signatures.consumed by signature, gates.validator.wp_gate_summaries by wp_id.
  - No wall clock calls; no locale formatting; stable newlines (LF).
  - No platform-specific paths in output (normalize to forward slashes).
- Open questions:
  - Exact parsing rules for TASK_BOARD + TRACEABILITY + SIGNATURE_AUDIT (robust regex vs markdown table parser) to minimize drift.
  - Should missing optional inputs (e.g., no `.GOV/validator_gates/` dir) be treated as empty list or hard error? (Spec: "if present" -> treat as empty if absent.)
  - Should validator fail if snapshot already exists but differs from generated output (expected: yes; bytes must match).
- Notes:
  - All input files included in `inputs[]` with SHA256; `spec.spec_sha1` computed from resolved spec file contents.
  - Snapshot must not include secrets/env vars/role mailbox bodies; only hashes/refs.

SKELETON APPROVED

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: generator/validator -> filesystem (whitelist-only inputs; deterministic output)
- SERVER_SOURCES_OF_TRUTH:
  - Whitelisted canonical governance files (explicit list in spec `#### 7.5.4.10`)
  - Resolved spec file path derived ONLY from `.GOV/roles_shared/SPEC_CURRENT.md`
  - `.GOV/validator_gates/*.json` discovered via deterministic readdir+sort (if present)
- REQUIRED_PROVENANCE_FIELDS:
  - `schema_version`
  - `spec.spec_target` + `spec.spec_sha1`
  - `inputs[{path, sha256}]`
  - `git.head_sha` only if explicitly enabled
- VERIFICATION_PLAN:
  - Validator regenerates snapshot twice and byte-compares.
  - Validator enforces whitelist-only reads and stable sorting invariants.
  - `just post-work` provides deterministic manifest linking changes to packet/spec anchors.
- ERROR_TAXONOMY_PLAN:
  - `WHITELIST_VIOLATION` (attempt to read non-whitelisted path)
  - `INPUT_MISSING` (required canonical file missing)
  - `SCHEMA_MISMATCH` (missing required fields / wrong types)
  - `NONDETERMINISTIC_OUTPUT` (byte mismatch across runs)
  - `TIMESTAMP_DETECTED` (snapshot contains time-like fields/text)
  - `GATE_SUMMARY_LEAK` (validator gate summaries include timestamps/raw logs)
- UI_GUARDRAILS:
  - N/A (CLI-only deterministic generator/validator)
- VALIDATOR_ASSERTIONS:
  - Snapshot bytes are stable: `JSON.stringify(obj, null, 2) + \"\\n\"`
  - No timestamps / wall-clock fields present
  - Inputs are whitelist-only and fully hashed in `inputs[]`
  - `wp_gate_summaries` is a list and omits timestamps/raw logs
  - Sorting invariants hold (per spec `#### 7.5.4.10`)

## IMPLEMENTATION
- Added deterministic generator: `.GOV/scripts/governance-snapshot.mjs`.
  - Whitelist-only reads for canonical `.GOV/**` governance inputs (no repo scan; no extras).
  - Stable sorting for all collections per spec.
  - Output bytes: `JSON.stringify(snapshot, null, 2) + "\\n"` to `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`.
  - `git.head_sha` is supported but omitted by default (enabled only via `--include-head-sha`).
- Added deterministic validator: `.GOV/scripts/validation/validator-governance-snapshot.mjs`.
  - Generates twice and byte-compares.
  - Validates minimum schema + whitelist inputs + "no timestamps/raw logs" invariants.
  - Exits nonzero on any mismatch.
- Added `just` command surface:
  - `just governance-snapshot`
  - `just validator-governance-snapshot`
- Generated canonical output: `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`.

## HYGIENE
- Ran:
  - `just governance-snapshot` (twice)
  - `just validator-governance-snapshot`
- Logs (not committed): `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `justfile`
- **Start**: 1
- **End**: 230
- **Line Delta**: 7
- **Pre-SHA1**: `fe32108fe7f70f1d7650f1b4fff448acf719636d`
- **Post-SHA1**: `bb9952f5d9423a32c7bb9fa360ffa72fddd5325c`
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
- **Lint Results**:
- **Artifacts**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: Added `just governance-snapshot` and `just validator-governance-snapshot` command surface.

- **Target File**: `Handshake_Master_Spec_v02.125.md`
- **Start**: 1
- **End**: 62680
- **Line Delta**: 62680
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `d16eb1eb5045e858112b2ce477f27aa0200621b0`
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
- **Lint Results**:
- **Artifacts**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- **Notes**: New spec version with `#### 7.5.4.10 Product Governance Snapshot (HARD)`.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Implemented; ready for Validator review.
- What changed in this update:
  - Implemented generator + validator + `just` recipes for deterministic Product Governance Snapshot.
  - Generated `.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json` with `git: {}` default.
- Next step / handoff hint:
  - BLOCKED: `just post-work ...` currently fails due to a SyntaxError in `.GOV/scripts/validation/post-work-check.mjs` (invalid regex literal). Requires explicit scope override to patch that file.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "`just governance-snapshot` writes bytes exactly `JSON.stringify(obj, null, 2) + \"\\n\"`"
  - EVIDENCE: `.GOV/scripts/governance-snapshot.mjs:334`
  - REQUIREMENT: "Generator reads ONLY whitelist inputs; hard-fail on non-whitelisted reads"
  - EVIDENCE: `.GOV/scripts/governance-snapshot.mjs:67`
  - REQUIREMENT: "Whitelist enforcement (WHITELIST_VIOLATION) is a hard error"
  - EVIDENCE: `.GOV/scripts/governance-snapshot.mjs:250`
  - REQUIREMENT: "schema_version is explicit: hsk.product_governance_snapshot@0.1"
  - EVIDENCE: `.GOV/scripts/governance-snapshot.mjs:19`
  - REQUIREMENT: "git provenance defaults to git: {} (head_sha omitted unless explicitly enabled)"
  - EVIDENCE: `.GOV/scripts/governance-snapshot.mjs:292`
  - REQUIREMENT: "Validator generates twice and byte-compares outputs"
  - EVIDENCE: `.GOV/scripts/validation/validator-governance-snapshot.mjs:199`
  - REQUIREMENT: "Validator enforces no timestamps/raw logs in snapshot output"
  - EVIDENCE: `.GOV/scripts/validation/validator-governance-snapshot.mjs:46`
  - REQUIREMENT: "Command surface exists: `just governance-snapshot` / `just validator-governance-snapshot`"
  - EVIDENCE: `justfile:176`

## EVIDENCE
- COMMAND: `just governance-snapshot`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/governance-snapshot-1.log` (not committed)
  - LOG_SHA256: `33229c88f27fd852b633bdaa55bc58d6ea246183e96293b84c03df9be0ecd4fc`
  - PROOF_LINES:
    - `Wrote: .GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`
- COMMAND: `just governance-snapshot`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/governance-snapshot-2.log` (not committed)
  - LOG_SHA256: `33229c88f27fd852b633bdaa55bc58d6ea246183e96293b84c03df9be0ecd4fc`
  - PROOF_LINES:
    - `Wrote: .GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`
- COMMAND: `just validator-governance-snapshot`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/validator-governance-snapshot.log` (not committed)
  - LOG_SHA256: `1c8af2856b35449288d63ed83bab5039811e9c5234f666297070dcd3592b1915`
  - PROOF_LINES:
    - `OK: .GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json`
- COMMAND: `just pre-work WP-1-Product-Governance-Snapshot-v3`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/pre-work.log` (not committed)
  - LOG_SHA256: `bba08fef9102370c8408039a8962c19ea6a55c9466887bce3b3096f1a772a832`
  - PROOF_LINES:
    - `Pre-work validation for WP-1-Product-Governance-Snapshot-v3...`
- COMMAND: `just post-work WP-1-Product-Governance-Snapshot-v3 --range bbf2a67308cd3706056db2b7e3e74cd21c07dbe7..HEAD`
  - EXIT_CODE: 1
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/post-work.log` (not committed)
  - LOG_SHA256: `dd91f8ea30653417ac9ab9c1117be217f3f4c536bdab0c7542bd70f46d25c143`
  - PROOF_LINES:
    - `SyntaxError: Invalid regular expression`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
