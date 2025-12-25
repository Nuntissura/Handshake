# Spec-Driven Multi-Agent Workflow (Project-Agnostic)

## Scope and Inputs
- Applies to any local or remote repo that uses four agents: Orchestrator, Coder, Validator, Tool Agent (mechanical/tool-calling).
- Assumes the following files exist (rename as needed): `docs/SPEC_CURRENT.md`, `docs/ORCHESTRATOR_PROTOCOL.md`, `docs/CODER_PROTOCOL.md`, `docs/VALIDATOR_PROTOCOL.md`, `docs/TASK_BOARD.md`, `docs/SIGNATURE_AUDIT.md`, `docs/task_packets/`, `scripts/validation/`, `justfile`.
- Replace project-specific names, versions, and paths with your own; the workflow and templates are intentionally generic.

## Core Principles

**1. Spec as Single Source of Truth**
- `docs/SPEC_CURRENT.md` is a pointer file naming the active Master Spec (e.g., `Master_Spec_v1.0.md`)
- Master Spec uses hierarchical SPEC_ANCHOR format (e.g., `A2.3.12.3`)
- Only Main Body sections are binding authority; Roadmap is optional reference
- Every work packet must cite a precise SPEC_ANCHOR that exists in the current spec; ambiguous anchors must be escalated, not guessed
- Spec change workflow: enrich a new version + update SPEC_CURRENT.md + run spec regression validator + obtain signature before use

**2. Signatures Create Forced Alignment**
- User-issued signature `{username}{DDMMYYYYHHMM}` (e.g., `alice_25121512345`) is required before spec enrichment OR packet creation
- Signature is a **collaborative pause**: Orchestrator proposes, user validates, signature locks the agreement (DONE_MEANS, TEST_PLAN, scope, risk tier)
- Signatures are **one-time-use only**, recorded in `docs/SIGNATURE_AUDIT.md`, verified via grep: `grep -r "{signature}" . | grep -c signature_audit.md` must equal 1
- Prevents autonomous drift; every significant decision has a human decision point

**3. Work is Packetized (One Requirement per Packet)**
- One logical requirement = one task packet (WP-{phase}-{name}.md)
- Packet contains 10 required fields: TASK_ID, STATUS, SPEC_ANCHOR, scope, DONE_MEANS, TEST_PLAN, BOOTSTRAP, ROLLBACK_HINT, VALIDATION, Signature Log
- Packets are **immutable after signature** (USER_SIGNATURE field freezes content); changes require `-v2` variant with fresh signature
- Prevents scope creep; enforces clear work boundaries

**4. State Consistency (Packet ↔ Task Board)**
- Task Board and packet STATUS must always match (Ready-for-Dev, In-Progress, Done, etc.)
- State changes are **atomic**: update both packet STATUS and task board entry together
- Prevents state drift; task board is always authoritative

**5. Evidence or It Does Not Exist**
- Every DONE_MEANS requirement must have:
  - **Code evidence**: specific file:line where the requirement is implemented
  - **Test proof**: command that fails if code is removed (removability check)
  - **Spec anchor**: which section governs this requirement
- Validator audits evidence deterministically; "looks good" is not evidence
- Traceability enables post-mortem analysis, regression detection, feature removal safety

## Roles & Responsibilities

### Orchestrator (Lead Architect / Engineering Manager)
- **Owns translation** of user intent and spec requirements into immutable task packets; does not write code
- **Pre-orchestration gates**: verify spec currency and regression, task board freshness, supply chain health (`cargo deny`, `npm audit`), governance file currency, dependency chains
- **Signature pause protocol**: before packet creation, propose DONE_MEANS (5-10 measurable checkpoints), TEST_PLAN commands, IN/OUT scope (exact file paths), RISK_TIER with justification, validator audit scope, rollback plan, packet variant triggers
- **Packet creation** (via `just create-task-packet WP-{id}`): fill all 10 required fields with zero placeholders; enforce SPEC_ANCHOR references; scope to exact files (not globs); one requirement per packet
- **Verification before delegation**: run `just pre-work WP-{id}` (blocks on failure); update `docs/TASK_BOARD.md` and packet STATUS in lockstep; lock packets with USER_SIGNATURE
- **Handoff**: provide packet path, WP_ID, RISK_TIER, authority docs (SPEC_CURRENT, protocols), and readiness confirmation; maintain SLAs (no downstream work until blockers are VALIDATED)

### Coder / Debugger
- **Refuses to start** without a complete task packet; performs scope adequacy check (can I identify all affected files? Are boundaries clear? Are there unexpected dependencies?)
- **Outputs BOOTSTRAP** (FILES_TO_OPEN 5-15, SEARCH_TERMS 10-20, RUN_COMMANDS 3-6, RISK_MAP 3-8) before first code change; moves WP to In Progress on Task Board
- **Implements strictly within IN_SCOPE_PATHS**, honoring DONE_MEANS and OUT_OF_SCOPE; enforces hard invariants (zero speculative requirements, no TODO placeholders without tracking IDs)
- **Validation order**: TEST_PLAN commands (cargo test, pnpm test, curl, etc.), then `just ai-review` for MEDIUM/HIGH risk, then `just post-work WP-{id}`. Each DONE_MEANS must have file:line evidence and test proof
- **Updates packet** with VALIDATION block (Command, Result, Notes), maintains Task Board sync, prepares commit message referencing WP_ID; no commits without passing post-work gate

### Validator (Senior Engineer / Lead Auditor)
- **Blocks merges** unless evidence proves alignment with spec, codex, and packet; preserves collaboration context in packets
- **Pre-flight checks**: confirm packet completeness (all 10 fields), spec version match, USER_SIGNATURE unchanged and valid, STATUS present, BOOTSTRAP present, TEST_PLAN concrete
- **Spec extraction**: enumerate every MUST/SHOULD from DONE_MEANS + spec anchors; map each to file:line evidence; missing evidence = FAIL
- **Hygiene gates**: reject hollow code, JSON blobs where types required, unwrap/expect/panic in production without waiver; enforce Zero Placeholder Policy; run `just validator-scan`
- **Targeted audits**: storage DAL (trait boundary, SQL portability, migration numbering, dual-backend readiness), LLM boundary, determinism/traceability (trace_id/job_id in mutations), security/RCE guardrails, git/build hygiene
- **Binary verdict**: PASS (evidence complete, audits clean, tests pass) appends validation report to packet and moves to Done; FAIL (gaps found) documents violations and returns to Orchestrator/Coder

### Tool Agent (Mechanical Execution)
- **Executes allowed commands** (tests, scans, formatters, linters, file searches) under an allowlist defined in the packet's Tooling section
- **Works within bounds**: respects IN_SCOPE_PATHS, honors OUT_OF_SCOPE, respects RISK_TIER constraints (no exploratory changes)
- **Returns deterministic logs**: command + output + exit code; logs attached to packet's VALIDATION section for reproducibility
- **Reduces context churn**: keeps Coder/Validator prompts shorter by executing mechanical operations independently

## End-to-End Flow (6 Stages)

**Stage 1: Intake & Coverage Check**
- User prompt arrives; Orchestrator extracts explicit requirements and implied constraints (treat prompt as proto-spec)
- Run "clearly covers" 5-point test against spec Main Body (does it cover scope, risks, success criteria, dependencies, rollback?)
- If any test fails, request clarification and enrich spec with new version + signature before proceeding

**Stage 2: Spec Anchoring & Signature Pause**
- Orchestrator maps each requirement to a SPEC_ANCHOR; if no anchor exists, enrich spec (new version, update SPEC_CURRENT.md, run `just validator-spec-regression`)
- **Signature pause (user-orchestrator alignment)**: Orchestrator proposes:
  - 5-10 DONE_MEANS (measurable yes/no checkpoints, not aspirational)
  - TEST_PLAN commands (executable proof for each DONE_MEANS)
  - IN_SCOPE_PATHS (exact files, 5-20 specific paths)
  - OUT_OF_SCOPE (explicit deferrals with reasons)
  - RISK_TIER (LOW/MEDIUM/HIGH with justification)
  - Validator audit scope (which audits required?)
  - Rollback plan (if implementation fails, how to recover?)
  - Packet variant triggers (scope creep handling)
- User validates: interpretation accuracy? TEST_PLAN feasible? Scope realistic? Risk acceptable? Cost/benefit aligned?
- User provides signature `{username}{DDMMYYYYHHMM}`; Orchestrator records in packet's Signature & Enrichment Log

**Stage 3: Packetization & Pre-Flight**
- Orchestrator creates task packet (via `just create-task-packet WP-{id}`) with all 10 fields (no placeholders)
- Runs `just pre-work WP-{id}` (gates on completeness)
- Updates `docs/TASK_BOARD.md` to Ready for Dev (atomic update: both packet and board)
- Issues handoff to Coder: packet path, WP_ID, RISK_TIER, authority docs, readiness confirmation

**Stage 4: Coder Implementation Phase**
- Coder reads task packet, performs scope adequacy check (identifies all affected files, validates boundaries, checks for hidden dependencies)
- Outputs BOOTSTRAP block before first change (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)
- Implements strictly within IN_SCOPE_PATHS, honoring DONE_MEANS and OUT_OF_SCOPE (no speculative requirements)
- Executes TEST_PLAN commands (copy-paste-ready bash commands from packet)
- For MEDIUM/HIGH risk: runs `just ai-review`
- Records validation evidence in packet's VALIDATION block (Command, Result, Notes)
- Runs `just post-work WP-{id}` (gates on completion)
- Updates packet STATUS to Ready-for-Validation + updates Task Board atomically
- Prepares commit message referencing WP_ID

**Stage 5: Validator Audit Phase**
- Validator reads packet, performs pre-flight: completeness? Spec match? USER_SIGNATURE valid? BOOTSTRAP present?
- Extracts every MUST/SHOULD from DONE_MEANS + spec anchor sections
- Maps each requirement to file:line code evidence
- Runs test commands to verify evidence (removability check: does test fail if code deleted?)
- Runs `just validator-scan` (forbidden patterns), custom audits (DAL, error codes, traceability, security, etc.)
- Records findings in new VALIDATION REPORT block (Verdict: PASS/FAIL, Scope, Findings, Tests, Risks/Gaps, Packet Updates)
- PASS: appends report to packet, updates packet STATUS to Done, updates Task Board to Done
- FAIL: documents gaps and returns to Orchestrator/Coder for rework

**Stage 6: Orchestrator Finalization**
- Publishes mechanical output to user: work summary + validation status + what changed
- Files validation report inside the original packet (immutable record)
- Updates Task Board entry to Done (with VALIDATED marker if applicable)
- Closes work item; issues SLA tracking for next phase

---

## Task Packet Lifecycle & State Flow

**State Transitions**:
```
Backlog → Ready-for-Dev → In-Progress → Ready-for-Validation → Done (VALIDATED)
   ↓          ↓               ↓                  ↓                  ↓
(waiting)  (queued)      (active)         (awaiting audit)      (closed)
```

**Status Rules**:
- **Backlog**: packet created but not spec-ready; waiting for enrichment or dependency resolution
- **Ready-for-Dev**: fully spec'd, signature obtained, `just pre-work` passed; awaiting coder assignment
- **In-Progress**: coder actively implementing; packet shows BOOTSTRAP + validation evidence
- **Ready-for-Validation**: implementation complete; `just post-work` passed; awaiting validator review
- **Done (VALIDATED)**: validator issued PASS; validation report appended to packet; task board entry shows VALIDATED

**Immutability & Variants**:
- Once USER_SIGNATURE is recorded, packet content is frozen
- Any change after signature requires creating a `-v2` variant with fresh signature and new collaboration notes
- Old packet versions remain in git history for audit trail

**SLAs & Escalation**:
- **Backlog >5 days**: escalate; dependency or prioritization issue
- **Ready-for-Dev >10 days**: escalate; insufficient coder capacity
- **In-Progress >30 days**: escalate; scope creep or complexity underestimated
- **Ready-for-Validation >3 days**: escalate; validator queue blocked

**Dependency Enforcement**:
- Task Board explicitly lists blockers: "WP-2-Feature-B blocked on WP-1-Foundation"
- Downstream packets remain BLOCKED until upstream STATUS is VALIDATED
- `just validator-phase-gate {PHASE}` prevents phase closure if blockers unresolved

## Signature Pause (Alignment & Enrichment)
- **Orchestrator proposes**: SPEC_ANCHORs or enrichment needed, 5-10 DONE_MEANS (measurable), TEST_PLAN commands, IN/OUT scope, RISK_TIER, validator audit scope, rollback plan, variant triggers.
- **User validates**: interpretation accuracy, feasibility of commands, scope realism, risk acceptance, cost/benefit.
- **Signature block (recorded in packet)**:
  ```
  ## Signature & Enrichment Log
  Signature: alice_25121512345 (immutable)
  Timestamp: 2025-12-15 12:34:56 UTC
  Notes:
  - Clarified: OAuth flow required; DONE_MEANS updated to include callback test.
  - Spec Enrichment: Added A4.2.7 OAuth; SPEC_CURRENT updated.
  - Validator Scope: DAL audit skipped; LLM boundary + security required.
  Locked: DONE_MEANS, TEST_PLAN, IN_SCOPE_PATHS, OUT_OF_SCOPE, validator checks.
  ```
- Any change after signature => create `WP-{id}-v2` with a new signature.

## Templates (Copy, then fill)

### Task Packet (10 Required Fields)
```
# Task Packet: WP-{phase}-{name}
- TASK_ID: WP-{phase}-{name}
- STATUS: Ready-for-Dev | In-Progress | Ready-for-Validation | Done | Backlog
- SPEC_ANCHOR: {e.g., A2.3.12.3}
- What: {1-2 sentences}
- Why: {rationale + spec reference}
- IN_SCOPE_PATHS:
  - {exact file path 1}
  - {exact file path 2}
- OUT_OF_SCOPE:
  - {explicit deferral + reason}
- DONE_MEANS: 5-10 measurable checkpoints (yes/no), each mapped to SPEC_ANCHOR
- TEST_PLAN:
  - {command 1}
  - {command 2}
  - {ai-review if medium/high}
- BOOTSTRAP (Coder work plan):
  - FILES_TO_OPEN: 5-15 files
  - SEARCH_TERMS: 10-20 grep strings
  - RUN_COMMANDS: 3-6 setup/test commands
  - RISK_MAP: 3-8 failure modes -> subsystems
- ROLLBACK_HINT: {git revert <sha> or explicit steps}
- VALIDATION (filled after work):
  - Command:
  - Result:
  - Notes:
- Signature & Enrichment Log: {signature block}
```

### Task Board Entry
```
## Phase {N} Closure Gates (Blocking)
- [WP-1-Storage-Layer] Ready-for-Dev (no blockers)
- [WP-1-AppState-Refactor] Blocked on WP-1-Storage-Layer

## In Progress
- [WP-1-Storage-Layer]

## Ready for Dev
- [WP-1-Migration-Framework]
```

### Orchestrator Handoff (to Coder)
```
Task Packet: docs/task_packets/WP-{id}.md
WP_ID: WP-{id}
RISK_TIER: LOW|MEDIUM|HIGH

Read: ORCHESTRATOR_PROTOCOL.md, CODER_PROTOCOL.md, SPEC_CURRENT.md
Run: just pre-work {WP_ID}
Deliver: BOOTSTRAP block before first change.
Scope is locked to IN_SCOPE_PATHS; OUT_OF_SCOPE is forbidden.
DONE_MEANS and TEST_PLAN are frozen by signature.
```

### Coder BOOTSTRAP Block (before coding)
```
BOOTSTRAP
WP_ID: WP-{id}
RISK_TIER: MEDIUM
TASK_TYPE: FEATURE|DEBUG|REFACTOR|HYGIENE
FILES_TO_OPEN:
- docs/START_HERE.md
- docs/SPEC_CURRENT.md
- docs/task_packets/WP-{id}.md
- src/path/file.rs
- ...
SEARCH_TERMS:
- "TraitName"
- "SqlPool"
- ...
RUN_COMMANDS:
- just dev
- cargo test --manifest-path src/backend/Cargo.toml
- pnpm -C app test
RISK_MAP:
- "Trait mismatch" -> "Storage layer"
- "API contract break" -> "Frontend/IPC"
```

### Validator Report
```
VALIDATION REPORT - WP-{id}
Verdict: PASS | FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-{id}.md (status: {status})
- Spec: {SPEC_CURRENT ref + anchors}

Findings:
- Requirement: {DONE_MEANS item} -> {path:line}; test: {command} (PASS/FAIL)
- Hygiene: {clean/issues}
- Forbidden Patterns: {results}
- Audits: {DAL/LLM/security/etc}

Tests:
- {command}: PASS/FAIL (summary)

Risks/Gaps:
- {residual risks or waivers}

Packet Update:
- STATUS set to Done|In-Progress|Blocked with reason
- Task Board updated
```

### Tool Agent Allowlist (per packet)
```
## Tooling (Allowed for Tool Agent)
- Workdir: {repo root or subdir}
- Allowed commands:
  - just pre-work {WP_ID}
  - cargo test --manifest-path ...
  - pnpm -C app test
  - rg "pattern" src/
- Output limits: {max lines/MB}
- Time limits: {per command}
- Logs: Attach output to VALIDATION section
```

### Evidence Table (inside packet)
```
## Evidence Mapping
| DONE_MEANS | SPEC_ANCHOR | Code Evidence (path:line) | Test Command |
|------------|-------------|---------------------------|--------------|
| "DB is behind trait" | A2.3.12.3 | src/storage/mod.rs:45-120 | cargo test -p core storage_tests |
| "No pool leaks" | A2.3.12.3 | src/api/*.rs imports checked | rg "SqlPool" src/api |
```

### Signature Audit Entry (project root)
```
| Signature | Used By | Timestamp | Purpose | Spec Version | Evidence |
|-----------|---------|-----------|---------|--------------|----------|
| alice_25121512345 | Orchestrator | 2025-12-15 12:34:56 UTC | Packet creation WP-1 | v02.85 | docs/task_packets/WP-1.md |
```

### Justfile Skeleton
```
create-task-packet WP_ID:
  node scripts/create-task-packet.mjs {{WP_ID}}

pre-work WP_ID:
  node scripts/validation/pre-work-check.mjs {{WP_ID}}

post-work WP_ID:
  node scripts/validation/post-work-check.mjs {{WP_ID}}

validator-scan:
  node scripts/validation/validator-scan.mjs

validator-spec-regression:
  node scripts/validation/validator-spec-regression.mjs

validate-workflow WP_ID:
  just pre-work {{WP_ID}}
  just validator-scan
  just validator-spec-regression
  just ai-review
  just post-work {{WP_ID}}
```

## Validation Scripts (Modular, One Concern Each)

**Pre-Work Gate (Gate 0)**: `pre-work-check.mjs`
- Verifies task packet file exists: `docs/task_packets/WP-{ID}.md`
- Confirms all 10 required fields present: TASK_ID, STATUS, SPEC_ANCHOR, scope, DONE_MEANS, TEST_PLAN, BOOTSTRAP, ROLLBACK_HINT, VALIDATION, Signature Log
- Scans for placeholder text (`{field_name}`, `TODO(TBD)`, `FIXME`, mock values)
- Exit 0: proceed to implementation; Exit 1: return to Orchestrator for completion
- **Purpose**: prevents handoff until packet is actionable

**Post-Work Gate (Gate 1)**: `post-work-check.mjs`
- Verifies VALIDATION section has outcomes recorded (Command, Result, Notes)
- If TEST_PLAN lists test commands, validates they're documented with results
- For MEDIUM/HIGH risk: verifies `ai-review` completed and not BLOCKED
- Checks git diff shows actual changes (work was done)
- Exit 0: work validated, safe to commit; Exit 1: incomplete, return to Coder
- **Purpose**: prevents merge until validation evidence present

**Forbidden Pattern Scanner**: `validator-scan.mjs`
- Scans production code for patterns that indicate incomplete work or unsafe patterns
- **Forbidden** (exceptions allowed in tests only): `unwrap`, `expect(`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`
- **Placeholder patterns** (anywhere): `Mock`, `Stub`, `placeholder`, `hollow`, `{field}`
- **Configurable** per project: scan paths, allowed exceptions
- Exit 0: clean; Exit 1: patterns found, lists all occurrences

**Spec Regression Validator**: `validator-spec-regression.mjs`
- Gates spec integrity and phase progression
- Checks: `docs/SPEC_CURRENT.md` exists and is readable
- Verifies points to valid spec file (e.g., `Master_Spec_v1.0.md`) in repo root
- Validates spec file contains all REQUIRED_ANCHORS (project-defined list):
  ```
  const REQUIRED_ANCHORS = [
    "A2.3.12",    // Example: Storage portability
    "A2.3.11",    // Example: Retention/GC
    "A2.6.7"      // Example: Semantic catalog
  ];
  ```
- Exit 0: spec valid; Exit 1: file missing or anchor gap
- **Purpose**: ensures spec coherence; blocks phase progression on regression

**Custom Auditors** (architecture-specific):
- `validator-dal-audit`: storage layer boundary (no direct DB access outside modules, SQL portability, trait isolation, migration hygiene)
- `validator-error-codes`: typed errors + traceability (no stringly errors, HSK-#### codes, trace IDs in mutations)
- `validator-traceability`: determinism markers (trace_id, job_id, request_id) in mutation points
- `validator-git-hygiene`: .gitignore coverage, no artifacts committed (target/, *.pdb, node_modules)
- `validator-coverage-gaps`: test coverage thresholds (<80% flagged)
- `validator-security`: RCE guardrails, input validation, secret detection

**Phase Gate Validator**: `validator-phase-gate.mjs`
- Reads `docs/TASK_BOARD.md` "Phase {N} Closure Gates (BLOCKING)"
- Verifies every listed blocking WP: status = Done AND VALIDATED
- Runs `just validator-spec-regression` (spec must be current for phase closure)
- Checks no unresolved dependencies (all upstream = VALIDATED)
- Exit 0: clear to proceed; Exit 1: blockers remain
- **Purpose**: gates phase progression; prevents moving forward with incomplete foundations

## Context Window & Prompt Hygiene (Mitigating Context Rot)

**Spec Anchoring Prevents Drift**:
- All instructions live in durable files (spec version, packet, task board), not ephemeral chat history
- Refer to SPEC_ANCHOR IDs instead of pasting entire specs (e.g., "per A2.3.12, implement trait-based storage" vs. entire storage section)
- Spec Signature freezes intent; changes require new signature, creating audit trail

**Retrieval-Style Prompts** (for each agent call):
- Supply only relevant spec slices: read SPEC_ANCHOR sections directly, not full spec
- Include the full task packet (includes Signature Log + DONE_MEANS + TEST_PLAN + BOOTSTRAP)
- Attach latest VALIDATION block (what has been verified so far)
- Keep total context <20KB for Coder/Validator calls; larger context for Orchestrator (who coordinates)

**Short & Structural Prompts**:
- Refer to packet fields by name + ID ("per DONE_MEANS item 3, verify file:line evidence")
- Cite SPEC_ANCHOR IDs instead of explaining requirements ("implement per A2.3.12.3" vs. "implement a trait-based storage layer that...")
- Use packet templates + signature log to avoid re-explaining context every turn

**Long-Running Packets (>5 days)**:
- Have Orchestrator issue brief state digests: current scope, open questions, last validation results, next expected step
- Digest replaces verbose transcript; agents read digest + packet, not entire conversation
- Reduces need to context-search through 50+ turns of negotiation

**Large-Context Models**:
- Even long-context models benefit from hierarchical prompting (anchor IDs + slices)
- Structured logs (validation blocks, BOOTSTRAP, command outputs) are safer for reuse than raw transcripts
- Logs are deterministic and auditable; transcripts drift

**Example Prompt (for Coder)**:
```
WP_ID: WP-1-Storage-Layer
SPEC_ANCHOR: A2.3.12.3
Read: docs/SPEC_CURRENT.md sections A2.3.12.3 + A2.3.12.1
Read: docs/task_packets/WP-1-Storage-Layer.md (full packet)
DONE_MEANS:
  1. "AppState exposes Database as Arc<dyn Trait>"
  2. "No SqlitePool leaks to API layer"
TEST_PLAN: cargo test -p core storage_tests; grep "SqlPool" src/api | wc -l
Task: Implement DONE_MEANS per packet; record evidence; run TEST_PLAN.
```

## Git Hook Integration (Pre-Commit Enforcement)

**File**: `scripts/hooks/pre-commit`

**Purpose**: Prevent broken commits; enforce codex/governance invariants before code enters tree

**Sample Implementation**:
```bash
#!/bin/bash
set -e

# 1. Verify hard invariants (governance rules)
node scripts/validation/codex-check.mjs

# 2. Warn about TODOs without tracking IDs
grep -r "TODO()" src/ app/ && \
  echo "❌ TODOs must have tracking ID: TODO(HSK-####)" && exit 1

# 3. Forbid placeholder values in code
grep -r "{" src/ app/ --include="*.rs" --include="*.ts" --include="*.tsx" && \
  echo "⚠️  Found placeholder text in code" && exit 1

# 4. Lint and format
cargo fmt --check || exit 1
pnpm run lint --quiet || exit 1

echo "✅ Pre-commit checks passed"
```

**Wire via**:
```bash
git config core.hooksPath scripts/hooks
```

**Effect**: Commit blocked until checks pass; catches errors before pushing to remote

---

## Documentation Structure (Project-Agnostic)

Create `docs/` directory with:

```
docs/
├── START_HERE.md              # Entry point; repo map; AI agent workflow
├── SPEC_CURRENT.md            # Pointer: "Current spec is Master_Spec_vX.Y.Z.md"
├── ARCHITECTURE.md            # Module layout, responsibilities, entry points, RDD
├── RUNBOOK_DEBUG.md           # Error codes, bug triage, debug patterns
├── QUALITY_GATE.md            # Risk tier definitions, Gate 0/1 requirements
├── TASK_BOARD.md              # Master task status + Phase gates + dependencies
├── SIGNATURE_AUDIT.md         # Immutable registry of consumed signatures
├── OWNERSHIP.md               # Path/area ownership (optional)
├── task_packets/
│   ├── README.md              # Packet naming convention, validation commands
│   ├── TEMPLATE.md            # Copy-paste packet template
│   └── WP-1-*.md, WP-2-*.md   # (20-30 actual packets during active work)
├── adr/                       # Architecture Decision Records
│   ├── ADR-0001-workflow.md
│   └── ADR-0002-spec-governance.md
└── agents/
    └── AGENT_REGISTRY.md      # Map of contributing agents/models
```

**Master Spec location** (repo root):
```
Master_Spec_v1.0.md            # Current authoritative spec (~1MB+)
Master_Spec_v0.9.md            # (Previous versions for audit trail)
{Project} Codex v1.0.md        # Governance rules (versioned)
{Project} Codex v0.9.md
```

---

## Phase Gate Pattern (Blocking Phase Progression)

**Command**: `just validator-phase-gate {PHASE_ID}`

**What It Does**:
1. Reads `docs/TASK_BOARD.md` section "Phase {PHASE_ID} Closure Gates (BLOCKING)"
2. Verifies every listed blocking WP: STATUS = Done AND VALIDATED
3. Runs `just validator-spec-regression` (spec must be current)
4. Checks no unresolved dependencies (all upstream = VALIDATED)
5. Returns exit 0 (clear to proceed) or exit 1 (blockers remain)

**Usage Example**:
```bash
# Before closing Phase 1, run:
just validator-phase-gate Phase-1

# Output:
# ✅ Phase 1 Closure Gates check...
# ❌ BLOCKED: WP-1-Storage-Layer (status: Done, not VALIDATED)
# ❌ BLOCKED: WP-1-AppState-Refactor (status: In-Progress)
# Exit 1

# Once all gates pass:
# ✅ All blocking WPs VALIDATED
# ✅ Spec regression check passed
# ✅ No unresolved dependencies
# Exit 0
```

**Gates Phase Progression** (in CI/CD or release script):
```bash
if just validator-phase-gate Phase-1; then
  echo "✅ Phase 1 ready to close"
  git tag phase-1-complete
  git push origin phase-1-complete
else
  echo "❌ Blockers remain; cannot close phase"
  echo "Run: just TASK_BOARD.md to review blocking WPs"
  exit 1
fi
```

**Why This Matters**:
- Prevents moving forward with incomplete foundations
- Enforces spec coherence (regression check)
- Makes dependency chains explicit
- Blocks merges to main/release until gates pass

---

## Making It Project-Agnostic
- Swap `SPEC_ANCHOR` format, file paths, and command runners to match your stack (e.g., `go test ./...`, `npm test`, `pytest`).
- Rename risk tiers or add tiers; adjust validator scripts to your invariants (e.g., API contract checks, schema checks).
- Replace LLM/LLM-boundary rules with your own hard invariants (e.g., no direct DB access, no network calls in handlers).
- Update forbidden patterns to reflect your language/tooling.

## Quick Start Checklist (new repo)
1) Create `docs/SPEC_CURRENT.md` pointing to your spec file.
2) Add packet template under `docs/task_packets/`.
3) Create `docs/TASK_BOARD.md` with columns and phase gates.
4) Add `docs/SIGNATURE_AUDIT.md` (empty table).
5) Add validator scripts under `scripts/validation/`; wire into `justfile`.
6) Add `scripts/create-task-packet.mjs` (or equivalent generator).
7) Add git hook (`scripts/hooks/pre-commit`) for invariants and linting.
8) Train agents on Orchestrator/Coder/Validator protocols and the signature pause.
9) Run `just pre-work WP-{id}` before handoff; block on any failure.
10) Require signature pause before enrichment or packet creation; log every signature.
