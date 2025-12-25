# Spec-Driven Multi-Agent Workflow (Complete Implementation Guide)

## Scope and Inputs

This document is **complete, standalone, and implementable by a fresh model** for any large software project using multi-agent AI workflows (Orchestrator, Coder, Validator, Tool Agent).

**What this covers**:
- Core concepts and 5 guiding principles
- Detailed roles and responsibilities
- 6-stage end-to-end flow
- Complete file structure and templates
- Real working examples (filled-in packets, validator output, etc.)
- Governance rules template (Codex)
- Protocol file templates
- Validator script implementations (samples + patterns)
- Phased rollout and implementation order
- Integration with existing projects
- Troubleshooting and common failures
- Testing strategy for the workflow itself

**Files you will create** (customize names as needed):
- `docs/SPEC_CURRENT.md` - Pointer to active spec
- `Master_Spec_v1.0.md` - The authoritative specification
- `{Project} Codex v1.0.md` - Governance rules
- `docs/ORCHESTRATOR_PROTOCOL.md` - Orchestrator workflow
- `docs/CODER_PROTOCOL.md` - Coder workflow
- `docs/VALIDATOR_PROTOCOL.md` - Validator workflow
- `docs/TASK_BOARD.md` - Master task tracking
- `docs/SIGNATURE_AUDIT.md` - Signature registry
- `docs/task_packets/TEMPLATE.md` - Packet template
- `docs/task_packets/WP-*.md` - Actual work packets
- `scripts/validation/*.mjs` - Validator scripts (10-15 files)
- `scripts/hooks/pre-commit` - Git hook
- `justfile` - Command wiring

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

## Implementation Order (Phased Rollout)

**DO NOT build everything at once.** Use this phased approach so each phase unblocks the next.

### Phase 0: Foundation (Days 1-2)
**Goal**: Create the spec infrastructure and first task packet template.

1. Create `Master_Spec_v1.0.md` (can start minimal: ~20KB with 5-10 sections and anchors)
2. Create `docs/SPEC_CURRENT.md` (pointer file, 3 lines)
3. Create `docs/SIGNATURE_AUDIT.md` (empty table, 5 lines)
4. Create `docs/TASK_BOARD.md` (empty structure, 10 lines)
5. Create `docs/task_packets/TEMPLATE.md` (copy from Templates section below, 50 lines)
6. Create `scripts/create-task-packet.mjs` (simple Node script: generate packet from template)
7. Test: `node scripts/create-task-packet.mjs WP-0-Test` → produces `docs/task_packets/WP-0-Test.md`

**Deliverable**: You can now create task packets.

### Phase 1: Validation Gates (Days 3-4)
**Goal**: Enforce packet completeness before handoff.

1. Create `scripts/validation/pre-work-check.mjs` (verify 10 required fields, no placeholders)
2. Create `scripts/validation/post-work-check.mjs` (verify VALIDATION results recorded)
3. Create `scripts/validation/validator-spec-regression.mjs` (verify spec file exists + anchors)
4. Create `scripts/validation/validator-scan.mjs` (grep for forbidden patterns)
5. Create `justfile` with commands:
   - `just create-task-packet WP_ID`
   - `just pre-work WP_ID`
   - `just post-work WP_ID`
   - `just validator-scan`
   - `just validator-spec-regression`
6. Test:
   - `just create-task-packet WP-1-Feature` → creates packet
   - `just pre-work WP-1-Feature` → FAIL (has placeholders)
   - Edit packet, remove placeholders
   - `just pre-work WP-1-Feature` → PASS

**Deliverable**: Packet quality gates work; you can't hand off incomplete packets.

### Phase 2: Governance Rules (Days 5-6)
**Goal**: Define hard invariants for your project.

1. Create `{Project} Codex v1.0.md` (copy template from section below; ~40KB)
   - Define 10-20 hard invariants (CX-101, CX-102, etc.)
   - Examples: "LLM calls only via module X", "No direct DB access outside storage", "All errors typed, not strings"
2. Create `scripts/validation/codex-check.mjs` (grep for codex violations)
3. Create `scripts/hooks/pre-commit` (wire codex-check + lint + tests)
4. Wire hook: `git config core.hooksPath scripts/hooks`
5. Test: Try to commit code that violates a codex rule → commit blocked

**Deliverable**: Hard invariants are enforced at commit time; code quality guaranteed.

### Phase 3: Protocol Files (Days 7-8)
**Goal**: Document workflow for each agent.

1. Create `docs/ORCHESTRATOR_PROTOCOL.md` (copy template + customize, ~50KB)
   - Pre-orchestration gates checklist
   - Signature pause protocol
   - Packet creation workflow
2. Create `docs/CODER_PROTOCOL.md` (copy template + customize, ~30KB)
   - Pre-coding checklist (scope adequacy)
   - Validation order (TEST_PLAN → ai-review → post-work)
   - Evidence recording format
3. Create `docs/VALIDATOR_PROTOCOL.md` (copy template + customize, ~20KB)
   - Pre-flight checks
   - Evidence verification steps
   - Audit scope (DAL, security, hygiene, etc.)
4. Test: Train an agent on each protocol; run through a mock task packet

**Deliverable**: Agents know their workflow; can operate semi-autonomously.

### Phase 4: Custom Validators (Days 9-12)
**Goal**: Add domain-specific validation (storage DAL, API contracts, security, etc.).

1. Create `scripts/validation/validator-dal-audit.mjs` (if storage-based)
   - Check trait boundaries, SQL portability, migration hygiene
2. Create `scripts/validation/validator-error-codes.mjs`
   - Enforce typed errors, trace ID logging, no stringly errors
3. Create `scripts/validation/validator-security.mjs`
   - RCE guardrails, input validation, secret detection
4. Create `scripts/validation/validator-git-hygiene.mjs`
   - .gitignore coverage, no artifacts committed
5. Create `scripts/validation/validator-phase-gate.mjs`
   - Block phase progression until all gates VALIDATED
6. Wire into `justfile`: `just validator-{concern}`, `just validator-phase-gate PHASE`
7. Test: Create a test task packet; run all validators; fix violations until PASS

**Deliverable**: Rich validation catches domain-specific issues; phase gates work.

### Phase 5: Integration & Docs (Days 13-14)
**Goal**: Make the workflow real and documented.

1. Create `docs/START_HERE.md` (entry point: repo map, AI agent workflow, links)
2. Create `docs/ARCHITECTURE.md` (module layout, responsibilities, entry points)
3. Create `docs/RUNBOOK_DEBUG.md` (error codes, debug patterns)
4. Create `docs/QUALITY_GATE.md` (risk tier definitions, Gate 0/1 requirements)
5. Create `docs/agents/AGENT_REGISTRY.md` (map of contributing agents/models)
6. Create first **real** task packet for your project's first feature
   - Use all the tools: pre-work gate, signature pause, BOOTSTRAP, etc.
   - Run full workflow: create → code → validate → merge
7. Test: Dry run the entire workflow on a low-risk feature

**Deliverable**: Workflow is live; documented; agents can operate independently.

---

## Real Working Examples

### Example 1: Complete Task Packet (Filled In)

**File**: `docs/task_packets/WP-1-User-Authentication.md`

```markdown
# Task Packet: WP-1-User-Authentication

## Metadata
- TASK_ID: WP-1-User-Authentication
- STATUS: Done (VALIDATED)
- DATE: 2025-12-15 10:00 UTC
- REQUESTOR: alice
- USER_SIGNATURE: alice_25121510000
- SPEC_VERSION: Master_Spec_v1.0.md

## What
Implement email + password authentication endpoint (POST /auth/login) returning JWT token with 24-hour TTL.

## Why
Foundation for user identity system. Spec A3.2.1 requires "Users authenticate with email+password, receive bearer token, subsequent requests validated via Bearer header."
Blocks: WP-2-Authorization-Roles, WP-3-Session-Management

## Scope
- IN_SCOPE_PATHS:
  - src/api/handlers/auth.rs
  - src/api/models/user.rs
  - src/services/jwt.rs
  - tests/integration/auth_test.rs
  - Cargo.toml (add jwt crate)
- OUT_OF_SCOPE:
  - OAuth/third-party auth (Phase 2)
  - Password reset/recovery (Phase 1.5)
  - Rate limiting (separate WP-0-Rate-Limiting)
  - Database schema changes (data layer frozen per A2.3.12)

## DONE_MEANS
1. POST /auth/login accepts {email, password} JSON
2. Valid credentials return {jwt_token, expires_in: 86400} JSON
3. Invalid credentials return 401 + error code AUTH-001
4. JWT decodes to {user_id, email, iat, exp}
5. Subsequent requests with "Authorization: Bearer {token}" header validated (token not expired, signature valid)
6. Expired token returns 401 + error code AUTH-002
7. Malformed token returns 401 + error code AUTH-003
8. Integration test: login → validate token → request with Bearer → success

## TEST_PLAN
- `cargo test -p api auth_integration` (PASS; 8 test cases)
- `curl -X POST http://localhost:3000/auth/login -H "Content-Type: application/json" -d '{"email":"test@example.com","password":"password123"}'` (returns jwt_token)
- `curl http://localhost:3000/user -H "Authorization: Bearer {token}"` (PASS, returns user data)
- `curl http://localhost:3000/user -H "Authorization: Bearer invalid"` (returns 401)
- `grep "JWT\|jwk\|token" src/api/models/user.rs | wc -l` (returns > 0, proves JWT in code)
- Code review: validate no plaintext passwords logged, no token leakage to stdout

## BOOTSTRAP (Coder's Work Plan)
- FILES_TO_OPEN:
  - docs/SPEC_CURRENT.md (section A3.2.1)
  - docs/task_packets/WP-1-User-Authentication.md (this packet)
  - src/api/handlers/mod.rs (entry point)
  - src/services/mod.rs (service structure)
  - tests/integration/auth_test.rs (test patterns)
  - Cargo.toml (dependencies)
  - docs/RUNBOOK_DEBUG.md (error codes section)
- SEARCH_TERMS:
  - "LoginRequest", "TokenResponse", "validate_jwt"
  - "user_id", "email_address"
  - "jsonwebtoken", "chrono"
  - "401", "AUTH-"
  - "Bearer", "Authorization"
- RUN_COMMANDS:
  - `cargo build --manifest-path src/Cargo.toml`
  - `cargo test -p api --test integration` (baseline)
  - `sqlx database create` (if new DB needed)
- RISK_MAP:
  - "JWT library vulnerability" → Security audit required (MEDIUM risk)
  - "Password stored plaintext" → Must use bcrypt (HIGH risk)
  - "Token expiry not enforced" → MEDIUM risk
  - "API contract mismatch" → Integration test catches (LOW risk)

## RISK_TIER
MEDIUM
- Why: Security-sensitive (passwords, tokens), but single module (no IPC yet)
- Triggers: ai-review required, security audit in validator
- Rollback: `git revert <sha>`; feature flag gated, can be disabled in config

## ROLLBACK_HINT
If authentication breaks:
1. `git revert <commit-hash>`
2. Restart server: `cargo run --bin api`
3. Test: `curl /auth/login` should return 404 (endpoint gone)

## VALIDATION (filled by Coder)

### BOOTSTRAP Output
```
WP_ID: WP-1-User-Authentication
RISK_TIER: MEDIUM
TASK_TYPE: FEATURE
FILES_TO_OPEN:
- docs/SPEC_CURRENT.md (A3.2.1)
- src/api/handlers/auth.rs (will create)
- src/services/jwt.rs (will create)
- tests/integration/auth_test.rs (will create)
- Cargo.toml (will update)
SEARCH_TERMS:
- "LoginRequest", "TokenResponse"
- "validate_jwt", "issue_token"
- "jsonwebtoken", "bcrypt"
- "401", "AUTH-"
RUN_COMMANDS:
- cargo build --manifest-path src/Cargo.toml
- cargo test -p api auth_integration
- curl -X POST http://localhost:3000/auth/login ...
RISK_MAP:
- "JWT library vulnerability" -> Security audit
- "Password stored plaintext" -> Bcrypt required
```

### Test Results
```
Command: cargo test -p api auth_integration
Result: PASSED (8/8 cases)
  ✓ login_valid_credentials
  ✓ login_invalid_password
  ✓ login_invalid_email
  ✓ token_decode_valid
  ✓ token_expired
  ✓ token_malformed
  ✓ bearer_header_validation
  ✓ bearer_token_not_found

Command: grep "bcrypt" src/services/jwt.rs | wc -l
Result: 2 (passwords are hashed)

Command: grep "println\|dbg\|password" src/services/jwt.rs | grep -v test
Result: 0 (no password logging in production)

ai-review: PASSED (security review)
```

## Signature & Enrichment Log

**Signature**: alice_25121510000 (immutable)
**Timestamp**: 2025-12-15 10:00 UTC

### User-Orchestrator Collaboration Notes:
- **Clarified**: User confirmed email+password only (OAuth deferred). DONE_MEANS expanded to include Bearer token validation in subsequent requests.
- **Spec Enrichment**: Spec section A3.2.1 already covers this fully; no enrichment needed.
- **Rubric Adjustments**: Added requirement 5 (Bearer token validation in API). TEST_PLAN now includes curl test of authenticated request. ai-review added for MEDIUM risk (security).
- **Risks Acknowledged**: User approved MEDIUM risk; rollback plan is git revert + restart. Security audit required before merge.

### Locked Intent:
- **DONE_MEANS**: [frozen above]
- **TEST_PLAN**: [frozen above]
- **IN_SCOPE_PATHS**: [frozen above]
- **OUT_OF_SCOPE**: [frozen above]
- **Validator Audit Scope**: hygiene (bcrypt, no password logging), security (JWT lib), error codes (AUTH-001, AUTH-002, AUTH-003)
```

---

### Example 2: Validator Report Output

```markdown
# VALIDATION REPORT - WP-1-User-Authentication

**Verdict**: PASS

## Scope Inputs
- Task Packet: docs/task_packets/WP-1-User-Authentication.md
- Spec: Master_Spec_v1.0.md (section A3.2.1)
- USER_SIGNATURE: alice_25121510000 (valid, not reused)

## Findings

### Requirement Mapping (Evidence Verification)
| DONE_MEANS | Spec Anchor | Code Evidence | Test Command | Status |
|------------|-------------|---------------|--------------|--------|
| "POST /auth/login accepts email+password" | A3.2.1 | src/api/handlers/auth.rs:12-35 (LoginRequest struct) | cargo test login_valid_credentials | PASS |
| "Returns jwt_token + expires_in" | A3.2.1 | src/api/handlers/auth.rs:40-52 (TokenResponse struct) | curl /auth/login | PASS |
| "Invalid credentials return 401" | A3.2.1 | src/api/handlers/auth.rs:60-75 (error handling) | cargo test login_invalid_password | PASS |
| "JWT decodes to user_id+email+iat+exp" | A3.2.1 | src/services/jwt.rs:30-60 (token claims) | cargo test token_decode_valid | PASS |
| "Bearer header validation" | A3.2.1 | src/api/middleware/auth.rs:15-40 (middleware) | cargo test bearer_header_validation | PASS |
| "Expired token rejected" | A3.2.1 | src/api/middleware/auth.rs:45-65 (exp check) | cargo test token_expired | PASS |
| "Malformed token rejected" | A3.2.1 | src/api/middleware/auth.rs:70-85 (parse error) | cargo test token_malformed | PASS |
| "Integration test: login → Bearer → success" | A3.2.1 | tests/integration/auth_test.rs:50-100 | cargo test auth_integration | PASS |

### Hygiene & Forbidden Patterns
- Forbidden patterns scan: PASS (no unwrap, expect, todo, println in src/services/jwt.rs production code)
- Placeholder check: PASS (no {field} or TODO(TBD) in code)
- Error codes: PASS (AUTH-001, AUTH-002, AUTH-003 documented in RUNBOOK_DEBUG.md)

### Security Audit
- Password hashing: PASS (bcrypt with cost 12, verified in test)
- No password logging: PASS (grep password src/services/jwt.rs returns 0 in production code)
- JWT library: PASS (jsonwebtoken v9.0, security audit OK)
- Token expiry: PASS (TTL 86400 seconds enforced)

### Test Verification
- TEST_PLAN execution: PASS (all 8 integration tests passed)
- Coverage: PASS (auth module 92% coverage, above 80% threshold)
- Removal check: PASS (removing token validation code breaks 3+ tests)

### Residual Risks / Waivers
- None. All gates passed. No waivers needed.

## Packet Update
- **STATUS**: Done (VALIDATED)
- **Task Board**: Move WP-1-User-Authentication to Done + mark VALIDATED
- **Next**: WP-2-Authorization-Roles can now proceed (was blocked on WP-1)
```

---

### Example 3: Task Board (Phase 1)

```markdown
# TASK_BOARD.md

## Phase 1 Closure Gates (BLOCKING)

Must be Done + VALIDATED before Phase 1 can close:

- [WP-1-User-Authentication] **Done (VALIDATED)** ✅
  - Blocker: None
  - Unblocks: WP-2-Authorization-Roles, WP-3-Session-Management

- [WP-1-Database-Migrations] **Done (VALIDATED)** ✅
  - Blocker: None
  - Unblocks: WP-1-Storage-Abstraction

- [WP-1-Error-Codes-Registry] **Done (VALIDATED)** ✅
  - Blocker: None
  - Unblocks: All (error codes are foundation)

- [WP-1-Logging-Framework] **Ready-for-Dev** (not yet started)
  - Blocker: None
  - Unblocks: WP-1-Observability

## In Progress

- [WP-1-Logging-Framework] Started 2025-12-16, by coder_model_v2

## Ready for Validation

- [WP-1-API-Contracts] Implementation complete, awaiting validator review (SLA: 2 days)

## Ready for Dev

- [WP-1-Observability] Spec'd, no blockers, awaiting coder assignment (SLA: 10 days)
- [WP-1-Metrics-Collection] Spec'd, awaiting blockers WP-1-Logging-Framework to clear

## Backlog

- [WP-2-Authorization-Roles] Blocked on WP-1-User-Authentication (ready when Phase 1 closes)
- [WP-2-Session-Management] Blocked on WP-1-User-Authentication
```

---

## Governance Rules (Codex) Template

**File**: `{Project} Codex v1.0.md` (~40-60KB, customize for your project)

```markdown
# {Project} Codex v1.0.md

## Hard Invariants (Governance Rules)

### CX-101: LLM Integration Boundary
**Rule**: All LLM API calls (chat, completion, embeddings) must go through `/src/backend/llm/client.rs`
**Scope**: Production code only (tests exempt)
**Rationale**: Centralized control, cost tracking, rate limiting, prompt injection prevention
**Violation**: Direct `openai_client.call(...)` or HTTP requests to LLM API outside `/src/backend/llm/`
**Waiver**: None (hard blocker)

### CX-102: Database Access Layer
**Rule**: Database queries only via `src/backend/storage/` module. No direct sqlx::query calls outside storage/.
**Scope**: Production code (tests exempt)
**Rationale**: Dual-backend readiness (SQLite → PostgreSQL), migration consistency, audit trail
**Violation**: `sqlx::query` or `.execute()` outside src/backend/storage/
**Waiver**: None (hard blocker)

### CX-103: Error Handling
**Rule**: All errors must be typed (custom enum), never string errors. Error codes prefixed {PROJECT_CODE}-#### (e.g., AUTH-001).
**Scope**: Production code (tests can use anyhow)
**Rationale**: Deterministic error handling, user-facing messages, post-mortem analysis
**Violation**: `Err(String::from(...))` or `anyhow!(...)` in production code
**Waiver**: None (hard blocker)

### CX-104: Logging & Observability
**Rule**: All mutations (create, update, delete, state changes) logged with trace_id. No plaintext passwords/secrets.
**Scope**: Production code
**Rationale**: Auditability, compliance, debugging, incident response
**Violation**: Missing trace_id, passwords/API keys logged, debug!() or println!() in production
**Waiver**: Acceptable for read-only operations; mutation logging non-negotiable

### CX-105: Test Coverage
**Rule**: New code must maintain >=80% coverage. Removal-style tests required (code removed = test fails).
**Scope**: Production code
**Rationale**: Regression detection, confidence in refactors
**Violation**: Coverage <80%, tests don't check behavior (just mock)
**Waiver**: Below 80% requires explicit team approval + documented reason

### CX-106: TODO Tracking
**Rule**: All TODOs must have tracking ID format: TODO(PROJECT-####) with GitHub issue reference.
**Scope**: Production code
**Rationale**: Prevent accumulation of dead TODOs; force closure
**Violation**: TODO() without ID, TODO(TBD), TODO(fixme)
**Waiver**: None (catch at pre-commit)

### CX-107: No Speculative Code
**Rule**: Code must implement exactly DONE_MEANS, nothing more. No "future-proofing" or "optional" features.
**Scope**: All code
**Rationale**: Scope creep prevention, evidence clarity, maintainability
**Violation**: Code that doesn't map to DONE_MEANS, unused abstractions, "phase 2" code in phase 1
**Waiver**: Refactoring code is exempt if no behavior change

### CX-108: Placeholder Removal
**Rule**: Zero placeholder values in production code. {field}, FIXME, Mock, Stub, TBD, placeholder must not appear.
**Scope**: All code
**Rationale**: Prevents incomplete code reaching production
**Violation**: Grep finds placeholder patterns
**Waiver**: None (catch at pre-commit)

### CX-109: Schema Immutability (Phase 1)
**Rule**: Database schema is frozen during Phase 1. Storage DAL trait contract (CX-102) cannot change.
**Scope**: src/backend/storage/trait.rs
**Rationale**: Storage is foundational; changes cascade to all code
**Violation**: Adding/removing trait methods, changing signatures
**Waiver**: Only if ALL Phase 1 downstream code updated in same packet (use WP-id-v2)

### CX-110: Git Hygiene
**Rule**: Commits must be atomic, reference WP_ID, pass pre-commit hooks.
**Scope**: All commits to main
**Rationale**: Traceability, revertability, compliance
**Violation**: Merge commits without WP reference, commits that fail tests, uncommitted changes blocking merge
**Waiver**: None (enforced at git hook + CI)
```

---

## Protocol File Templates

### ORCHESTRATOR_PROTOCOL.md Template

```markdown
# ORCHESTRATOR_PROTOCOL.md

## Pre-Orchestration Checklist (BLOCKING GATES)

Before accepting ANY user prompt, Orchestrator MUST verify:

### Gate 1: Spec Currency
- [ ] `docs/SPEC_CURRENT.md` exists and is readable
- [ ] Points to valid spec file (e.g., `Master_Spec_v1.0.md`)
- [ ] Run `just validator-spec-regression` → PASS
- [ ] If FAIL: escalate to user; spec must be fixed before proceeding

### Gate 2: Task Board Fresh
- [ ] `docs/TASK_BOARD.md` exists
- [ ] All In-Progress items have SLA tracked (started date recorded)
- [ ] No stale items (>30 days In-Progress) without escalation
- [ ] Phase N Closure Gates listed explicitly

### Gate 3: Governance Current
- [ ] `{Project} Codex v1.0.md` exists
- [ ] All protocol files current (ORCHESTRATOR, CODER, VALIDATOR)
- [ ] Pre-commit hooks wired and working

### Gate 4: Supply Chain
- [ ] Run `cargo deny` → no blockers
- [ ] Run `npm audit` (if applicable) → no critical
- [ ] Dependency check: no major version changes without review

### Gate 5: Signature Audit
- [ ] `docs/SIGNATURE_AUDIT.md` exists
- [ ] All previous signatures logged (audit trail complete)

---

## Signature Pause Protocol

When user prompt arrives, Orchestrator MUST:

1. **Intake**: Extract explicit requirements + implied constraints
2. **Coverage Check**: Does prompt "clearly cover" scope, risks, success criteria, dependencies, rollback?
   - If NO: request clarification; escalate; do NOT proceed
3. **Spec Anchoring**: Map requirements to SPEC_ANCHOR
   - If gap: enrich spec (new version) + obtain signature before proceeding
4. **Signature Pause**: PAUSE and collaborate with user
   - Propose DONE_MEANS (5-10 measurable checkpoints)
   - Propose TEST_PLAN (executable commands)
   - Propose IN/OUT scope (exact files)
   - Propose RISK_TIER (LOW/MEDIUM/HIGH)
   - Propose validator audit scope
5. **User Validation**: User MUST vet and approve
   - Interpretation accuracy? Feasibility? Scope realistic? Risk acceptable?
6. **Signature**: User provides signature `{username}{DDMMYYYYHHMM}`
7. **Packet Creation**: Create packet, run `just pre-work`, update Task Board

---

## Packet Creation Checklist

When creating packet, Orchestrator MUST fill:

- [ ] TASK_ID: `WP-{phase}-{name}`
- [ ] STATUS: `Ready-for-Dev`
- [ ] SPEC_ANCHOR: cite exact anchor from spec (e.g., A2.3.12.3)
- [ ] What: 1-2 sentence description
- [ ] Why: rationale + business value
- [ ] IN_SCOPE_PATHS: 5-20 exact files (not globs)
- [ ] OUT_OF_SCOPE: explicit deferrals
- [ ] DONE_MEANS: 5-10 measurable checkpoints, 1:1 with SPEC_ANCHOR
- [ ] TEST_PLAN: copy-paste bash commands
- [ ] BOOTSTRAP: FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP
- [ ] ROLLBACK_HINT: git revert or manual steps
- [ ] RISK_TIER: LOW/MEDIUM/HIGH
- [ ] Signature & Enrichment Log: frozen intent

---

## Handoff to Coder

Issue packet + provide:
1. Packet path: `docs/task_packets/WP-{id}.md`
2. WP_ID: `WP-{id}`
3. RISK_TIER: `LOW | MEDIUM | HIGH`
4. Authority docs: `SPEC_CURRENT.md`, `CODER_PROTOCOL.md`
5. Command: `just pre-work WP-{id}` (must PASS)
6. Confirmation: "Packet is ready for coding. No changes permitted after signature."
```

---

### CODER_PROTOCOL.md Template

```markdown
# CODER_PROTOCOL.md

## Pre-Coding Checklist (BLOCKING GATES)

### Gate 0: Packet Exists & Complete
- [ ] Task packet file exists: `docs/task_packets/WP-{id}.md`
- [ ] All 10 required fields present
- [ ] No placeholder text (`{field_name}`, `TODO(TBD)`)
- [ ] Run `just pre-work WP-{id}` → PASS (exit 0)
- [ ] If FAIL: return packet to Orchestrator

### Gate 1: Scope Adequacy
- [ ] Can I identify all affected files? (yes/no)
- [ ] Are scope boundaries clear? (yes/no)
- [ ] Are there unexpected dependencies? (list them)
- [ ] Is scope realistic for RISK_TIER? (yes/no)
- [ ] If NO to any: escalate to Orchestrator before starting

### Gate 2: Understand Spec
- [ ] Read `docs/SPEC_CURRENT.md` section on SPEC_ANCHOR
- [ ] Understand DONE_MEANS: can I explain each one?
- [ ] Understand TEST_PLAN: will each command actually prove DONE_MEANS?

---

## Implementation Workflow

### Step 1: Output BOOTSTRAP (BEFORE FIRST CODE CHANGE)
Create BOOTSTRAP block in packet:
```
BOOTSTRAP
WP_ID: WP-{id}
RISK_TIER: {LOW|MEDIUM|HIGH}
TASK_TYPE: {FEATURE|DEBUG|REFACTOR|HYGIENE}
FILES_TO_OPEN: [read packet FILES_TO_OPEN]
SEARCH_TERMS: [read packet SEARCH_TERMS]
RUN_COMMANDS: [read packet RUN_COMMANDS]
RISK_MAP: [read packet RISK_MAP]
```
Mark WP STATUS → `In-Progress` in Task Board

### Step 2: Implement Strictly Within Scope
- Only modify files in IN_SCOPE_PATHS
- Do NOT touch OUT_OF_SCOPE files
- Do NOT implement anything not in DONE_MEANS
- Enforce hard invariants (CX-101, CX-102, CX-103, etc.)
- No TODO() without tracking ID: `TODO(PROJECT-1234)`

### Step 3: Run TEST_PLAN
Execute each command from TEST_PLAN section:
```
Command: cargo test -p {module} {tests}
Result: PASSED (N/N cases)
  ✓ case_1
  ✓ case_2
  ...

Command: {grep/curl/etc}
Result: [output]
```
Record in packet VALIDATION block.

### Step 4: For MEDIUM/HIGH Risk: ai-review
Run `just ai-review` (e.g., Gemini CLI) to audit code changes.
Record pass/fail in VALIDATION block.

### Step 5: Post-Work Gate
Run `just post-work WP-{id}` (verifies VALIDATION results recorded)
- Exit 0: work validated, safe to commit
- Exit 1: incomplete, return to implementation

### Step 6: Update Task Board
- Mark WP STATUS → `Ready-for-Validation`
- Update Task Board entry in lockstep

### Step 7: Prepare Commit
Commit message format:
```
feat: WP-{id} - {short description}

{Detailed explanation of what changed and why}

Evidence:
- {DONE_MEANS 1}: src/file.rs:45-60
- {DONE_MEANS 2}: test result PASS
- {DONE_MEANS 3}: grep "pattern" returns N matches

WP_ID: WP-{id}
Refs: #{issue_number}
```
```

---

### VALIDATOR_PROTOCOL.md Template

```markdown
# VALIDATOR_PROTOCOL.md

## Pre-Flight Checks (BLOCKING GATES)

Before auditing implementation:

- [ ] Task packet exists and complete (all 10 fields)
- [ ] Spec version in packet matches SPEC_CURRENT.md
- [ ] USER_SIGNATURE present and valid (not reused)
- [ ] STATUS in packet is consistent with Task Board
- [ ] BOOTSTRAP block present (shows work was started)
- [ ] TEST_PLAN section exists with commands
- [ ] VALIDATION section has outcomes recorded

**If ANY fail**: return packet to Orchestrator for fixes.

---

## Core Validation Steps

### Step 1: Spec Extraction
Extract every MUST/SHOULD from:
- DONE_MEANS in task packet
- SPEC_ANCHOR sections in spec
Create evidence mapping table:
| DONE_MEANS | SPEC_ANCHOR | Code Evidence | Test Proof |
| ... | ... | ... | ... |

### Step 2: Evidence Verification
For each requirement:
1. Locate file:line code evidence
2. Read the code; verify it implements requirement
3. Run test command; verify it PASSES
4. Verify test is removable (fails if code deleted)
5. **Missing evidence = FAIL**

### Step 3: Hygiene Gates
Run validators:
```
just validator-scan              → 0 forbidden patterns
just validator-git-hygiene       → 0 artifacts committed
just validator-error-codes       → typed errors, codes documented
just validator-{custom}          → domain-specific checks
```
**Any FAIL = overall FAIL**

### Step 4: Test Verification
- Verify TEST_PLAN was executed (results in VALIDATION block)
- Check coverage >= 80% (or documented waiver)
- Verify at least one removal-check test (code removal breaks test)

### Step 5: Security & Architecture Audits
- LLM boundary: all calls via /src/backend/llm/ (CX-101)
- Storage boundary: no direct DB outside /storage/ (CX-102)
- Error handling: typed errors with codes (CX-103)
- Logging: trace IDs in mutations, no secrets (CX-104)
- Traceability: job_id, request_id correlation

---

## Verdict (Binary: PASS or FAIL)

### PASS Criteria (ALL must be true):
- Evidence mapping complete (every DONE_MEANS has file:line + test)
- All tests PASS
- Hygiene clean (no forbidden patterns)
- Coverage >= 80%
- Codex compliance verified
- Custom audits satisfied

**Action**: Append validation report to packet; update STATUS → Done; update Task Board to Done (VALIDATED)

### FAIL Criteria (ANY is true):
- Missing evidence for requirement
- Test fails or missing
- Forbidden pattern found
- Codex violation detected
- Coverage < 80% without waiver

**Action**: Document gaps; list violations; return packet to Orchestrator/Coder for rework

---

## Waiver Policy

Waivers are ALLOWED (with approval) for:
- Coverage <80% (if business-justified + documented)
- Test gap in Phase 1 (Phase 2 backlog)
- Deferred design debt (explicit TODO(PROJECT-####) + issue link)

Waivers are NOT ALLOWED for:
- Spec regression (requirement not met)
- Evidence gaps (must have file:line proof)
- Codex violations (hard invariants)
- Security issues (RCE, plaintext secrets)
```

---

## Validator Script Implementations

### Sample: pre-work-check.mjs

```javascript
// scripts/validation/pre-work-check.mjs
// Gate 0: Ensure packet is complete before handoff

import fs from 'fs';
import path from 'path';

const REQUIRED_FIELDS = [
  'TASK_ID',
  'STATUS',
  'SPEC_ANCHOR',
  'What',
  'Why',
  'IN_SCOPE_PATHS',
  'OUT_OF_SCOPE',
  'DONE_MEANS',
  'TEST_PLAN',
  'BOOTSTRAP',
  'ROLLBACK_HINT',
  'VALIDATION',
  'Signature & Enrichment Log'
];

const PLACEHOLDERS = [
  /\{field\w+\}/g,
  /TODO\(TBD\)/g,
  /FIXME/g,
  /placeholder/gi,
  /\{.*?\}/g  // Any {something}
];

export async function validate(wpId, projectRoot) {
  const packetPath = path.join(projectRoot, 'docs', 'task_packets', `${wpId}.md`);

  if (!fs.existsSync(packetPath)) {
    console.error(`❌ Packet not found: ${packetPath}`);
    process.exit(1);
  }

  const content = fs.readFileSync(packetPath, 'utf-8');

  // Check all required fields present
  for (const field of REQUIRED_FIELDS) {
    if (!content.includes(`## ${field}`) && !content.includes(`- ${field}`)) {
      console.error(`❌ Missing required field: ${field}`);
      process.exit(1);
    }
  }

  // Check for placeholders
  for (const placeholder of PLACEHOLDERS) {
    const matches = content.match(placeholder);
    if (matches) {
      console.error(`❌ Found placeholder text: ${matches[0]}`);
      process.exit(1);
    }
  }

  console.log(`✅ Pre-work check passed: ${wpId}`);
  process.exit(0);
}

const wpId = process.argv[2];
const projectRoot = process.cwd();
validate(wpId, projectRoot);
```

### Sample: validator-scan.mjs

```javascript
// scripts/validation/validator-scan.mjs
// Scan for forbidden patterns in production code

import { execSync } from 'child_process';

const SCAN_PATHS = [
  'src/',
  'app/src/'
];

const FORBIDDEN_PATTERNS = [
  { pattern: 'unwrap()', reason: 'Panic in production; must handle error' },
  { pattern: 'expect(', reason: 'Panic in production; must handle error' },
  { pattern: 'todo!()', reason: 'Incomplete code; must implement or defer' },
  { pattern: 'unimplemented!()', reason: 'Incomplete code' },
  { pattern: 'dbg!(', reason: 'Debug macro; must remove' },
  { pattern: 'println!(', reason: 'Use logging framework; no stdout' },
  { pattern: 'eprintln!(', reason: 'Use logging framework; no stderr' }
];

const PLACEHOLDER_PATTERNS = [
  'Mock', 'Stub', 'placeholder', 'hollow', /\{.*?\}/
];

let violations = 0;

for (const scanPath of SCAN_PATHS) {
  // Exclude tests
  const excludeTests = `--exclude-dir=tests --exclude-dir=test`;

  for (const { pattern, reason } of FORBIDDEN_PATTERNS) {
    try {
      const cmd = `rg "${pattern}" ${scanPath} ${excludeTests}`;
      const result = execSync(cmd, { encoding: 'utf-8' });

      if (result.trim()) {
        console.error(`❌ FORBIDDEN: ${pattern}`);
        console.error(`   Reason: ${reason}`);
        console.error(result);
        violations++;
      }
    } catch (e) {
      // No matches (expected)
    }
  }

  for (const placeholder of PLACEHOLDER_PATTERNS) {
    try {
      const cmd = `rg "${placeholder}" ${scanPath} ${excludeTests}`;
      const result = execSync(cmd, { encoding: 'utf-8' });

      if (result.trim()) {
        console.error(`⚠️  PLACEHOLDER: ${placeholder}`);
        console.error(result);
        violations++;
      }
    } catch (e) {
      // No matches
    }
  }
}

if (violations > 0) {
  console.error(`\n❌ Found ${violations} violations`);
  process.exit(1);
} else {
  console.log('✅ validator-scan passed');
  process.exit(0);
}
```

### Template: Custom Validator Pattern

```javascript
// scripts/validation/validator-{your-concern}.mjs
// Template for project-specific validators

export async function validate(wpId, projectRoot) {
  // 1. Read task packet
  const packetPath = `${projectRoot}/docs/task_packets/${wpId}.md`;
  const packetContent = readFile(packetPath);

  // 2. Parse IN_SCOPE_PATHS from packet
  const inScopePaths = extractField(packetContent, 'IN_SCOPE_PATHS');

  // 3. Apply domain-specific checks
  // Example: For storage audits
  for (const filePath of inScopePaths) {
    const content = readFile(`${projectRoot}/${filePath}`);

    // Check 1: No direct DB access
    if (content.includes('sqlx::query') && !filePath.includes('storage/')) {
      console.error(`❌ CX-DBP-VAL-010: Direct DB access in ${filePath}`);
      process.exit(1);
    }

    // Check 2: SQL portability
    if (content.includes('strftime(') || content.includes('?1')) {
      console.error(`❌ CX-DBP-VAL-011: SQLite-only SQL in ${filePath}`);
      process.exit(1);
    }
  }

  // 4. Return verdict
  console.log(`✅ validator-{concern} passed`);
  process.exit(0);
}

const wpId = process.argv[2];
validate(wpId, process.cwd());
```

---

## Troubleshooting & Common Failures

### "Packet fails pre-work check"

**Symptoms**: `just pre-work WP-1` returns exit 1

**Causes**:
1. Missing required field (check all 10 fields present)
2. Placeholder text in packet (`{field}`, `TODO(TBD)`)
3. Field value is empty or just comment marker

**Fix**:
1. Read pre-work-check output: identifies which field is missing
2. Open packet; add field with concrete value
3. Remove all `{placeholders}` and replace with real values
4. Re-run `just pre-work WP-1` → exit 0

### "Validator says evidence missing"

**Symptoms**: Validator report says "Missing evidence for DONE_MEANS item 3"

**Causes**:
1. Code not implemented (DONE_MEANS not actually done)
2. Test command not executed or failed
3. file:line reference doesn't actually contain the code

**Fix**:
1. Verify code exists at file:line location (open file, check line numbers)
2. Verify test command runs without error: `cargo test ...`
3. If code missing: implement it
4. If test fails: debug test; fix code
5. Record new results in VALIDATION block
6. Re-run validator

### "Codex check fails at commit"

**Symptoms**: `git commit` blocked with "CX-101 violation: LLM call outside /src/backend/llm/"

**Causes**:
1. Code calls LLM API directly (not via module)
2. Code uses `println!` or `dbg!` (logging instead of logging framework)
3. Code has `unwrap()` or `expect(` in production

**Fix**:
1. Read codex rule (e.g., CX-101) for what's forbidden and why
2. Refactor code to comply:
   - LLM calls: move to `/src/backend/llm/client.rs`
   - Logging: use logging framework instead of println
   - Error handling: use typed errors instead of unwrap
3. Stage changes: `git add .`
4. Try commit again: `git commit ...`

### "Scope creep: features not in DONE_MEANS"

**Symptoms**: Coder implements extra features not in task packet

**Causes**:
1. Coder assumed "while we're here, let's also..."
2. Spec ambiguity; coder inferred additional requirements
3. Scope adequacy check not done properly

**Fix**:
1. **Before implementation**: Coder must do Scope Adequacy Check (identify all affected files, validate boundaries)
2. **During implementation**: Stick to DONE_MEANS only; nothing more
3. **If new feature discovered**: Create WP-{id}-v2 packet with fresh signature
4. **Validator catches**: Will FAIL if code doesn't match DONE_MEANS (removable test fails)

### "Signature audit shows reuse"

**Symptoms**: `grep -r "alice_25121510000" . | wc -l` returns 2+ (signature used twice)

**Causes**:
1. Signature was issued for Phase 0 enrichment
2. Same signature reused for Phase 1 packet (violation)
3. Typo in signature log (same signature recorded twice)

**Fix**:
1. Each signature is ONE-TIME USE ONLY
2. If reuse detected: STOP work; escalate to user
3. Request fresh signature (different timestamp): `alice_25121510100`
4. Log new signature in SIGNATURE_AUDIT.md
5. Re-run spec regression check

### "Task Board out of sync with packets"

**Symptoms**: Packet STATUS = "Done" but Task Board shows "Ready-for-Dev"

**Causes**:
1. Coder updated packet but forgot Task Board
2. Validator updated Task Board but forgot packet
3. Manual edit error

**Fix**:
1. Atomicity rule: **ALWAYS update both packet and Task Board together**
2. If out of sync: align them now
   - Read current packet STATUS
   - Update Task Board entry to match
   - Verify with: `grep "WP-1-Feature" docs/TASK_BOARD.md` matches packet STATUS

### "Phase gate blocked; can't close Phase 1"

**Symptoms**: `just validator-phase-gate Phase-1` returns exit 1

**Causes**:
1. At least one "Phase 1 Closure Gates (BLOCKING)" WP is not Done+VALIDATED
2. Spec regression check failed (anchor missing)
3. Dependency unresolved (downstream WP blocker still In-Progress)

**Fix**:
1. Run command with verbose output: see which WP is blocking
2. Go to blocking WP: move to Done and validate
3. Check spec regression: `just validator-spec-regression` should PASS
4. Check dependencies: all upstream WPs must be VALIDATED
5. Once all clear: `just validator-phase-gate Phase-1` returns exit 0
6. Can now close phase: `git tag phase-1-complete && git push origin phase-1-complete`

---

## Integration with Existing Projects

If you have an **existing codebase** (not starting from scratch), use this retrofit approach:

### Week 1: Minimal Setup (Don't Block Current Work)
1. Create `Master_Spec_v0.1.md` describing current state (big picture, 10-15 sections)
2. Create `docs/SPEC_CURRENT.md` pointer
3. Create `docs/SIGNATURE_AUDIT.md` (empty)
4. Create `docs/TASK_BOARD.md` (list current in-flight work)
5. Create `scripts/create-task-packet.mjs` (simple generator)

**Do NOT force everyone to use this workflow yet.** Just set up infrastructure.

### Week 2: Optional Adoption (Volunteer Features)
1. Volunteer a low-risk feature (e.g., documentation, small bug fix)
2. Create task packet for it (using template)
3. Run full workflow: pre-work → code → validate → merge
4. Measure: did workflow catch issues? Did it help?

### Week 3: Phased Rollout (Team Agreement)
- If Week 2 successful: make workflow mandatory for NEW features only
- Existing work continues as-is
- Gradual adoption: new feature per week uses workflow

### Month 2: Enforcement
- All NEW feature work uses workflow
- All bug fixes use workflow
- Refactors optional (can use or skip)

### Month 3: Full Adoption
- All work uses workflow
- Agents can operate semi-autonomously
- Governance rules enforced at pre-commit

---

## Testing the Workflow Infrastructure

Before using on real work, validate each piece works:

### Test 1: Packet Creation
```bash
just create-task-packet WP-0-Test
# Verify: docs/task_packets/WP-0-Test.md created with template
```

### Test 2: Pre-Work Gate
```bash
just pre-work WP-0-Test
# Should FAIL (has placeholders)
# Edit packet: remove {field} values
just pre-work WP-0-Test
# Should PASS (all fields concrete, no placeholders)
```

### Test 3: Spec Regression
```bash
just validator-spec-regression
# Should PASS (spec file exists, required anchors present)
```

### Test 4: Validator Scan
```bash
just validator-scan
# Should PASS (no forbidden patterns in src/)
# To test: add unwrap() to src/main.rs, re-run → FAIL
```

### Test 5: Pre-Commit Hook
```bash
# Add println!() to src/lib.rs
git add src/lib.rs
git commit -m "test"
# Should FAIL (pre-commit hook rejects println)
# Remove println(), re-commit → PASS
```

### Test 6: Full Workflow (Dry Run)
1. Create test packet for small feature
2. Implement it
3. Run `just post-work WP-{id}`
4. Validator audits
5. Merge

If all tests pass, infrastructure is ready.

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
