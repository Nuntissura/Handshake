# Task Packet: WP-1-Product-Governance-Snapshot-v3

## METADATA
- TASK_ID: WP-1-Product-Governance-Snapshot-v3
- WP_ID: WP-1-Product-Governance-Snapshot-v3
- BASE_WP_ID: WP-1-Product-Governance-Snapshot (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-06T13:40:41.412Z
- MERGE_BASE_SHA: 85e20bf1071facd9b7e89e2777203f60b1b59b7c
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
- NONE

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
just post-work WP-1-Product-Governance-Snapshot-v3 --range 85e20bf1071facd9b7e89e2777203f60b1b59b7c..HEAD
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
  git diff --name-only 85e20bf1071facd9b7e89e2777203f60b1b59b7c..HEAD
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
- (Coder fills after skeleton approval.)

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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

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
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Snapshot-v3/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
