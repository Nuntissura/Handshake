# Handshake Spec-Driven Multi-Agent Workflow (Local Execution)

## Scope and Sources
- Based on current governance: `docs/SPEC_CURRENT.md` (Master Spec v02.84), `Handshake Codex v1.4.md`, `docs/ORCHESTRATOR_PROTOCOL.md`, `docs/CODER_PROTOCOL.md`, `docs/VALIDATOR_PROTOCOL.md`, `docs/TASK_BOARD.md`, `scripts/create-task-packet.mjs`.
- Audience: people running Handshake locally who need a precise description of how the spec, orchestrator, coder, validator, and future tool-calling agent cooperate.

## Spec as the Authority
- `docs/SPEC_CURRENT.md` pins the active Master Spec version; Main Body sections are the only acceptable authority for work packets. Roadmap items require promotion plus signature before use.
- The signature checkpoint (user-provided `{username}{DDMMYYYYHHMM}`) is a **forced collaborative pause** before spec enrichment or packet creation. Signatures are single-use, immutable locks, and must be logged in `docs/SIGNATURE_AUDIT.md`. See "Signature Pause: Alignment & Enrichment Protocol" for collaboration workflow.
- Spec change workflow: clone the current spec to a new version, enrich with clarified requirements, update protocol files, bump references, and rerun `just validator-spec-regression`.
- Every work packet must cite a precise `SPEC_ANCHOR` (e.g., `A2.3.12.3`) that exists in the current spec; ambiguous anchors must be escalated, not guessed.

## Roles and Responsibilities

### Orchestrator (Lead Architect / Engineering Manager)
- Owns translation of user intent/spec requirements into immutable task packets; does not write code.
- Pre-orchestration gates: verify spec currency and regression, task board freshness, supply chain (`cargo deny`, `npm audit`), governance file currency, and dependency chains.
- **Signature pause protocol (before packet creation):** Orchestrator proposes enrichment and rubric adjustments; user and orchestrator collaborate to agree on DONE_MEANS, TEST_PLAN, scope, risk tier, and evidence model. See "Signature Pause: Alignment & Enrichment Protocol" for full workflow. Signature validates the collaboration and locks intent.
- Packet creation (via `just create-task-packet WP-{id}`): fill all 10 required fields with no placeholders (scope with IN/OUT paths, RISK_TIER, TEST_PLAN commands, DONE_MEANS mapped 1:1 to SPEC_ANCHOR, ROLLBACK_HINT, BOOTSTRAP with FILES/TERMS/RUN/RISK_MAP).
- Scope precision: IN_SCOPE_PATHS are exact files; OUT_OF_SCOPE is explicit deferral; one requirement per packet.
- Verification before delegation: run `just pre-work WP-{id}`; block on failure; update `docs/TASK_BOARD.md` and packet STATUS in lockstep; lock packets with USER_SIGNATURE (immutable thereafter; create variant packets for changes).
- Handoff: provide packet path, WP_ID, risk tier, authority docs, and readiness confirmation. Maintain status SLAs and dependency rules (no downstream work until blockers are VALIDATED).

### Coder / Debugger
- Refuses to start without a complete task packet; performs scope adequacy check before reading details.
- Outputs BOOTSTRAP (FILES_TO_OPEN 5-15, SEARCH_TERMS 10-20, RUN_COMMANDS 3-6, RISK_MAP 3-8) before first change; moves WP to In Progress on the Task Board.
- Implements only within IN_SCOPE_PATHS, honoring DONE_MEANS and OUT_OF_SCOPE. Enforces hard invariants: LLM calls only via `/src/backend/llm/` [CX-101], no direct HTTP in jobs/features [CX-102], no `println!/eprintln!` [CX-104], TODOs must be `TODO(HSK-####)`, zero speculative requirements.
- Validation order: TEST_PLAN commands (e.g., cargo test, pnpm test/lint, clippy), then `just ai-review` for MEDIUM/HIGH risk, then `just post-work WP-{id}`. Each DONE_MEANS item must have file:line evidence and test proof.
- Updates packet with validation block/status, maintains Task Board sync, prepares commit message referencing WP_ID; no commits without validation.

### Validator (Senior Engineer / Lead Auditor)
- Blocks merges unless evidence proves alignment with spec, codex, and packet; preserves User Context sections in packets.
- Pre-flight: confirm packet completeness, spec version match, USER_SIGNATURE unchanged, STATUS present, BOOTSTRAP present, TEST_PLAN concrete.
- Spec extraction: enumerate every MUST/SHOULD from DONE_MEANS + spec slices; evidence map each to path:line; missing evidence = FAIL.
- Skeleton/hygiene gates: reject hollow code, JSON blobs where types required, unwrap/expect/panic/dbg/println/todo in production without waiver; enforce Zero Placeholder Policy.
- Targeted audits: storage DAL (trait boundary, SQL portability, migration hygiene, dual-backend readiness), LLM boundary, determinism/traceability (trace_id/job_id), security/RCE guardrails, git/build hygiene. Uses `just validator-scan`, `just validator-dal-audit`, `just validator-error-codes`, `just validator-hygiene-full`, etc.
- Test verification: ensure TEST_PLAN executed; require at least one removal-check style test or documented waiver for new logic; <80% coverage flagged as risk.
- Verdict is binary PASS/FAIL. On PASS, append validation report to packet, update STATUS, and move Task Board entry to Done; on FAIL, document gaps and return to orchestrator/coder.

## Prompt-to-Spec and Packet Conversion
1) **Intake:** treat the user prompt as a proto-spec. Extract explicit requirements and implied constraints.
2) **Coverage check:** run the "clearly covers" 5-point test against the Main Body. If any point fails, request clarification and enrich the spec with a new version + signature before proceeding.
3) **Spec anchoring:** map each requirement to a SPEC_ANCHOR. If no anchor exists, enrich spec (new version, update protocols, audit log).
4) **Signature pause (alignment checkpoint):** Orchestrator proposes DONE_MEANS (5–10 measurable checkpoints), TEST_PLAN commands, scope boundaries (IN/OUT paths), RISK_TIER, and evidence audit model. User and Orchestrator collaborate to:
   - Confirm DONE_MEANS are unambiguous and testable (not "implement auth", but "users can POST /auth/login and receive jwt_token")
   - Validate TEST_PLAN commands are executable and prove each DONE_MEANS
   - Agree on audit scope and validator rubric (e.g., DAL audit? LLM boundary? security checks?)
   - Adjust RISK_TIER and rollback plan if needed
   - Lock interpretation with signature; record collaboration notes in packet. See "Signature Pause: Alignment & Enrichment Protocol" for details.
5) **Packetization:** for each requirement, generate packet with locked DONE_MEANS, TEST_PLAN, scope, and bootstrap plan. Use the generator script and replace every placeholder; run `just pre-work WP-{id}`.
6) **Task board instantiation:** add the packet to `docs/TASK_BOARD.md` under Ready for Dev (or Blocked if dependencies are unresolved) with dependency notes.

## Signature Pause: Alignment & Enrichment Protocol

The signature checkpoint is a **forced collaborative pause** where User and Orchestrator (the LLM agent) align on requirements before any packet is created or spec is enriched. This ensures misalignment does not cascade downstream to Coder, Validator, and Tool Agent.

### What the Orchestrator Proposes (Pre-Signature)

Before requesting a signature, the Orchestrator must propose:

1. **Spec Enrichment Slate**
   - New sections needed in SPEC_CURRENT.md?
   - Existing SPEC_ANCHOR sections that apply, or gaps requiring enrichment?
   - Hidden interdependencies or constraint violations (e.g., DAL boundary impacts, security surface changes)?

2. **DONE_MEANS Refinement (5–10 Measurable Checkpoints)**
   - Not aspirational ("implement authentication"), but concrete and testable ("users can POST /auth/login with email+password, receive jwt_token in response header with 24-hour TTL, and subsequent requests with Authorization: Bearer token succeed").
   - Each item must be verifiable with a yes/no test or file:line evidence.
   - Map each DONE_MEANS to a SPEC_ANCHOR.

3. **TEST_PLAN Commands (Executable Proof)**
   - Concrete cargo test, pnpm test, curl, or inspection commands that prove each DONE_MEANS.
   - Commands must be runnable in the current codebase with no ambiguity.
   - Coverage threshold and removal-check style test requirements.

4. **Scope Precision (IN/OUT_SCOPE_PATHS)**
   - IN_SCOPE_PATHS: exact files that will be modified (not directories, not glob patterns).
   - OUT_OF_SCOPE: explicit deferrals and constraints (e.g., "storage DAL structure is immutable", "do not touch validator protocol").
   - One logical requirement per packet; interdependencies flagged for sequencing.

5. **RISK_TIER Assessment (LOW / MEDIUM / HIGH) with Justification**
   - Why this tier? (e.g., "MEDIUM: modifies LLM boundary + storage schema, triggers DAL audit").
   - If MEDIUM/HIGH: validator must-checks and ai-review triggers.
   - Rollback plan: if implementation fails or validator rejects, what is recovery?

6. **Validator Rubric & Audit Scope**
   - Which validator scripts will run? (hygiene, DAL, LLM, security, etc.)
   - Evidence checklist for validator: what files, what test commands, what trace IDs?
   - Any waivers or exceptions pre-agreed?

7. **Packet Variant Triggers**
   - If user asks "can you also do X?" during implementation, will it spawn a `-v2` packet or amend DONE_MEANS?
   - Pre-agree on scope creep handling to prevent mid-flight surprises.

### What the User Should Clarify & Validate

During the pause, the user must vet:

1. **Interpretation Accuracy**
   - Read the DONE_MEANS back to yourself. Does it match what you asked for?
   - Are the checkpoints over-engineered, under-specified, or just right?
   - Did Orchestrator miss implicit requirements (performance SLAs, backward compatibility, deprecation path)?

2. **TEST_PLAN Feasibility**
   - Can those commands actually run in the current repo state?
   - Will a cargo test command in FILES_TO_OPEN succeed without manual setup?
   - Is coverage threshold (e.g., 80%) realistic given the scope?

3. **Scope Realism**
   - Are the IN_SCOPE_PATHS achievable, or has Orchestrator underestimated dependencies?
   - Are the OUT_OF_SCOPE deferrals acceptable, or do they undermine the goal?
   - Can one requirement truly fit in one packet, or should this be two packets?

4. **Risk Acknowledgment**
   - Do you accept the RISK_TIER and validator audit scope?
   - Is the rollback plan adequate if something goes wrong?
   - Are there hidden systems (auth, storage, LLM integration) that might break?

5. **Cost vs. Benefit**
   - Is the effort aligned with the business value?
   - Are there pre-requisite tasks or blockers that should be resolved first?

### Collaboration Outcome: Signature Block

Once aligned, the user issues a signature `{username}{DDMMYYYYHHMM}`. The Orchestrator records this block in the packet:

```markdown
## Signature & Enrichment Log

**Signature:** alice_25121512345  (immutable)
**Timestamp:** 2025-12-15 12:34:56 UTC

### User-Orchestrator Collaboration Notes:
- **Clarified:** User explained that auth must support both email and OAuth. Orchestrator updated DONE_MEANS to include OAuth flow.
- **Spec Enrichment:** Added new section A4.2.7 (OAuth Integration) to SPEC_CURRENT.md v02.85; updated ORCHESTRATOR_PROTOCOL.md.
- **Rubric Adjustments:** DONE_MEANS expanded from 4 to 7 items; TEST_PLAN added OAuth callback test; DAL audit added (no storage changes, but JWT schema validated).
- **Risks Acknowledged:** User approved RISK_TIER = MEDIUM; rollback is to revert feature flag and purge test OAuth clients.

### Locked Intent (Immutable):
- **DONE_MEANS:** [frozen, word-for-word]
- **TEST_PLAN:** [frozen]
- **IN_SCOPE_PATHS:** [frozen]
- **OUT_OF_SCOPE:** [frozen]
- **Validator Audit Scope:** [frozen]
```

Once signed, the packet is locked. Any change requires user+orchestrator to collaborate again and create a `-v2` variant with a new signature.

### Operational Application

When Handshake agents run operationally, every spec change triggers the same pause:
- Agent (playing Orchestrator role) proposes enrichment.
- User reviews and agrees or refines.
- User provides signature to lock intent.
- Downstream agents execute with unambiguous, signed requirements.

This ensures governance and provenance at development time and runtime.

## Task Packet Lifecycle and State Flow
- States: Backlog -> Ready for Dev -> In Progress -> Ready for Validation -> Done (VALIDATED). Task Board and packet STATUS must always match.
- Dependency enforcement: downstream packets remain BLOCKED until upstream STATUS is VALIDATED; SLAs (Blocked >5 days, Ready >10 days, In Progress >30 days) trigger escalations.
- Locking: USER_SIGNATURE freezes packet content; variants (e.g., `-v2`) are created for scope changes.
- Artifacts: packet in `docs/task_packets/`, Task Board entry, validation block inside packet, optional logger entry for milestones/hard bugs.

## Local Execution Pipeline (Three Agents + Future Tool Agent)
1) Orchestrator (local LLM) performs gates, creates/locks packet, runs `just pre-work WP-{id}`, updates Task Board, issues handoff.
2) Coder (local code-capable model) runs BOOTSTRAP, implements within scope, executes TEST_PLAN and hygiene commands, appends validation notes, and proposes commit message.
3) Validator (audit model or scripted checks) replays TEST_PLAN selectively, runs validator scripts, audits evidence, and records PASS/FAIL in the packet; signals orchestrator to close or reopen.
4) Orchestrator finalizes: posts mechanical output to user (what changed, validation summary), files validation report into the packet, moves Task Board item to Done.

## Introducing a Tool-Calling Agent (Fourth Role)
- Purpose: execute mechanical commands (just tasks, rg/grep scans, formatting, small file edits, MCP calls) so Coder/Validator prompts stay shorter and safer.
- Workflow impact:
  - Orchestrator adds a "Tooling" section to packets with approved commands, cwd constraints, output size limits, and logs destination.
  - Coder delegates TEST_PLAN and search commands to the tool agent; the agent returns logs for inclusion in the packet's validation block.
  - Validator can request reruns of validator scripts via the tool agent to keep validation reproducible.
  - Governance: tool agent must honor IN_SCOPE_PATHS, OUT_OF_SCOPE, and risk tier; orchestrator should require command allowlists and cap time/output for security.
- Benefits: reduces context churn for large prompts, standardizes command execution, and provides deterministic logs for Validator; risk is minimized by explicit allowlists and packet-specified bounds.

## Context Window Strategy and Mitigating Context Rot
- Spec anchoring plus immutable packets mitigate drift: all instructions live in durable files (spec version, packet, Task Board) rather than chat history.
- Use retrieval-style prompts: supply only relevant spec slices (SPEC_ANCHOR sections), packet fields, and recent validation notes to each agent call.
- Keep prompts short and structural: refer to packet fields by name and cite SPEC_ANCHOR IDs instead of pasting entire specs.
- Periodically refresh summaries: when packets stay open, have orchestrator issue brief state digests (scope, open questions, last validation) to combat context rot.
- Large-model trends: long-context models help but still benefit from hierarchical prompting (anchor IDs + concise evidence). Structured logs (validation blocks, BOOTSTRAP, command outputs) are safer than raw transcripts for reuse.

## Treating Prompts as Specs for Heavy Tasks
- Promote user prompts into packet-grade requirements: rewrite as DONE_MEANS with measurable yes/no checks, explicit file scope, and a TEST_PLAN.
- Require spec anchors or explicit enrichment for any new behavior; record rationale in the packet "Why" and in SIGNATURE_AUDIT when enrichment occurs.
- Survivability under scrutiny comes from determinism: every claim must point to SPEC_ANCHOR and have a verification command; assumptions go into packet Notes until resolved.

## End-to-End Handshake Loop
1) **User prompt arrives.** Orchestrator performs clarity + spec coverage checks; enriches spec if needed (signature, version bump).
2) **Signature pause (user-orchestrator alignment).** Orchestrator proposes DONE_MEANS, TEST_PLAN, scope, RISK_TIER, and validator rubric. User and Orchestrator collaborate to refine. Once aligned, user provides signature to lock intent. Collaboration notes recorded in packet.
3) **Packet creation and pre-flight.** Orchestrator generates packet, runs `just pre-work`, updates Task Board to Ready for Dev, and hands off.
4) **Coder execution phase.** Coder BOOTSTRAPs, implements within scope, runs tests/ai-review/post-work, and records validation with evidence against locked DONE_MEANS.
5) **Validator audit phase.** Validator audits spec alignment (DONE_MEANS vs SPEC_ANCHOR), hygiene, tests (TEST_PLAN), DAL/LLM/security, and records PASS/FAIL in the packet.
6) **Orchestrator finalization.** Publishes mechanical output to user (summary + validation status), files validation report inside packet, updates Task Board to Done, and closes work item.

## Task Packet Template & Required Fields

Every work packet must contain exactly 10 required fields (no placeholders):

1. **TASK_ID** - `WP-{phase}-{name}` (e.g., `WP-1-Storage-Layer`)
2. **STATUS** - One of: `Ready-for-Dev | In-Progress | Done | Backlog`
3. **What** - 1-2 sentence description of the requirement
4. **Why** - Business/technical rationale; references SPEC_ANCHOR
5. **IN_SCOPE_PATHS** - Exact file paths (5-20 specific files, not directory globs)
6. **OUT_OF_SCOPE** - Explicit deferrals with reasons
7. **DONE_MEANS** - 5-10 measurable, yes/no checkpoints mapped 1:1 to SPEC_ANCHOR
8. **TEST_PLAN** - Copy-paste-ready bash commands proving each DONE_MEANS
9. **BOOTSTRAP** - (Coder's work plan)
   - `FILES_TO_OPEN`: 5-15 specific files to read first
   - `SEARCH_TERMS`: 10-20 exact grep strings to find related code
   - `RUN_COMMANDS`: 3-6 test/setup commands to verify state
   - `RISK_MAP`: 3-8 failure modes → affected subsystems
10. **VALIDATION** - (Updated by Coder, audited by Validator)
    - `Command`: Exact test command run
    - `Result`: PASS/FAIL with details
    - `Notes`: Warnings or special conditions

**Template Generation** (via script):
```bash
just create-task-packet WP-{ID}
# → Generates docs/task_packets/WP-{ID}.md with all 10 fields + template text
# → Requires manual replacement of every placeholder before packet is valid
# → just pre-work WP-{ID} will FAIL until all placeholders removed
```

---

## Validation Script Architecture

Create `scripts/validation/` directory with modular validator scripts. Each script checks ONE concern and exits 0 (pass) or 1 (fail). Wire through `justfile` for uniform command interface.

### Pre-Work Gate (Gate 0)

**File**: `scripts/validation/pre-work-check.mjs`

**Checks**:
1. Task packet file exists: `docs/task_packets/WP-{ID}.md`
2. Packet contains all 10 required fields
3. No placeholder text (`{field_name}`, `TODO(TBD)`)
4. TASK_ID, RISK_TIER, SCOPE, TEST_PLAN, DONE_MEANS present

**Command**: `just pre-work WP-{ID}`

**Exit**: 0 (proceed to implementation), 1 (return to Orchestrator)

### Post-Work Gate (Gate 1)

**File**: `scripts/validation/post-work-check.mjs`

**Checks**:
1. Task packet exists
2. VALIDATION section has outcomes recorded (Command, Result, Notes)
3. If TEST_PLAN lists cargo test/pnpm test, validate documented in VALIDATION
4. If RISK_TIER MEDIUM/HIGH: verify ai_review completed and not BLOCKED
5. FILES changed in git (work was actually done)

**Command**: `just post-work WP-{ID}`

**Exit**: 0 (work validated, safe to commit), 1 (incomplete, return to Coder)

### Forbidden Pattern Scanner

**File**: `scripts/validation/validator-scan.mjs`

**Forbidden in production code**:
```
unwrap              (Exception: OK in tests)
expect(             (Exception: OK in tests)
todo!               (Exception: OK in tests)
unimplemented!
dbg!
println!            (Exception: OK in tests)
eprintln!
panic!              (Exception: OK in tests)
```

**Placeholder patterns** (anywhere):
```
Mock, Stub, placeholder, hollow, {field}
```

**Command**: `just validator-scan`

**Exit**: 0 (clean), 1 (patterns found, lists all)

**Note**: Update SCAN_PATHS and FORBIDDEN_PATTERNS arrays for your project.

### Spec Regression Validator

**File**: `scripts/validation/validator-spec-regression.mjs`

**Purpose**: Gate phase progression; ensure spec integrity

**Checks**:
1. `docs/SPEC_CURRENT.md` exists and is readable
2. Points to valid spec file (e.g., `Master_Spec_v02.84.md`) in repo root
3. Spec file contains all REQUIRED_ANCHORS (define for your project):
   ```javascript
   const REQUIRED_ANCHORS = [
     "§2.3.12",    // Storage portability
     "§2.3.11",    // Retention/GC
     "§2.6.7",     // Semantic catalog
     "§2.9.3"      // Mutation traceability
   ];
   ```

**Command**: `just validator-spec-regression`

**Exit**: 0 (spec valid), 1 (file missing or anchor gap)

### Custom Audit Scripts (Architecture-Specific)

**Template Pattern**:
```javascript
// scripts/validation/validator-{concern}.mjs
export async function validate({WP_ID, projectRoot}) {
  // 1. Read files in scope
  // 2. Apply domain-specific checks
  // 3. Collect evidence
  // 4. Return {passed: bool, violations: []}
}
```

**Examples to implement**:
- `validator-dal-audit.mjs` - Storage layer boundary checks
- `validator-error-codes.mjs` - Typed errors, not stringly errors
- `validator-traceability.mjs` - Trace IDs in mutations
- `validator-coverage-gaps.mjs` - Test coverage thresholds
- `validator-git-hygiene.mjs` - .gitignore, repo bloat

---

## Justfile Command Wiring

Create `justfile` with commands for the entire workflow. Pattern:

```makefile
# Scaffold
create-task-packet WP_ID:
  node scripts/create-task-packet.mjs {{WP_ID}}

# Validation gates
pre-work WP_ID:
  node scripts/validation/pre-work-check.mjs {{WP_ID}}

post-work WP_ID:
  node scripts/validation/post-work-check.mjs {{WP_ID}}

# Validator suite
validator-scan:
  node scripts/validation/validator-scan.mjs

validator-spec-regression:
  node scripts/validation/validator-spec-regression.mjs

validator-dal-audit:
  node scripts/validation/validator-dal-audit.mjs

# Unified workflow command
validate-workflow WP_ID:
  just pre-work {{WP_ID}}
  just validator-scan
  just validator-spec-regression
  just ai-review
  just post-work {{WP_ID}}

# Phase progression gate
validator-phase-gate PHASE:
  node scripts/validation/validator-phase-gate.mjs {{PHASE}}
```

**Total**:
- 5-7 core validation commands
- 2 gates (pre-work, post-work)
- 4-6 specialist validators
- 1 unified workflow command
- 1 phase gate

**Wire all into justfile for uniform CLI interface.**

---

## SIGNATURE_AUDIT.md: Immutable Signature Registry

**File**: `docs/SIGNATURE_AUDIT.md`

**Purpose**: Single source of truth for signature consumption; prevents replay and reuse

**Format**:

```markdown
# Signature Audit Log

**Signature Format**: {username}{DDMMYYYYHHMM}

**Example**: `ilja_25122512345` = user "ilja" + 2025-12-25 12:34:56 UTC

**Rules**:
- Each signature is ONE-TIME USE only
- External clock required (user must verify timestamp)
- Consumption is permanent (see log below)
- Reuse check: `grep -r "{signature}" .` should return ONLY this audit entry

---

## Consumed Signatures

| Signature | Used By | Timestamp | Purpose | Spec Version | Evidence |
|-----------|---------|-----------|---------|--------------|----------|
| ilja_25122512345 | Orchestrator | 2025-12-25 12:34:56 UTC | Strategic Pause: Enrich storage layer scope | v02.84 | docs/task_packets/WP-1-Storage-Layer.md |
| ilja_25122513456 | Orchestrator | 2025-12-25 13:45:56 UTC | Packet creation: Phase 1 blocker tasks | v02.84 | docs/TASK_BOARD.md update |

---

**Verification**:
1. Orchestrator proposes enrichment + signature
2. User provides signature: {username}{DDMMYYYYHHMM}
3. Orchestrator verifies reuse: `grep -r "{signature}" . | grep -c signature_audit.md` == 1
4. If count != 1, FAIL and reject
5. Log signature and proceed
```

**Integration with workflow**:
- Orchestrator fetches fresh signature before enriching spec or creating packet
- Checks SIGNATURE_AUDIT.md for reuse via grep
- Records signature immediately after consumption
- Prevents autonomous drift by forcing human decision points

---

## Evidence Model: Requirement → Code → Test

**Pattern**: Every DONE_MEANS requirement maps to code evidence + test proof

**Structure in Packet**:

```markdown
## Evidence Mapping

| DONE_MEANS | SPEC_ANCHOR | Code Evidence | Test Command |
|------------|-------------|---------------|--------------|
| "AppState exposes DB as Arc<dyn Trait>" | §2.3.12.3 | src/main.rs:42-55 | grep "Arc<dyn" src/api/*.rs \| wc -l |
| "No SqlitePool leakage to API layer" | §2.3.12.3 | src/api/handler.rs (imports checked) | grep -r "SqlitePool" src/api/ \| wc -l |
| "Auth accepts email+password via POST" | §4.1 | src/api/auth.rs:100-150 | curl -X POST /auth/login -d '...' |
```

**Validator verification**:
1. Read DONE_MEANS
2. For each, locate file:line evidence
3. Run test command
4. Record result in VALIDATION block
5. FAIL if: evidence not found, test command fails, or test doesn't actually verify requirement

**Evidence must be**:
- Deterministic (same command always returns same result)
- Removable (if code deleted, test fails)
- Specific (file:line, not just "looks good")

---

## Git Hook Integration

**File**: `scripts/hooks/pre-commit`

**Purpose**: Prevent broken commits; enforce codex invariants before code enters tree

**Commands to run**:
```bash
#!/bin/bash
set -e

# 1. Verify hard invariants (codex)
node scripts/validation/codex-check.mjs

# 2. Warn about TODOs without tracking IDs
grep -r "TODO()" src/ app/ && echo "❌ TODOs must have tracking ID: TODO(HSK-####)" && exit 1

# 3. Forbid placeholder values
grep -r "{" src/ app/ --include="*.rs" --include="*.ts" --include="*.tsx" && \
  echo "⚠️  Found placeholder text in code; this won't work" && exit 1

# 4. Lint and format
cargo fmt --check || exit 1
pnpm run lint --quiet || exit 1

echo "✅ Pre-commit checks passed"
```

**Wire via**:
```bash
git config core.hooksPath scripts/hooks
```

**Effect**: Commit blocked until checks pass; catches errors before pushing

---

## Documentation Structure (Project-Agnostic)

Create `docs/` directory with:

```
docs/
├── START_HERE.md              # Entry point, repo map, AI agent workflow
├── SPEC_CURRENT.md            # Pointer: "Current spec is Master_Spec_vX.Y.Z.md"
├── ARCHITECTURE.md            # Module layout, responsibilities, entry points
├── RUNBOOK_DEBUG.md           # Error codes, bug triage, debug patterns
├── QUALITY_GATE.md            # Risk tier definitions, Gate 0/1 requirements
├── TASK_BOARD.md              # Master task status table + dependency tracking
├── SIGNATURE_AUDIT.md         # Immutable registry of consumed signatures
├── OWNERSHIP.md               # Path/area ownership (optional)
├── task_packets/
│   ├── README.md              # Packet naming convention, validation commands
│   ├── WP-1-*.md              # (20-30 actual packets during active work)
│   └── TEMPLATE.md            # (Packet template for copy-paste)
├── adr/                       # Architecture Decision Records
│   └── ADR-0001-workflow.md
└── agents/
    └── AGENT_REGISTRY.md      # Map of contributing agents/models
```

**Master Spec location** (root level):
```
Master_Spec_v1.0.md            # Current authoritative spec (~1MB+)
Master_Spec_v0.9.md            # (Previous versions archived in root)
{Project} Codex v1.0.md        # Governance rules (versioned)
```

---

## Phase Gate Pattern (Blocking Phase Progression)

**Command**: `just validator-phase-gate {PHASE_ID}`

**What it does**:
1. Reads `docs/TASK_BOARD.md` "Phase {PHASE_ID} Closure Gates (BLOCKING)"
2. Verifies every listed WP is status `Done` AND `VALIDATED`
3. Runs `just validator-spec-regression` (spec must be current)
4. Checks no unresolved dependencies
5. Returns 0 (clear to proceed) or 1 (blockers remain)

**Usage**:
```bash
# Before closing Phase 1, run:
just validator-phase-gate Phase-1

# Output:
# ❌ BLOCKED: WP-1-Storage-Layer not VALIDATED (status: Done)
# ❌ BLOCKED: WP-1-AppState-Refactor not VALIDATED (status: In-Progress)
# Exit 1
```

**Gates phase progression** (used in release workflows, CI/CD):
```bash
if just validator-phase-gate Phase-1; then
  echo "✅ Phase 1 ready to close"
  git tag phase-1-complete
else
  echo "❌ Blockers remain; cannot close phase"
  exit 1
fi
```

---

## Immediate Recommendations
- **Implement signature pause protocol:** Before creating any packet or enriching specs, Orchestrator must propose DONE_MEANS, TEST_PLAN, scope, RISK_TIER, and validator rubric. User and Orchestrator collaborate to align. Signature locks the collaboration notes and frozen intent. Update `scripts/create-task-packet.mjs` to include Signature & Enrichment Log template.
- **Standardize packet creation** via `scripts/create-task-packet.mjs` and enforce `just pre-work WP-{id}` before every handoff.
- **Create validator suite** with modular scripts (scan, spec-regression, custom audits). Wire all into `justfile` for uniform CLI.
- **Set up Git hooks** (`scripts/hooks/pre-commit`) to enforce hard invariants before code enters tree.
- **Create SIGNATURE_AUDIT.md** and enforce one-time-use signatures via grep on every enrichment.
- **Document TASK_BOARD.md** with Phase N Closure Gates explicitly listing blocking WPs; gate progression via `just validator-phase-gate`.
- **Keep Task Board and packet STATUS changes atomic** (edit both together); reject work if either drifts.
- **Keep retrieval sets tight:** SPEC_ANCHOR slice + packet (esp. Signature Log + DONE_MEANS) + latest validation block; avoid pasting whole specs to prevent context rot.
- **Train all four agents on the signature pause protocol:** Orchestrator must always propose before pausing for signature. Coder and Validator must honor locked DONE_MEANS and TEST_PLAN. Tool Agent respects scope and audit constraints.
- **Bootstrap repository checklist** (for new projects):
  1. Create master spec file (versioned, with SPEC_ANCHOR hierarchies)
  2. Create docs/SPEC_CURRENT.md pointer
  3. Create docs/TASK_BOARD.md template + Phase definitions
  4. Create docs/SIGNATURE_AUDIT.md (empty initially)
  5. Create scripts/validation/*.mjs validators (at least pre-work, post-work, spec-regression)
  6. Wire into justfile
  7. Create scripts/hooks/pre-commit + wire git config
  8. Create task packet template
  9. Document protocols (ORCHESTRATOR_PROTOCOL.md, CODER_PROTOCOL.md, VALIDATOR_PROTOCOL.md)
  10. Train agents on the workflow
