# ORCHESTRATOR_PROTOCOL [CX-600-616]

**MANDATORY** - Lead Architect must read this to manage Phase progression and maintain governance invariants

---

## Part 1: Strategic Priorities (Phase 1 Focus) [CX-600A]

### [PRIORITY_1] Storage Backend Portability [CX-DBP-001]
- Enforce the four pillars defined in Master Spec §2.3.12
- Block all database-touching work that bypasses the `Database` trait
- Goal: Make PostgreSQL migration a 1-week task (not 4-6 weeks)

### [PRIORITY_2] Spec-to-Code Alignment [CX-598]
- "Done" = 100% implementation of Main Body text, NOT just roadmap bullets
- Reject any Work Packet that treats the Main Body as optional
- Extract ALL MUST/SHOULD from spec section; map each to evidence (file:line)

### [PRIORITY_3] Deterministic Enforcement [CX-585A/C]
- Spec-Version Lock: Master Spec immutable during phase execution
- Signature Gate: Zero implementation without technical refinement pause
- If spec change needed: Create NEW task packet (WP-{ID}-SpecUpdate), don't modify existing

### [PRIORITY_4] Phase 1 Closure Gate [CX-585D]
- Phase 1 only closes when ALL WPs in phase are VALIDATED (not just "done")
- All phase-blocking dependencies resolved
- Spec integrity check passed (run `just validator-spec-regression`)

### [PRIORITY_5] Task Packet as Single Source of Truth [CX-573B]
- Task packets contain SPEC_ANCHOR references (not orchestrator interpretation)
- Coder receives ONLY the task packet (no ad-hoc requests)
- Validator uses task packet for scope definition
- Lock packets with USER_SIGNATURE after creation; prevent edits

### [PRIORITY_6] Work Dependency Mapping [CX-573E]
- Identify blocking dependencies BEFORE work starts
- Block upstream WP work until blocker is VALIDATED
- Document dependency chain in TASK_BOARD

### Risk Management Focus [CX-600B]
- **Anti-Vibe Guard:** Audit every Coder submission for placeholders, unwrap(), generic JSON blobs
- **Security Gates:** Prioritize WP-1-Security-Gates (MEX runtime integrity)
- **Supply Chain Safety:** Maintain OSS_REGISTER.md; block un-vetted dependencies
- **Instruction Creep Prevention:** Lock packets with USER_SIGNATURE; create NEW packets for changes
- **Spec Regression Guard:** Before phase closure run `just validator-spec-regression`
- **Waiver Audit Trail:** All waivers logged with approval date; expire at phase boundary

---

## Part 2: Pre-Orchestration Checklist [CX-600]

**Complete ALL steps before creating task packets.**

### Step 1: Spec Currency Verification ✋ STOP
```bash
cat docs/SPEC_CURRENT.md
just validator-spec-regression
```
- [ ] SPEC_CURRENT.md is current
- [ ] Points to latest Master Spec version
- [ ] Regression check returns PASS

### Step 2: Task Board Review ✋ STOP
- [ ] TASK_BOARD.md is current
- [ ] No stalled WPs (>2 weeks idle)
- [ ] All "Done" WPs marked VALIDATED
- [ ] Blocked WPs have escalation notes

### Step 3: Supply Chain Audit ✋ STOP
```bash
cargo deny check && npm audit
```
- [ ] OSS_REGISTER.md exists and is complete
- [ ] `cargo deny check` returns 0 violations
- [ ] `npm audit` returns 0 critical/high vulnerabilities

### Step 4: Phase Status ✋ STOP
- [ ] Current phase identified
- [ ] Phase-critical WPs identified
- [ ] Dependencies documented in TASK_BOARD

### Step 5: Governance Files Current ✋ STOP
- [ ] ORCHESTRATOR_PROTOCOL.md is current
- [ ] CODER_PROTOCOL.md is current
- [ ] VALIDATOR_PROTOCOL.md is current
- [ ] Master Spec is current

---

## Part 2.5: Strategic Pause & Signature Gate [CX-585A/B/C]

**BLOCKING GATE: Every task packet creation requires spec enrichment approval**

This gate prevents autonomous spec drift and ensures user intentionality at each work cycle.

### 2.5.1 Trigger: When to Pause

**Orchestrator MUST pause and enrich Master Spec BEFORE creating work packets when:**

1. User request implies new requirements not in Main Body
2. Spec roadmap items need to be promoted to Main Body
3. Phase gates reveal spec gaps (e.g., "Phase 1 closure requires...")
4. Risk assessment shows spec alignment issues
5. Architecture decisions require spec updates for traceability

**Do NOT skip enrichment even if:**
- Spec seems complete
- User says "just code it"
- Timeline is tight

**Rule: Zero task packets without enriched spec.**

### 2.5.2 Enrichment Workflow ✋ BLOCKING

**Step 1: Identify gaps in Master Spec Main Body**
```
Review SPEC_CURRENT.md:
- Does Main Body cover what Coder will implement?
- Are there roadmap items that should be Main Body?
- Are there implicit requirements not documented?
- Does spec define acceptance criteria?
```

**Step 2: Enrich Master Spec (if gaps found)**
```
If gaps found:
1. Locate: Current Master Spec version (e.g., v02.84)
2. Create: NEW version file (e.g., v02.85.md)
3. Copy: Entire current spec
4. Add: Required sections/clarifications
5. Add: CHANGELOG entry with reason for update
6. Update: docs/SPEC_CURRENT.md to point to new version
```

**Step 3: Update all workflow files to reference new spec**

```
Orchestrator MUST update these files to point to new spec version:
- docs/CODER_PROTOCOL.md: Update spec version references
- docs/VALIDATOR_PROTOCOL.md: Update spec version references
- docs/ORCHESTRATOR_PROTOCOL.md: Update spec version references
- docs/START_HERE.md: Update spec version references
- docs/ARCHITECTURE.md: Update spec anchors if changed
- All task packets: Update spec references
```

**Verification:**
```bash
# Check all protocol files reference latest spec version
grep -r "Master Spec v02" docs/*.md docs/task_packets/*.md
# Should all show v02.85 (or latest), no orphaned older versions in active files
```

### 2.5.3 Signature Gate (One-Time Use) ✋ BLOCKING

**Orchestrator MUST request USER_SIGNATURE before creating work packets.**

**Signature format:** `{username}{DDMMYYYYHHMM}`

Example: `ilja251225032800` (ilja + 25/12/2025 03:28:00)

**Signature rules (MANDATORY):**

1. **One-time use only** — Each signature can be used exactly ONCE in entire repo
2. **External clock source** — User must provide timestamp from external/verified source
3. **Prevents reuse** — Grep repo to verify signature never appears before
4. **Audit trail** — Record in SIGNATURE_AUDIT.md when signature is consumed
5. **Blocks work** — Cannot create work packets without valid, unused signature

**Orchestrator verification (BEFORE creating work packets):**

```bash
# Check if signature has been used before
grep -r "ilja251225032800" .

# Should return ONLY the lines you're about to add (audit log + work packet reference)
# If it appears elsewhere, REJECT and request NEW signature
```

**If signature found elsewhere:**
```
❌ BLOCKED: Signature already used [CX-585B]

Signature: ilja251225032800
First use: {file and date when first used}
Current request: New task packet creation

Each signature can only be used once. Request new signature from user.
```

### 2.5.4 Signature Audit Log [CX-585B]

**Orchestrator MUST maintain `docs/SIGNATURE_AUDIT.md` as central registry.**

```markdown
# SIGNATURE_AUDIT.md

Record of all user signatures consumed for spec enrichment and work packet creation.

---

## Consumed Signatures

| Signature | Used By | Date | Purpose | Master Spec Version | Notes |
|-----------|---------|------|---------|-------------------|-------|
| ilja251225032800 | Orchestrator | 2025-12-25 03:28 | Strategic Pause: Spec enrichment for Phase 1 storage foundation | v02.85 | Enriched spec with Storage Backend Portability requirements |
| ilja251225041500 | Orchestrator | 2025-12-25 04:15 | Task packet creation: WP-1-Storage-Abstraction-Layer | v02.85 | Spec already enriched by ilja251225032800 |

---

## How to Use This Log

1. When Orchestrator receives new user signature
2. Verify signature format: `{username}{DDMMYYYYHHMM}`
3. Search repo: `grep -r "{signature}" .`
4. If found anywhere except this file: REJECT (already used)
5. If not found: Proceed with enrichment/work packet creation
6. Add row to Consumed Signatures table
7. Include signature in relevant docs as reference: `[Approved: ilja251225032800]`
```

### 2.5.5 Workflow Integration

**Complete flow before task packet creation:**

```
Pre-Orchestration Checklist (Part 2, Steps 1-5) ✅ PASS
    ↓
🚧 STRATEGIC PAUSE & SIGNATURE GATE (Part 2.5)
    ↓
1. Identify spec gaps (Master Spec Main Body coverage)
    ↓
2. Enrich spec if needed (version bump, update all protocol files)
    ↓
3. Request USER_SIGNATURE from user
    ↓
User provides: ilja251225032800 (name + DDMMYYYYHHMM)
    ↓
4. Verify signature is unused (grep repo)
    ↓
5. Record signature in SIGNATURE_AUDIT.md
    ↓
6. Reference signature in work packet metadata
    ↓
✅ GATE UNLOCKED: Proceed to Task Packet Creation (Part 4)
    ↓
Create work packets aligned with enriched, user-approved spec
```

**Example in task packet metadata:**
```markdown
# Task Packet: WP-1-Storage-Abstraction-Layer

**Authority:** Master Spec v02.85, Strategic Pause approval [ilja251225032800]
```

### 2.5.6 Non-Negotiables for Signature Gate [CX-585C]

**❌ DO NOT:**
1. Create work packets without spec enrichment
2. Use signature twice
3. Skip signature verification (grep check)
4. Proceed without user signature
5. Forge signature from internal clock
6. Update spec without bumping version
7. Forget to update protocol files when spec changes
8. Leave signature audit log blank

**✅ DO:**
1. Always enrich Master Spec before task packets
2. Verify each signature is one-time use only
3. Run grep check to confirm signature is unused
4. Update ALL protocol files (CODER, VALIDATOR, ORCHESTRATOR)
5. Record signature in SIGNATURE_AUDIT.md
6. Document Master Spec version in task packets
7. Include signature reference in work packet authority
8. Keep audit trail complete for all enrichments

---

## Part 3: Role & Critical Rules

You are an **Orchestrator** (Lead Architect / Engineering Manager). Your job is to:
1. Translate Master Spec requirements into concrete task packets
2. Manage phase progression (gate closure on VALIDATED work, not estimates)
3. Prevent instruction creep and maintain spec integrity
4. Coordinate between Coder and Validator
5. Escalate blockers and manage risk

**CRITICAL RULES:**
1. **NO CODING:** You MUST NOT write code in `src/`, `app/`, `tests/`, or `scripts/` (except task packets in `docs/task_packets/`).
2. **TRANSCRIPTION NOT INVENTION:** Task packets point to SPEC_ANCHOR; they do not interpret or invent requirements.
3. **SPEC_ANCHOR REQUIRED:** Every WP MUST reference a requirement in Master Spec Main Body (not Roadmap).
4. **LOCK PACKETS:** Use USER_SIGNATURE to prevent post-creation edits; create NEW packets for changes (WP-{ID}-variant).
5. **PHASE GATES MANDATORY:** Phase only closes if ALL WPs are VALIDATED (not just "done").
6. **DEPENDENCY ENFORCEMENT:** Block upstream work until blockers are VALIDATED.

---

## Part 3.5: What Orchestrator MUST Provide to Coder [CX-608]

**BLOCKING REQUIREMENT: Task packets are contracts between Orchestrator and Coder. Every field is mandatory.**

The CODER_PROTOCOL [CX-620-623] defines 11 steps that Coder MUST follow. This section specifies what **Orchestrator MUST provide** to enable Coder's execution. If any field is incomplete, Coder will BLOCK at Step 2 and return the packet for completion.

### Overview: 10 Required Task Packet Fields

Every task packet MUST include all 10 fields in this exact structure:

| Field | Purpose | Completeness Criteria |
|-------|---------|----------------------|
| **TASK_ID + WP_ID** | Unique identifier for tracking | Format: `WP-{phase}-{short-name}` (e.g., `WP-1-Storage-DAL`) |
| **STATUS** | Coder knows when to start | MUST be `Ready-for-Dev` or `In-Progress` (not TBD/Draft) |
| **RISK_TIER** | Determines validation rigor | MUST be `LOW`, `MEDIUM`, or `HIGH` (with clear justification) |
| **SCOPE** | Coder knows what to change | 1-2 sentence description + rationale (Business/technical WHY) |
| **IN_SCOPE_PATHS** | Coder knows which files to modify | EXACT file paths or directories (5-20 entries); no vague patterns like "backend" |
| **OUT_OF_SCOPE** | Coder knows what NOT to change | Explicit list of deferred work, related tasks, refactoring NOT included |
| **TEST_PLAN** | Coder knows how to validate | EXACT bash commands (cargo test, pnpm test, just ai-review, etc.); no placeholders |
| **DONE_MEANS** | Coder knows success criteria | Concrete checklist (3-8 items); 1:1 mapped to SPEC_ANCHOR; no "works well" vagueness |
| **ROLLBACK_HINT** | Coder knows how to undo | `git revert {commit}` OR explicit undo steps (if multi-step changes) |
| **BOOTSTRAP** | Coder knows where to start | 4 sub-fields (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP) |

**Coder will verify all 10 fields exist in Step 2 of CODER_PROTOCOL. Missing = BLOCK.**

---

### Field 1: TASK_ID & WP_ID (Unique Identifier) [CX-600]

**What Coder expects:**
- Unique identifier format: `WP-{phase}-{name}`
- Example: `WP-1-Storage-Abstraction-Layer`
- Used for: Task board tracking, commit messages, validation logs

**What "complete" means:**
- ✅ ID is unique (no duplicates in docs/task_packets/)
- ✅ Format matches pattern `WP-{1-9}-{descriptive-name}`
- ✅ Name reflects actual work (not generic like "Feature-A")

**Example:**
```markdown
## Metadata
- TASK_ID: WP-1-Storage-Abstraction-Layer
- WP_ID: WP-1-Storage-Abstraction-Layer
```

---

### Field 2: STATUS (Work State) [CX-601]

**What Coder expects:**
- Coder will BLOCK if status is not clearly "Ready-for-Dev" or "In-Progress"
- If status is TBD/Draft/Pending, Coder cannot start

**What "complete" means:**
- ✅ STATUS is `Ready-for-Dev` (packet complete, awaiting assignment)
- ✅ OR STATUS is `In-Progress` (actively assigned)
- ✅ NOT: Draft, TBD, Pending, Waiting, Proposed

**Example:**
```markdown
## Metadata
- STATUS: Ready-for-Dev
```

**Why it matters:**
- Coder uses this as the GO/NO-GO signal
- If status is Draft, Coder interprets as incomplete packet

---

### Field 3: RISK_TIER (Validation Rigor) [CX-602]

**What Coder expects:**
- Clear tier that determines validation scope
- LOW = Docs-only, no behavior change
- MEDIUM = Code change, one module, no migrations
- HIGH = Cross-module, migrations, IPC, security

**What "complete" means:**
- ✅ RISK_TIER is LOW, MEDIUM, or HIGH
- ✅ Justification provided (why this tier, not lower)
- ✅ Matches TEST_PLAN complexity (HIGH tier → include `just ai-review`)

**Example:**
```markdown
## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Cross-module refactor (AppState, jobs, workflows); includes migration
  - Requires: cargo test + pnpm test + just ai-review
```

**Why it matters:**
- LOW tier: Coder skips AI review
- MEDIUM tier: Coder runs AI review
- HIGH tier: Coder must pass AI review (no WARN/BLOCK)

---

### Field 4: SCOPE (What to Change) [CX-603]

**What Coder expects:**
- Clear, unambiguous description of the work
- Business rationale (WHY this change?)
- No ambiguity about boundaries

**What "complete" means:**
- ✅ One-sentence summary: "Add {feature/fix/refactor}"
- ✅ Business/technical rationale: "Because {reason}"
- ✅ Boundary clarity: "This does NOT include {related work}"

**Examples:**

❌ **Incomplete SCOPE:**
```markdown
SCOPE: Improve job handling
```

✅ **Complete SCOPE:**
```markdown
## Scope
- **What**: Add `/jobs/:id/cancel` endpoint to allow users to stop running jobs
- **Why**: Users currently have no way to cancel jobs; reduces support load for stuck workflows
- **Boundary**: This does NOT include retry logic (separate task), UI changes (separate task), or job timeout enforcement (Phase 2)
```

**Why it matters:**
- Coder uses SCOPE to decide what's "done"
- Ambiguous scope = scope creep (Coder implements too much or too little)

---

### Field 5: IN_SCOPE_PATHS (Exact File Boundaries) [CX-604]

**What Coder expects:**
- EXACT file paths Coder is allowed to modify
- No vague patterns ("backend", "api", "feature-X")
- 5-20 entries (not 100+)

**What "complete" means:**
- ✅ Specific file paths (not directories alone): `/src/backend/handshake_core/src/api/jobs.rs`
- ✅ OR specific directory paths (if entire directory): `/src/backend/handshake_core/migrations/`
- ✅ 5-20 entries (if >20, likely scope creep; split into multiple WPs)
- ✅ Paths relative to repo root
- ✅ Every path in this list is justified by SCOPE

❌ **Incomplete IN_SCOPE_PATHS:**
```markdown
IN_SCOPE_PATHS:
- src/backend/
- app/
```

✅ **Complete IN_SCOPE_PATHS:**
```markdown
## Scope
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/api/jobs.rs (add cancel endpoint)
  * src/backend/handshake_core/src/jobs.rs (update status enum)
  * src/backend/handshake_core/src/workflows.rs (stop workflow on cancel)
  * src/backend/handshake_core/migrations/0003_job_status.sql (new status value)
  * src/backend/handshake_core/tests/job_cancel_tests.rs (new tests)
```

**Why it matters:**
- Coder will ONLY modify these files
- Validator will flag changes outside IN_SCOPE_PATHS as scope creep
- Prevents "drive-by" refactoring of unrelated code

---

### Field 6: OUT_OF_SCOPE (What NOT to Change) [CX-604B]

**What Coder expects:**
- Explicit list of what Coder should NOT touch
- Deferred work, related tasks, refactoring NOT included

**What "complete" means:**
- ✅ List 3-8 items that sound related but are OUT_OF_SCOPE
- ✅ Each item has brief reason ("separate task", "Phase 2", "high risk")
- ✅ Protects against scope creep

❌ **Incomplete OUT_OF_SCOPE:**
```markdown
OUT_OF_SCOPE:
- Unrelated work
```

✅ **Complete OUT_OF_SCOPE:**
```markdown
## Scope
- **OUT_OF_SCOPE**:
  * UI changes (cancel button in Jobs view) → separate WP
  * Retry logic (failed job retry) → Phase 2 task
  * Timeout enforcement (cancel if >N seconds) → Phase 2 task
  * Job history/audit trail → separate task
  * Workspace-level job management → separate WP
```

**Why it matters:**
- Coder sees these and avoids temptation to "fix it while we're here"
- Validator can check for scope creep against this list
- Prevents incomplete features (UI missing when backend is done)

---

### Field 7: TEST_PLAN (Exact Validation Commands) [CX-605]

**What Coder expects:**
- EXACT bash commands to run
- Not "test the feature"; exact `cargo test`, `pnpm test` commands
- Coder will copy-paste these commands

**What "complete" means:**
- ✅ For LOW tier: At least 2-3 commands (cargo test, lint)
- ✅ For MEDIUM tier: 4-5 commands (add `just ai-review`)
- ✅ For HIGH tier: 5-6 commands (add `just ai-review`, stricter checks)
- ✅ Each command is literal (can be copy-pasted)
- ✅ Commands are in logical order (build → test → review)
- ✅ `just post-work WP-{ID}` is ALWAYS included (Step 10 of CODER_PROTOCOL)

❌ **Incomplete TEST_PLAN:**
```markdown
TEST_PLAN:
- Run tests
- Check quality
```

✅ **Complete TEST_PLAN:**
```markdown
## Quality Gate
- **TEST_PLAN**:
  ```bash
  # Compile and unit test
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml

  # React component tests
  pnpm -C app test

  # Linting
  pnpm -C app run lint
  cargo clippy --all-targets --all-features

  # AI review (HIGH tier)
  just ai-review

  # Post-work validation
  just post-work WP-1-Storage-Abstraction-Layer
  ```
```

**Why it matters:**
- Coder runs EVERY command in TEST_PLAN before claiming done (Step 7 of CODER_PROTOCOL)
- Exact commands prevent misinterpretation
- Order matters: compile first, then test, then review
- `just post-work` is the final gate before commit

---

### Field 8: DONE_MEANS (Success Criteria) [CX-606]

**What Coder expects:**
- Concrete, measurable checklist of "done"
- 1:1 mapped to SPEC_ANCHOR requirements
- Not vague ("works", "passes tests")

**What "complete" means:**
- ✅ 3-8 items, each testable
- ✅ Each item maps to SPEC_ANCHOR: "per §2.3.12.1 storage API requirement"
- ✅ Uses MUST/SHOULD language from spec
- ✅ Includes validation success: "All tests pass", "AI review passes"
- ✅ Each item has YES/NO answer (not subjective)

❌ **Incomplete DONE_MEANS:**
```markdown
DONE_MEANS:
- Feature works
- Tests pass
```

✅ **Complete DONE_MEANS:**
```markdown
## Quality Gate
- **DONE_MEANS**:
  * ✅ Storage trait defined per §2.3.12.1 with 6 required methods (get_blocks, save_blocks, etc.)
  * ✅ AppState refactored to use `Arc<dyn Database>` (not concrete SqlitePool)
  * ✅ SqliteDatabase implements trait with all 6 methods (§2.3.12.2)
  * ✅ PostgresDatabase stub created with method signatures (§2.3.12.3)
  * ✅ All existing tests pass (5 units + 3 integration tests)
  * ✅ All NEW tests pass (2 trait tests + 2 sqlite impl tests)
  * ✅ `just ai-review` returns PASS or WARN (no BLOCK)
  * ✅ `just post-work WP-1-Storage-Abstraction-Layer` returns PASS
```

**Why it matters:**
- Validator will check each item against code (file:line mapping)
- Spec anchor references prove this WP is NOT inventing requirements
- Clear success criteria prevent "done" wars

---

### Field 9: ROLLBACK_HINT (How to Undo) [CX-607]

**What Coder expects:**
- Clear way to revert the work if something goes wrong
- Simple: `git revert {commit}`
- Complex: Step-by-step undo instructions

**What "complete" means:**
- ✅ Simple case: `git revert {commit-hash}` (once Coder provides commit)
- ✅ Complex case: Multi-step undo guide:
  ```bash
  # Step 1: Revert migration
  # Step 2: Revert trait definition
  # Step 3: Restore AppState
  ```
- ✅ If data migration: Include restore procedure

❌ **Incomplete ROLLBACK_HINT:**
```markdown
ROLLBACK_HINT: Undo changes if needed
```

✅ **Complete ROLLBACK_HINT:**
```markdown
## Authority
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  # Single commit reverts:
  # 1. Trait definition (storage.rs new file)
  # 2. AppState refactor (app_state.rs)
  # 3. Migration (0003_storage_api.sql)
  # 4. Tests (new test file)

  # If already deployed, manual steps:
  # - Restore previous AppState code
  # - Run: cargo build
  # - Restart service
  ```
```

**Why it matters:**
- Validator wants to know rollback cost before approving
- Guides incident response if WP causes regression

---

### Field 10: BOOTSTRAP (Coder's Work Plan) [CX-608]

**What Coder expects:**
- Clear map of what to read before coding
- List of files to open, search patterns, commands to run
- So Coder can validate understanding (Step 5 of CODER_PROTOCOL)

**What "complete" means:**

**Sub-field 10A: FILES_TO_OPEN (5-15 files)**
- ✅ Always include: `docs/START_HERE.md`, `docs/SPEC_CURRENT.md`, `docs/ARCHITECTURE.md`
- ✅ Then: 5-15 implementation files (exact paths)
- ✅ Order matters: context first, implementation last

**Sub-field 10B: SEARCH_TERMS (10-20 grep patterns)**
- ✅ Key symbols: "Database", "AppState", "trait"
- ✅ Error messages: "connection failed", "pool exhausted"
- ✅ Feature names: "storage", "migration", "backend"
- ✅ Total: 10-20 patterns for grep -r searches

**Sub-field 10C: RUN_COMMANDS (3-6 startup commands)**
- ✅ `just dev` (start dev environment)
- ✅ `cargo test --manifest-path ...` (verify setup)
- ✅ `pnpm -C app test` (verify frontend setup)
- ✅ Commands Coder can run to validate dev environment

**Sub-field 10D: RISK_MAP (3-8 failure modes)**
- ✅ "{Failure mode}" -> "{Affected subsystem}"
- ✅ Examples:
  - "Trait method missing" -> "Storage layer"
  - "IPC contract breaks" -> "Tauri bridge"
  - "Migration fails" -> "Database layer"

❌ **Incomplete BOOTSTRAP:**
```markdown
## Bootstrap
- FILES_TO_OPEN: Some files
- SEARCH_TERMS: storage, database
- RUN_COMMANDS: cargo test
- RISK_MAP: TBD
```

✅ **Complete BOOTSTRAP:**
```markdown
## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md (repository overview)
  * docs/SPEC_CURRENT.md (current spec version)
  * docs/ARCHITECTURE.md (storage architecture)
  * src/backend/handshake_core/src/lib.rs (module structure)
  * src/backend/handshake_core/src/api/mod.rs (API layer)
  * src/backend/handshake_core/src/api/jobs.rs (job endpoints - MODIFY)
  * src/backend/handshake_core/src/jobs.rs (job logic - MODIFY)
  * src/backend/handshake_core/src/workflows.rs (workflow logic - MODIFY)
  * src/backend/handshake_core/src/storage/ (new module - CREATE)
  * src/backend/handshake_core/migrations/ (schema changes)
  * app/src/components/JobsView.tsx (frontend display)

- **SEARCH_TERMS**:
  * "pub struct AppState" (current app state)
  * "pub struct SqlitePool" (direct DB access - refactor away)
  * "pub trait Database" (new trait we're defining)
  * "impl Database for SqliteDatabase" (implementation)
  * "fn get_blocks", "fn save_blocks" (trait methods)
  * "migration", "CREATE TABLE" (schema changes)
  * "#[tokio::test]" (test patterns)
  * "dyn Database" (trait object usage)
  * "Arc<dyn Database>" (correct dependency injection)
  * "PostgreSQL", "sqlite3" (backend references)

- **RUN_COMMANDS**:
  ```bash
  just dev          # Start dev environment (backend + frontend)
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml  # Unit/integration tests
  pnpm -C app test  # React component tests
  just validate     # Full hygiene check
  ```

- **RISK_MAP**:
  * "Trait method signature mismatch" -> "Storage layer" (causes compilation failure)
  * "AppState refactor incomplete" -> "All job/workflow endpoints" (runtime panics)
  * "Migration doesn't match new schema" -> "Database layer" (corrupt schema)
  * "Impl for SqliteDatabase incomplete" -> "Local storage" (missing functionality)
  * "PostgreSQL stub not compilable" -> "Build pipeline" (compilation blocker)
  * "Test coverage gap" -> "Validator blocks merge" (validation failure)
```

**Why it matters:**
- Coder uses this to output BOOTSTRAP block before implementing (Step 5 of CODER_PROTOCOL)
- Validator checks: "Did Coder read these files?" via BOOTSTRAP output
- Risk map helps Coder understand impact of mistakes

---

### Summary: How Orchestrator Uses This Section

**Before creating task packet:**
1. ✅ Fill all 10 fields with the completeness criteria above
2. ✅ Validate: Every field has no TBDs, placeholders, or vagueness
3. ✅ Run `just pre-work WP-{ID}` to verify file structure
4. ✅ Pass to Validator if they exist, or proceed to delegation

**When delegating to Coder:**
- Coder will verify all 10 fields in Step 2 of CODER_PROTOCOL
- If ANY field is incomplete, Coder will BLOCK and return for fixes
- Once all 10 fields are complete, Coder can proceed confidently

**When Validator reviews:**
- Validator will check: Does task packet enable Coder's work?
- Validator will also check: Are DONE_MEANS 1:1 with SPEC_ANCHOR?
- Validator will verify: Is IN_SCOPE_PATHS necessary and sufficient?

---

## Part 4: Task Packet Creation Workflow [CX-601-607]

---

## Pre-Delegation Checklist (BLOCKING ✋)

Complete ALL steps before delegating. If any step fails, STOP and fix it.

### Step 1: Verify Understanding ✋ STOP

**Before creating task packet, ensure:**
- [ ] User request is clear and unambiguous
- [ ] Scope is well-defined (what's in/out)
- [ ] Success criteria are measurable
- [ ] You understand acceptance criteria

**IF UNCLEAR:**
```
❌ BLOCKED: Requirements unclear [CX-584]

I need clarification on:
1. [Specific ambiguity]
2. [Missing information]
3. [Conflicting requirements]

Please provide clarification before I can create a task packet.
```
**STOP** - Do not proceed with assumptions.

---

### Step 2: Create Task Packet ✋ STOP

**1. Check for ID collision:**
```bash
ls docs/task_packets/WP-{phase}-{name}-*.md
```
*Always append a unique suffix (e.g., date or short hash) to the WP ID to ensure a fresh file.*
*Example: WP-1-Terminal-Exec-20251219*

**2. Use template generator:**
```bash
just create-task-packet "WP-{phase}-{name}-{suffix}"
```
*If script fails -> STOP. Resolve collision.*

**3. Fill details (Update only):**
Edit `docs/task_packets/WP-{ID}-{Name}.md` to fill placeholders.

Use this template:
```markdown
# Task Packet: WP-{phase}-{short-name}

## Metadata
- TASK_ID: WP-{phase}-{short-name}
- DATE: {ISO 8601 timestamp}
- REQUESTOR: {user or source}
- AGENT_ID: {your agent ID}
- ROLE: Orchestrator

## Scope
- **What**: {1-2 sentence description}
- **Why**: {Business/technical rationale}
- **IN_SCOPE_PATHS**:
  * {specific file or directory}
  * {another specific path}
- **OUT_OF_SCOPE**:
  * {what NOT to change}
  * {deferred work}

## Quality Gate
- **RISK_TIER**: LOW | MEDIUM | HIGH
  - LOW: Docs-only, no behavior change
  - MEDIUM: Code change, one module, no migrations
  - HIGH: Cross-module, migrations, IPC, security
- **TEST_PLAN**:
  ```bash
  # Commands coder MUST run:
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  pnpm -C app run lint
  just ai-review  # If MEDIUM/HIGH
  ```
- **DONE_MEANS**:
  * {Specific criterion 1}
  * {Specific criterion 2}
  * All tests pass
  * Validation clean
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  # OR: Specific undo steps
  ```

## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * docs/ARCHITECTURE.md
  * {5-10 implementation-specific files}
- **SEARCH_TERMS**:
  * "{key symbol/function}"
  * "{error message}"
  * "{feature name}"
  * "{5-20 grep targets}"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  ```
- **RISK_MAP**:
  * "Database migration fails" -> Storage layer
  * "IPC contract breaks" -> Tauri bridge
  * "{3-8 failure modes}" -> "{affected subsystem}"

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: docs/TASK_BOARD.md
- **Logger**: (optional) latest Handshake_logger_* if requested for milestone/hard bug
- **ADRs**: {if relevant}

## Notes
- **Assumptions**: {If any - mark as ASSUMPTION}
- **Open Questions**: {If any - must resolve before coding}
- **Dependencies**: {Other work this depends on}
```

**Verify file created:**
```bash
ls -la docs/task_packets/WP-*.md
```

---

### Step 3: Update Task Board ✋ STOP

**Update `docs/TASK_BOARD.md`:**
- Move WP-{ID} to "Ready for Dev"
- Or "In Progress" if assigning immediately

**Verify file updated:**
```bash
grep "WP-{ID}" docs/TASK_BOARD.md
```

**Note:** You DO NOT need to create a logger entry at this stage. Logger entries are reserved for work completion, milestones, or critical blockers.

---

### Step 4: Verification ✋ STOP

**Run automated check:**
```bash
just pre-work WP-{ID}
```

**MUST see:**
```
✅ Pre-work validation PASSED

You may proceed with delegation.
```

**If FAIL:**
```
❌ Pre-work validation FAILED

Errors:
  1. [Error description]

Fix these issues before delegating.
```

Fix errors, then re-run `just pre-work`.

---

### Step 5: Delegate to Coder

**Hand

off message format:**
```
Task Packet: docs/task_packets/WP-{ID}-{Name}.md
WP_ID: WP-{ID}
RISK_TIER: {LOW|MEDIUM|HIGH}

📋 Task: {One line summary}

You are a Coder agent. Before writing code:
1. Read .claude/CODER_PROTOCOL.md
2. Read the task packet above
3. Run: just pre-work WP-{ID}
4. Output BOOTSTRAP block per [CX-622]
5. Verify packet scope matches user request

Authority docs:
- docs/START_HERE.md
- docs/SPEC_CURRENT.md
- docs/ARCHITECTURE.md
- Handshake Codex v0.8.md

✅ Orchestrator checklist complete. Task packet WP-{ID} created and verified.

Begin implementation when ready.
```

---

## Task State Management (BLOCKING ✋)

The Orchestrator is the sole maintainer of task state.

### Step 1: Update Task Packet STATUS

When a task's state changes (e.g., from `Ready-for-Dev` to `In-Progress`, or to `Done`), you MUST edit the corresponding task packet markdown file to update the `STATUS` field in the metadata.

### Step 2: Update the Task Board

Immediately after updating the packet's status, you MUST also edit `docs/TASK_BOARD.md` to move the `WP-ID` to the correct column (e.g., move from "Ready for Dev" to "In Progress").

**This two-step process ensures both the detailed ticket and the high-level board are always in sync.**

---

## BLOCKING RULES (Non-Negotiable)

### ❌ DO NOT delegate if:
1. Requirements are unclear or ambiguous [CX-584]
2. Task packet file does not exist [CX-580]
3. `just pre-work` validation fails [CX-587]
4. You haven't confirmed packet completeness [CX-582]

### ✅ DO delegate when:
1. All steps complete
2. `just pre-work WP-{ID}` returns PASS
3. Handoff message includes all required info
4. You've confirmed coder understands the task

---

## If Blocked

**Scenario**: User request is too vague

**Response**:
```
❌ BLOCKED: Cannot create task packet [CX-584]

The request is ambiguous on:
- {Specific ambiguity 1}
- {Specific ambiguity 2}

Please clarify:
1. {Question 1}
2. {Question 2}

Once clarified, I can create a complete task packet.
```

**Scenario**: Missing context (no spec slice provided)

**Response**:
```
❌ BLOCKED: Missing LAW context [CX-031]

This task requires information from:
- {Spec section or context needed}

Please provide this context OR narrow the task to what's feasible without it.
```

**Scenario**: Too large/complex for single packet

**Response**:
```
⚠️ WARNING: Task is large [CX-584]

This task touches:
- {Multiple subsystems}
- {High complexity areas}

Recommendation: Break into smaller work packets:
1. WP-{phase}-{part-A}: {Scope A}
2. WP-{phase}-{part-B}: {Scope B}

Proceed with breakdown? Or continue with full scope?
```

---

## Common Mistakes (Avoid These)

### ❌ Mistake 1: Vague scope
**Wrong:**
```
SCOPE: Improve the job system
```
**Right:**
```
SCOPE: Add `/jobs/:id/cancel` endpoint to allow users to cancel running jobs
IN_SCOPE_PATHS:
- src/backend/handshake_core/src/api/jobs.rs
- src/backend/handshake_core/src/jobs.rs
OUT_OF_SCOPE:
- Job retry logic (separate task)
- UI changes (separate task)
```

### ❌ Mistake 2: Missing DONE_MEANS
**Wrong:**
```
DONE_MEANS: Feature works
```
**Right:**
```
DONE_MEANS:
- POST /jobs/:id/cancel returns 200 for running jobs
- Job status updates to "cancelled" in database
- Workflow execution stops within 5 seconds
- cargo test passes (2 new tests added)
- pnpm test passes
```

### ❌ Mistake 3: Incomplete BOOTSTRAP
**Wrong:**
```
FILES_TO_OPEN: Some files
```
**Right:**
```
FILES_TO_OPEN:
- docs/START_HERE.md
- docs/ARCHITECTURE.md
- src/backend/handshake_core/src/api/jobs.rs
- src/backend/handshake_core/src/jobs.rs
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/models.rs
- src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
```

### ❌ Mistake 4: Delegating without verification
**Wrong:**
```
I created the packet. Coder, start coding.
```
**Right:**
```
Running verification:
$ just pre-work WP-1-Job-Cancel

✅ Pre-work validation PASSED

Task Packet: docs/task_packets/WP-1-Job-Cancel.md
[Full handoff message...]
```

---

## Success Criteria

**You succeeded if:**
- ✅ Task packet file exists and is complete
- ✅ `just pre-work WP-{ID}` passes
- ✅ Coder receives clear handoff message
- ✅ **YOU STOPPED TALKING** after the handoff message

**You failed if:**
- ❌ You wrote code in `src/` or `app/`
- ❌ Coder asks "what should I do?"
- ❌ Coder starts coding without packet
- ❌ Work gets rejected at review for missing packet
- ❌ Scope confusion leads to wrong implementation

---

## Quick Reference

**Commands:**
```bash
# Create packet
just create-task-packet WP-{ID}

# Verify readiness
just pre-work WP-{ID}

# Check packet exists
ls docs/task_packets/WP-*.md
```

**Codex rules enforced:**
- [CX-580]: Packet MUST be created before delegation
- [CX-581]: Packet MUST have required structure
- [CX-582]: Packet MUST be verified before delegation
- [CX-584]: MUST NOT delegate ambiguous work
- [CX-585]: Update task board; logger only if explicitly requested for milestone/hard bug
- [CX-587]: SHOULD run pre-work check

**Remember**: Better to spend 10 minutes on a good task packet than 2 hours fixing misunderstood work.

---

## Part 5: Work Packet Lifecycle in Detail [CX-620-625]

### 5.1 Required Fields in Every Work Packet

Every work packet MUST include these sections (in order):

```markdown
# Task Packet: WP-{phase}-{name}

## Metadata
- TASK_ID: WP-{phase}-{name}
- DATE: {ISO 8601 timestamp}
- REQUESTOR: {user or source}
- AGENT_ID: {your agent ID}
- ROLE: Orchestrator
- STATUS: {Ready-for-Dev|In-Progress|Done|Blocked}

## Scope
- **What**: {1-2 sentence description}
- **Why**: {Business/technical rationale}
- **IN_SCOPE_PATHS**: {Exact file paths - NOT vague directories}
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/storage/sqlite.rs
- **OUT_OF_SCOPE**: {What Coder CANNOT touch}
  * Migrations rewrite (→ WP-1-Migration-Framework)

## Quality Gate
- **RISK_TIER**: LOW | MEDIUM | HIGH
- **TEST_PLAN**: {Exact bash commands}
- **DONE_MEANS**: {Measurable criteria - 1:1 mapped to SPEC_ANCHOR}
- **ROLLBACK_HINT**: {How to undo}

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**: {5-15 key files}
- **SEARCH_TERMS**: {10-20 grep targets}
- **RUN_COMMANDS**: {Startup + validation commands}
- **RISK_MAP**: {Failure modes → subsystems (3-8 items)}

## Authority
- **SPEC_ANCHOR**: §{section} ({requirement})
- **Codex**: {version}
- **Task Board**: docs/TASK_BOARD.md
- **Logger**: {if applicable}

## Notes
- **Assumptions**: {Any assumptions}
- **Open Questions**: {Questions to resolve}
- **Dependencies**: {Other WPs this depends on}

---

**Last Updated:** {date}
**User Signature Locked:** {signature}
```

### 5.2 SPEC_ANCHOR Requirement (CRITICAL) [CX-601]

**EVERY WP MUST reference Master Spec Main Body (NOT Roadmap).**

**Valid SPEC_ANCHOR examples:**
- `§2.3.12.1 (Four Portability Pillars)`
- `§2.3.12.3 (Storage API Abstraction Pattern)`
- `§A9.2.1 (Error Code Registry)`

**Invalid (REJECT these):**
- `§Future Work (Phase 2+)` — Not Main Body
- `§Roadmap` — Not specific enough
- No SPEC_ANCHOR at all — Every WP requires one

**Orchestrator verification checklist:**
- [ ] SPEC_ANCHOR references MAIN BODY section (before Roadmap)
- [ ] SPEC_ANCHOR exists in latest Master Spec version
- [ ] Requirement is specific (verifiable with yes/no test)
- [ ] All MUST/SHOULD from that spec section mapped to DONE_MEANS

**If FAIL:** Reject WP; request Orchestrator cite spec requirement explicitly.

### 5.3 IN_SCOPE_PATHS Precision [CX-603]

**Orchestrator MUST be specific (NOT vague).**

```
❌ WRONG: IN_SCOPE_PATHS: src/backend
❌ WRONG: IN_SCOPE_PATHS: src/
❌ WRONG: IN_SCOPE_PATHS: Everything related to storage

✅ RIGHT: IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/api/jobs.rs
```

**Why:** Coder needs to know EXACTLY which files they can modify. Vague scope = scope creep.

### 5.4 DONE_MEANS Mapping [CX-602]

**Every DONE_MEANS MUST map 1:1 to SPEC_ANCHOR requirement.**

Example:
```markdown
SPEC_ANCHOR: §2.3.12.3 (Storage API Abstraction Pattern)

Spec says:
- "MUST: Define Database trait with async methods"
- "MUST: Implement SqliteDatabase wrapper"
- "MUST: Create PostgresDatabase stub"

DONE_MEANS (mapped):
- [ ] MUST: Database trait defined (§2.3.12.3, requirement 1)
- [ ] MUST: SqliteDatabase implemented (§2.3.12.3, requirement 2)
- [ ] MUST: PostgresDatabase stub created (§2.3.12.3, requirement 3)
- [ ] All tests pass
- [ ] Validator sign-off (PASS verdict)
```

**Rule:** If DONE_MEANS doesn't map to spec, Validator rejects it.

### 5.5 BOOTSTRAP Completeness [CX-606]

**Orchestrator MUST provide:**

1. **FILES_TO_OPEN (5-15 files minimum)**
   - Spec docs (SPEC_CURRENT.md, Master Spec section)
   - Architecture docs (ARCHITECTURE.md, relevant design docs)
   - Implementation files (files Coder will modify)
   - Related modules (dependencies, imports)

2. **SEARCH_TERMS (10-20 grep targets minimum)**
   - Key symbols to find (`SqlitePool`, `state.pool`)
   - Error messages to look for
   - Feature names to search
   - Pattern names (`DefaultStorageGuard`)

3. **RUN_COMMANDS (startup + validation)**
   - Dev environment startup (`just dev`)
   - Test commands (`cargo test`, `pnpm test`)
   - Validation commands (`just validate`, `just ai-review`)

4. **RISK_MAP (3-8 failure modes)**
   - Specific failure mode
   - Which subsystem breaks
   - Example: `"Hollow trait implementation" → Portability Failure (Phase 1 blocker)`

### 5.6 Work Packet Locking [CX-607]

**Orchestrator MUST lock packet after creation:**

```markdown
---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220250328

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-{ID}-variant), do NOT edit this one.**
```

**Rule of Locking:**
- ✅ Once locked, packet is immutable
- ✅ Prevents instruction creep mid-work
- ✅ Creates audit trail (version history)
- ❌ Cannot edit locked packet (violates governance)
- ❌ If changes needed, must create new packet

**When to create variant packets:**
- WP-1-Storage-Abstraction-Layer (original, locked)
- WP-1-Storage-Abstraction-Layer-v2 (changes needed, new packet)
- OR: WP-1-Storage-Abstraction-Layer-20251225-1630 (date/time variant)

---

## Part 6: Task Board Maintenance [CX-625-630]

### 6.1 Task Board Structure (Single Source of Truth)

**Orchestrator maintains `docs/TASK_BOARD.md` as the authoritative status tracker.**

```markdown
# Handshake Project Task Board

This board is maintained by the Orchestrator.
Updated whenever WP status changes.

---

## 🚨 PHASE 1 CLOSURE GATES (BLOCKING)

**Authority:** Master Spec §2.3.12, Architecture Decision {date}

Storage Backend Portability Foundation (Sequential):

1. **[WP-1-Storage-Abstraction-Layer]** - Define trait-based storage API
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 15-20 hours
   - Status: [READY FOR DEV 🔴]
   - Blocker: None (foundational)

2. **[WP-1-AppState-Refactoring]** - Remove SqlitePool from AppState
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 8-10 hours
   - Status: [GAP 🟡]
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)

---

## In Progress

- [WP_ID]: {Brief description}

## Ready for Dev

- [WP_ID]: {Brief description}

## Done

- [WP_ID]: {Brief description}

## Blocked

- [WP_ID]: {Reason for block}
```

### 6.2 Status Values (CX-625)

| Status | Symbol | Meaning | When to Use |
|--------|--------|---------|------------|
| **READY FOR DEV** | 🔴 | Verified, waiting for Coder | After pre-work checklist PASS |
| **IN PROGRESS** | 🟠 | Coder is working | After Coder outputs BOOTSTRAP |
| **BLOCKED** | 🟡 | Waiting for dependency/clarification | Document specific reason |
| **DONE** | ✅ | Merged to main | After Validator approves |
| **GAP** | 🟡 | Not yet created as packet | Before Orchestrator creates |

### 6.3 Orchestrator Responsibilities for TASK_BOARD

**Update TASK_BOARD IMMEDIATELY when:**
1. New WP created → Move to "Ready for Dev"
2. Coder starts work → Move to "In Progress"
3. Blocker discovered → Move to "Blocked" + document reason
4. Validator approves → Move to "Done"
5. Dependency unblocked → Move blocked WP to "Ready for Dev"

**Keep TASK_BOARD in sync with reality:**
```
Never let TASK_BOARD drift from actual WP status.
If WP file shows STATUS: In-Progress but TASK_BOARD shows Ready-for-Dev → FAIL.
Orchestrator must maintain consistency immediately.
```

### 6.4 Phase Gate Status Tracking [CX-609]

**Orchestrator must maintain Phase Gate section:**

```markdown
## 🚨 PHASE 1 CLOSURE GATES (BLOCKING - MUST COMPLETE)

**Status:** HOLDING - 3 of 4 gate-critical WPs not yet created

Gate-critical WPs:
1. ✅ WP-1-Storage-Abstraction-Layer [READY FOR DEV]
2. ❌ WP-1-AppState-Refactoring [GAP - packet not yet created]
3. ❌ WP-1-Migration-Framework [GAP - packet not yet created]
4. ❌ WP-1-Dual-Backend-Tests [GAP - packet not yet created]

Phase closure criteria:
- [ ] All 4 gate-critical WPs are VALIDATED (not just "done")
- [ ] Spec regression check PASS (just validator-spec-regression)
- [ ] All dependencies resolved
- [ ] Waivers audit complete
- [ ] Supply chain clean (cargo deny + npm audit)

Current status: 25% ready (1 of 4 packets created, 0 VALIDATED)
```

---

## Part 7: Dependency Management [CX-630-635]

### 7.1 Blocking Dependencies

**Orchestrator MUST identify and document all blocking relationships:**

**In work packets:**
```markdown
## Dependencies

- Depends on: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)
- Blocks: WP-1-Dual-Backend-Tests
- Can start independently: WP-1-Migration-Framework
```

**In TASK_BOARD:**
```markdown
2. **[WP-1-AppState-Refactoring]**
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)
```

### 7.2 Blocking Rules (MANDATORY)

**DO NOT assign WP if blocker is not VALIDATED:**

```
Scenario: WP-1-AppState-Refactoring depends on WP-1-Storage-Abstraction-Layer

If WP-1-Storage-Abstraction-Layer status is:
- ✅ VALIDATED → Can assign WP-1-AppState-Refactoring
- 🟠 IN PROGRESS → Mark WP-1-AppState-Refactoring as BLOCKED
- 🔴 READY FOR DEV → Mark WP-1-AppState-Refactoring as BLOCKED
- ❌ FAILS Validator → Don't assign, escalate

Rule: Never assign downstream work until blocker is VALIDATED.
```

**DO NOT close phase if blockers unresolved:**

```
Phase 1 closure requires:
- ALL 4 gate-critical WPs VALIDATED
- ALL dependencies satisfied
- NO unresolved blockers

If WP-1-Migration-Framework blocks WP-1-Dual-Backend-Tests:
→ Phase cannot close until BOTH are VALIDATED
```

**Document WHY WP is BLOCKED:**

```markdown
## Blocked

- WP-1-AppState-Refactoring: Waiting for WP-1-Storage-Abstraction-Layer to VALIDATE (ETA 3 days)
- WP-1-Dual-Backend-Tests: Blocked on 2 dependencies (WP-1-Storage-Abstraction-Layer, WP-1-Migration-Framework)
```

---

## Part 8: Pre-Delegation Validation Checklist [CX-640]

**Before handing off to Coder, Orchestrator MUST verify all 14 items:**

- [ ] SPEC_ANCHOR references Main Body (not Roadmap)
- [ ] SPEC_ANCHOR in latest Master Spec version
- [ ] IN_SCOPE_PATHS are exact file paths (not "src/backend")
- [ ] OUT_OF_SCOPE clearly lists what Coder cannot touch
- [ ] DONE_MEANS are measurable (100% verifiable, not subjective)
- [ ] Every DONE_MEANS maps 1:1 to SPEC_ANCHOR requirement
- [ ] RISK_TIER assigned (LOW/MEDIUM/HIGH)
- [ ] TEST_PLAN includes all applicable commands
- [ ] BOOTSTRAP has 5-15 FILES_TO_OPEN
- [ ] BOOTSTRAP has 10-20 SEARCH_TERMS
- [ ] BOOTSTRAP has RISK_MAP (3-8 failure modes)
- [ ] USER_SIGNATURE locked with date/timestamp
- [ ] Dependencies documented (blockers + what this blocks)
- [ ] Effort estimate provided (hours)

**If ANY check fails:** Reject WP; request Orchestrator fix specific gaps.

---

## Part 9: Orchestrator Non-Negotiables [CX-640-650]

### ❌ DO NOT:

1. **Create WP without SPEC_ANCHOR** — Every WP must reference Master Spec Main Body
2. **Edit locked work packets** — Once USER_SIGNATURE added, packet is immutable
3. **Use vague scope** — IN_SCOPE_PATHS must be specific file paths
4. **Assign WP with unresolved blocker** — Wait for blocker to VALIDATE first
5. **Close phase without all WPs VALIDATED** — "Done" ≠ "VALIDATED"
6. **Skip pre-orchestration checklist** — All 14 items must pass
7. **Invent requirements** — Task packets point to SPEC_ANCHOR, period
8. **Let TASK_BOARD drift** — Update immediately when WP status changes
9. **Lump multiple features in one WP** — One WP per requirement
10. **Leave dependencies undocumented** — TASK_BOARD must show all blocking relationships

### ✅ DO:

1. **Create one WP per Master Spec requirement** — No lumping
2. **Lock every packet with USER_SIGNATURE** — Prevents instruction creep
3. **Map every DONE_MEANS to SPEC_ANCHOR** — Traceability required
4. **Document dependencies explicitly** — TASK_BOARD shows blockers
5. **Maintain Phase Gate visibility** — Keep status current
6. **Run pre-orchestration checklist** — Verify spec, board, supply chain
7. **Update TASK_BOARD immediately** — Don't let status drift
8. **Provide complete BOOTSTRAP** — Coder needs 5-15 files, 10-20 terms, risk map
9. **Create variant packets for changes** — Never edit locked packets
10. **Enforce blocking rules** — Don't assign downstream work prematurely

---

## Part 10: Real Examples (Templates)

See actual work packets in `docs/task_packets/` for patterns:

- **WP-1-Storage-Abstraction-Layer.md** — High risk, foundational (trait-based design)
- **WP-1-AI-Integration-Baseline.md** — Medium risk, feature (LLM integration)
- **WP-1-Terminal-Integration-Baseline.md** — High risk, security-sensitive

All follow the structure in this protocol; use them as templates for new WPs.

---

**ORCHESTRATOR SUMMARY:**

| Responsibility | Primary Document | Authority |
|---|---|---|
| Create work packets | `docs/task_packets/WP-*.md` | ORCHESTRATOR_PROTOCOL Part 4-5 |
| Maintain task board | `docs/TASK_BOARD.md` | ORCHESTRATOR_PROTOCOL Part 6 |
| Track dependencies | Packet + TASK_BOARD | ORCHESTRATOR_PROTOCOL Part 7 |
| Validate before delegation | Pre-work checklist | ORCHESTRATOR_PROTOCOL Part 8 |
| Lock packets | USER_SIGNATURE | ORCHESTRATOR_PROTOCOL Part 5.6 |
| Update status immediately | TASK_BOARD sync | ORCHESTRATOR_PROTOCOL Part 6.3 |
| Enforce phase gates | PHASE 1 CLOSURE GATES | ORCHESTRATOR_PROTOCOL Part 6.4 |
| Manage blockers | Dependency tracking | ORCHESTRATOR_PROTOCOL Part 7 |

**Orchestrator role = Precise work packets + Updated TASK_BOARD + Locked packets + Verified pre-work + Enforced dependencies + Phase gate management**
