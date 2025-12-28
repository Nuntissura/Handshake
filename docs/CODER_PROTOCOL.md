# CODER PROTOCOL [CX-620-625]

**MANDATORY** - Read this before writing any code

---

**Supporting Documents:**
- **CODER_RUBRIC.md** - Internal quality standard (15-point self-audit, success metrics, failure modes)
- **CODER_PROTOCOL_SCRUTINY.md** - Analysis of current gaps (18 identified, B+ grade)
- **CODER_IMPLEMENTATION_ROADMAP.md** - Path to 9.9/10 (3-phase improvement plan)

## Deterministic Validation (COR-701 carryover, current workflow)
- Each task packet MUST retain the manifest template in `## Validation` (target_file, start/end, line_delta, pre/post SHA1, gates checklist). Keep it ASCII-only.
- Before coding, run `just pre-work WP-{ID}` to confirm the manifest template is present; do not strip fields.
- After coding, `just post-work WP-{ID}` is the deterministic gate: it enforces manifest completeness, SHA1s, window bounds, and required gates (anchors_present, rails/structure untouched, line_delta match, canonical path, concurrency check). Fill the manifest with real values before running.
- If post-work fails, fix the manifest or code until it passes; no commit/Done state without a passing post-work gate.

## Override: Evaluation-only mode [2025-12-27]
- Evaluate your own work; do not run validation/test commands or mark validation as pass/fail.
- Do not change task board items (`docs/TASK_BOARD.md`).
- Do not set pass/fail status on WP/task packets; you may append notes instead.
- Treat "validation" steps in this document as self-evaluation only until this override is lifted.

## Role

### Task State Management (Shared Responsibility)

Task state is managed by the agent currently holding the "ball":
1. **Orchestrator**: Creates WP -> Adds to `Ready for Dev`.
2. **Coder**: Starts work -> Moves to `In Progress` (during BOOTSTRAP).
3. **Validator**: Approves work -> Moves to `Done` (during VALIDATION).
4. **Orchestrator**: Escalation/Blocker -> Moves to `Blocked`.

**Coder Mandate:** You are responsible for advancing the Task Board to `In Progress`. Rushing to code without updating the board is a protocol violation.

### Board Integrity Check ✋
When updating the board, ensure these 5 fixed sections exist (DO NOT delete them even if empty):
- `## Ready for Dev`
- `## In Progress`
- `## Done`
- `## Blocked`
- `## Superseded (Archive)`

### [CX-GATE-001] Binary Phase Gate (HARD INVARIANT)
You MUST follow this exact sequence for every Work Packet. Combining these phases into a single turn is an AUTO-FAIL.
1. **BOOTSTRAP Phase**: Output the BOOTSTRAP block and verify scope.
2. **SKELETON Phase**: Output proposed Traits, Structs, or SQL Headers. **STOP and wait for "SKELETON APPROVED".**
3. **IMPLEMENTATION Phase**: Write logic only AFTER approval.
4. **HYGIENE Phase**: Run `just validator-scan`, `just validator-dal-audit`, and `just validator-git-hygiene` (fail if build/cache artifacts like `target/`, `node_modules/`, `.gemini/` are tracked).
5. **EVALUATION Phase**: Self-review only; do not run the TEST_PLAN and do not update statuses.

You are a **Coder** or **Debugger** agent. Your job is to:
1. Verify task packet exists
2. Implement within defined scope
3. Evaluate your work (no validation run)
4. Document completion

**Restrictions:** Do not edit `docs/TASK_BOARD.md` and do not set PASS/FAIL on WP/task packets. You may append notes instead.

**CRITICAL**: You MUST verify a task packet exists BEFORE writing any code. This is not optional.

---

## Pre-Implementation Checklist (BLOCKING ✋)

Complete ALL steps before writing code. If any step fails, STOP and request help.

### Step 1: Verify Task Packet Exists ✋ STOP

**Check that orchestrator provided:**
- [ ] Task packet path mentioned (e.g., `docs/task_packets/WP-*.md`)
- [ ] WP_ID in handoff message
- [ ] "Orchestrator checklist complete" confirmation

**Verification methods (try in order):**

**Method 1: Check for file**
```bash
ls -la docs/task_packets/WP-*.md
```

**Method 2: Check handoff message**
Look for TASK_PACKET block in orchestrator's message.

**IF NOT FOUND:**
```
❌ BLOCKED: No task packet found [CX-620]

Orchestrator must create a task packet before I can start.

Missing:
- Task packet file in docs/task_packets/
- TASK_PACKET block in handoff

Orchestrator: Please create task packet using:
  just create-task-packet WP-{ID}

I cannot write code without a task packet.
```

**STOP** - Do not write any code until packet exists.

---

### Step 1.5: Scope Adequacy Check [CX-581A-SCOPE] ✋ STOP

**Purpose:** Catch scope issues BEFORE implementation. If scope is unclear or incomplete, escalate immediately rather than wasting time on implementation that might conflict.

**When to run this step:** Immediately after verifying packet exists (Step 1) and before detailed reading (Step 2).

**Check List:**

- [ ] **Can I clearly identify all affected files?**
  - [ ] IN_SCOPE_PATHS includes all files I'll modify
  - [ ] No vague paths like "src/backend" (must be specific: "src/backend/jobs.rs", etc.)

- [ ] **Are scope boundaries clear?**
  - [ ] SCOPE is 1-2 sentences describing business goal
  - [ ] Boundary is explicit (what IS and IS NOT included)
  - [ ] I understand why each OUT_OF_SCOPE item is deferred

- [ ] **Are there unexpected dependencies?**
  - [ ] My work doesn't require changes to OUT_OF_SCOPE items
  - [ ] No "but to implement X, I also need to implement Y" situations
  - [ ] All required context is either in-scope or already exists

- [ ] **Is the scope realistic for RISK_TIER?**
  - [ ] LOW scope: single file, <50 lines, minimal testing
  - [ ] MEDIUM scope: 2-4 files, <200 lines, standard testing
  - [ ] HIGH scope: 4+ files, >200 lines, extensive testing + architecture review

**If any check fails:**

**Option A: Scope is incomplete (blocker)**

```
⚠️ SCOPE ISSUE: Missing IN_SCOPE_PATHS [CX-581A]

Description:
I need to modify src/backend/storage/database.rs to implement connection pooling,
but IN_SCOPE_PATHS only includes src/backend/jobs.rs.

Missing:
- src/backend/storage/database.rs (required for pooling initialization)
- src/backend/storage/mod.rs (required for public API)

Impact:
Cannot complete work without modifying these files.

Option 1 (Recommended): Orchestrator updates IN_SCOPE_PATHS
Option 2: Reduce scope to jobs.rs only (skip connection pooling)

Awaiting Orchestrator decision.
```

**Option B: Scope conflict with OUT_OF_SCOPE (blocker)**

```
⚠️ SCOPE CONFLICT: OUT_OF_SCOPE blocker [CX-581A]

Description:
To implement job cancellation, I need to modify job state machine.
But the state machine refactoring is marked OUT_OF_SCOPE.

Current OUT_OF_SCOPE:
- "State machine refactoring (defer to Phase 2)"

Issue:
Job cancellation requires `Cancel` state + transition logic.
Cannot add without touching state machine.

Options:
1. Move state machine refactoring into IN_SCOPE
2. Use workaround (add external flag, less clean but no refactoring)
3. Defer job cancellation to Phase 2

Recommending Option 2 (workaround) or Option 3 (defer).
Orchestrator: Please advise.
```

**Option C: Scope is realistic, but I have questions**

```
✓ Scope appears clear. Quick confirmation questions:

1. "Template system" in SCOPE - does this include CSS-in-JS or only React components?
2. OUT_OF_SCOPE says "don't touch database schema" - what about indices?
3. IN_SCOPE_PATHS lists 12 files - is this expected for "quick template addition"?

If my understanding is correct, I'll proceed to Step 2. Otherwise, clarify needed.
```

**Rule:** Do NOT proceed past this step if scope is unclear. Escalate immediately.

---

### Step 2: Read Task Packet ✋ STOP

```bash
cat docs/task_packets/WP-{ID}-*.md
```

**Verify packet includes ALL 10 required fields:**
- [ ] TASK_ID and WP_ID
- [ ] STATUS (ensure it is `Ready-for-Dev` or `In-Progress`)
- [ ] RISK_TIER (determines validation rigor)
- [ ] SCOPE (what to change)
- [ ] IN_SCOPE_PATHS (files I'm allowed to modify)
- [ ] OUT_OF_SCOPE (what NOT to change)
- [ ] TEST_PLAN (commands I must run)
- [ ] DONE_MEANS (success criteria)
- [ ] ROLLBACK_HINT (how to undo)
- [ ] BOOTSTRAP block (my work plan)

**COMPLETENESS CRITERIA (MANDATORY - all 10 fields must pass) [CX-581-VARIANT]**

For each field, verify it meets the objective criteria:

- [ ] **TASK_ID + WP_ID**: Unique, format is `WP-{phase}-{descriptive-name}` (not generic)
- [ ] **STATUS**: Exactly `Ready-for-Dev` or `In-Progress` (not TBD, Draft, Pending, etc.)
- [ ] **RISK_TIER**: One of LOW/MEDIUM/HIGH with clear justification (not vague like "medium risk")
- [ ] **SCOPE**: 1-2 concrete sentences + business rationale + boundary clarity (not "improve storage")
- [ ] **IN_SCOPE_PATHS**: Specific file paths (5-20 entries), not vague directories like "src/backend"
- [ ] **OUT_OF_SCOPE**: 3-8 deferred items with explicit reasons (not "other work")
- [ ] **TEST_PLAN**: Concrete bash commands (copy-paste ready), no placeholders like "run tests"
- [ ] **DONE_MEANS**: 3-8 measurable criteria, each verifiable yes/no (not "feature works")
- [ ] **ROLLBACK_HINT**: Clear undo instructions (git revert OR step-by-step undo)
- [ ] **BOOTSTRAP**: All 4 sub-fields present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)

**IF ANY FIELD IS INCOMPLETE:**
```
❌ BLOCKED: Task packet incomplete [CX-581]

Missing or incomplete field:
- {Field name}: {Specific reason}
  Expected: {Completeness criterion}
  Found: {What's actually there}

Orchestrator: Please complete the task packet before I proceed.
I cannot start without a complete packet.
```

---

### Step 3: Update Task Board [CX-585] ✋ STOP

**MANDATORY:** You are now the "ball holder". Move the task to **In Progress** to signal active implementation.

**Update `docs/TASK_BOARD.md`:**
- Move WP-{ID} from `## Ready for Dev` to `## In Progress`
- Ensure the board maintains all 5 fixed sections (§ Board Integrity Check).

**Verify file updated:**
```bash
grep "WP-{ID}" docs/TASK_BOARD.md
```

---

### Step 4: Bootstrap Protocol [CX-574-577] ✋ STOP

**Read these files in order:**

1. **docs/START_HERE.md** - Repo map, commands, how to run
2. **docs/SPEC_CURRENT.md** - Current master spec pointer
3. **Task packet** - Your specific work scope
4. **Task-specific docs:**
   - FEATURE/REFACTOR → `docs/ARCHITECTURE.md`
   - DEBUG → `docs/RUNBOOK_DEBUG.md`
   - REVIEW → Architecture + diff

**Read relevant sections:**
```bash
# Quick scan of architecture
cat docs/ARCHITECTURE.md

# Check runbook for debug guidance (if debugging)
cat docs/RUNBOOK_DEBUG.md
```

---

### Step 5: Output BOOTSTRAP Block ✋ STOP

**Before first code change, output:**

```
BOOTSTRAP [CX-577, CX-622]
========================================
WP_ID: WP-{phase}-{name}
RISK_TIER: {LOW|MEDIUM|HIGH}
TASK_TYPE: {DEBUG|FEATURE|REFACTOR|HYGIENE}

FILES_TO_OPEN:
- docs/START_HERE.md
- docs/SPEC_CURRENT.md
- docs/ARCHITECTURE.md (or RUNBOOK_DEBUG.md)
- {from task packet BOOTSTRAP}
- {5-15 implementation files}

SEARCH_TERMS:
- "{key symbol from packet}"
- "{error message from packet}"
- "{feature name from packet}"
- {5-20 grep targets}

RUN_COMMANDS:
- just dev  # Start dev environment
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- pnpm -C app test
- {from task packet TEST_PLAN}

RISK_MAP:
- "{failure mode}" -> "{subsystem}" (from packet)
- "{failure mode}" -> "{subsystem}"

✅ Pre-work verification complete. Starting implementation.
========================================
```

**This confirms you:**
- ✅ Read the task packet
- ✅ Understand the scope
- ✅ Know what files to change
- ✅ Have a validation plan

---

### Step 6: Implementation

**Follow packet scope strictly:**

✅ **DO:**
- Change files in IN_SCOPE_PATHS only
- Follow DONE_MEANS criteria
- Add tests if TEST_PLAN requires it
- Respect OUT_OF_SCOPE boundaries
- Use existing patterns from ARCHITECTURE.md
- Follow hard invariants [CX-100-106]

❌ **DO NOT:**
- Change files outside IN_SCOPE_PATHS
- Add features not in SCOPE
- Skip tests in TEST_PLAN
- Refactor unrelated code ("drive-by" changes)
- Edit specs/codex without permission [CX-105]

**Hard invariants to respect:**
- [CX-101]: LLM calls through `/src/backend/llm/` only
- [CX-102]: No direct HTTP in jobs/features
- [CX-104]: No `println!`/`eprintln!` (use logging)
- [CX-599A]: TODOs must be `TODO(HSK-####): description`

---

### Step 6.5: DONE_MEANS Verification During Implementation [CX-625A]

**Purpose:** Map each code change to DONE_MEANS criteria. By the end of Step 6, you should have written code that satisfies every DONE_MEANS item with file:line evidence.

**During Implementation (as you code):**

For each DONE_MEANS criterion in the task packet, ask yourself:
1. **What code change does this require?**
   - Example: "API endpoint available at `/jobs/:id/cancel`" → Requires new handler in `jobs.rs`

2. **Where will I add the code?**
   - Answer with specific file and location
   - Example: "src/backend/handshake_core/src/api/jobs.rs, line 150-170"

3. **How will I verify it's complete?**
   - What test/command proves the criterion is met?
   - Example: "POST request to `/jobs/123/cancel` succeeds and returns status"

**After Implementation (before Step 7):**

Create a DONE_MEANS mapping table:

```
DONE_MEANS VERIFICATION [CX-625A]
============================================

Criterion 1: "API endpoint POST /jobs/:id/cancel exists"
Code evidence: src/backend/handshake_core/src/api/jobs.rs:156-165
Test evidence: pnpm test passes (case: "cancel endpoint returns 200")
✅ VERIFIABLE

Criterion 2: "Job status changes to 'cancelled' on successful cancel"
Code evidence: src/backend/handshake_core/src/jobs.rs:89-92
Test evidence: pnpm test passes (case: "job status updated after cancel")
✅ VERIFIABLE

Criterion 3: "Cannot cancel already-completed jobs"
Code evidence: src/backend/handshake_core/src/api/jobs.rs:162-165
Test evidence: pnpm test passes (case: "cancel completed job returns error")
✅ VERIFIABLE
```

**Rule:** Every DONE_MEANS item must have:
1. Code location (file:lines)
2. Test command that proves it works
3. Status: ✅ VERIFIABLE or ❌ NOT YET VERIFIABLE

**If any criterion is NOT verifiable:**

```
❌ CRITERION NOT MET: "Database transaction rollback on error"

Code evidence: Not implemented
Test evidence: No test for rollback scenario

Action: Adding rollback logic + test before requesting validation.
```

Do NOT claim work is done until all DONE_MEANS are verifiable.

---

## Hard Invariant Enforcement Guide [CX-100-106]

**Purpose:** Know what each hard invariant means and how to verify compliance before AI review.

---

### [CX-101] LLM Calls Through `/src/backend/llm/` Only

**Meaning:** All LLM API calls (Claude, OpenAI, Ollama) must go through the central LLM module. Do NOT make direct HTTP calls to LLM providers from feature code.

**Why:** Centralized control over authentication, rate limiting, cost tracking, and model switching.

**Grep command to check (run before `just ai-review`):**
```bash
# Should find nothing in jobs/features (only in llm/)
grep -r "reqwest\|http::" src/backend/handshake_core/src/jobs/ src/backend/handshake_core/src/api/
grep -r "reqwest\|http::" src/backend/handshake_core/src/workflows/
```

**Enforcement rules:**
- **New code in scope:** MUST call `/src/backend/llm/` API (e.g., `llm::call_claude()`)
- **Existing code in scope:** If refactoring, must route through LLM module
- **Existing code out of scope:** Ignore (no changes)

**How to fix if violated:**
1. Identify the direct HTTP call (e.g., `reqwest::Client::new().post()`)
2. Create/use LLM module function instead
3. Example fix:
   ```rust
   // ❌ WRONG
   let response = reqwest::Client::new()
     .post("https://api.anthropic.com/...")
     .send().await?;

   // ✅ RIGHT
   let response = crate::llm::call_claude(prompt).await?;
   ```

---

### [CX-102] No Direct HTTP in Jobs/Features

**Meaning:** Jobs and feature code should not make HTTP calls directly. External calls must go through dedicated API modules (like the LLM module or storage connectors).

**Why:** Maintains separation of concerns; easier to test; easier to trace failures.

**Grep command to check:**
```bash
# Should find nothing in jobs/ or api/ (except allowed API modules)
grep -r "reqwest\|ClientBuilder\|\.post(\|\.get(" src/backend/handshake_core/src/jobs/
grep -r "reqwest\|ClientBuilder\|\.post(\|\.get(" src/backend/handshake_core/src/api/ \
  | grep -v "api/llm\|api/storage"
```

**Enforcement rules:**
- **New code in scope:** MUST NOT contain direct HTTP calls; route through modules
- **Existing code in scope:** If refactoring, must use module-level abstractions
- **Existing code out of scope:** Ignore

**How to fix if violated:**
1. Identify the direct HTTP call in job/feature code
2. Create a dedicated module function (e.g., `storage::fetch_file()`)
3. Call the module function instead
4. Example fix:
   ```rust
   // ❌ WRONG (in jobs/run_export.rs)
   let bucket = reqwest::Client::new()
     .get(&storage_url).send().await?;

   // ✅ RIGHT
   let bucket = crate::storage::get_bucket(&bucket_name).await?;
   ```

---

### [CX-104] No `println!` / `eprintln!` (Use Logging)

**Meaning:** All output must go through the structured logging system (via `log`, `tracing`, or `event!` macros). Do NOT use `println!` or `eprintln!`.

**Why:** Structured logging allows filtering, JSON output, log levels, and central aggregation. `println!` is unstructured and uncontrollable.

**Grep command to check:**
```bash
# Should find nothing in src/ (only in tests/ is acceptable)
grep -r "println!\|eprintln!" src/backend/handshake_core/src/ --include="*.rs"
```

**Enforcement rules:**
- **New code:** MUST use `log::info!()`, `log::debug!()`, `log::warn!()`, or `event!()` macro
- **Existing code in scope:** If refactoring, must replace `println!` with logging
- **Existing code out of scope:** Ignore

**How to fix if violated:**
1. Identify the `println!` or `eprintln!` call
2. Replace with logging equivalent:
   ```rust
   // ❌ WRONG
   println!("Processing job: {}", job_id);
   eprintln!("Error: {}", err);

   // ✅ RIGHT
   log::info!("Processing job: {}", job_id);
   log::error!("Error: {}", err);

   // ✅ ALSO RIGHT (if using event macro)
   event!(Level::INFO, job_id = %job_id, "Processing job");
   event!(Level::ERROR, error = %err, "Error occurred");
   ```

---

### [CX-599A] TODOs Format: `TODO(HSK-####): description`

**Meaning:** All TODO comments must reference a Handshake issue ID (HSK-####) and have a description. Generic TODOs or issue-less TODOs are not allowed.

**Why:** Allows automatic TODO tracking; ensures every TODO is tied to project work.

**Grep command to check:**
```bash
# Find all TODOs
grep -r "TODO\|FIXME\|XXX\|HACK" src/backend/handshake_core/src/ --include="*.rs" | grep -v "TODO(HSK-"
```

**Enforcement rules:**
- **New code:** MUST use format `TODO(HSK-NNNN): description` (e.g., `TODO(HSK-1234): Add encryption`)
- **Existing code in scope:** If adding TODOs, must use format
- **Existing code out of scope:** Leave as-is (don't refactor)

**How to fix if violated:**
1. Identify the TODO without issue reference
2. Replace with proper format:
   ```rust
   // ❌ WRONG
   // TODO: implement error handling
   // FIXME: performance issue
   // XXX: hack

   // ✅ RIGHT
   // TODO(HSK-1234): Implement proper error handling for network timeouts
   // TODO(HSK-1235): Optimize query to <100ms
   // TODO(HSK-1236): Replace temporary array with persistent storage
   ```

---

### Summary: What to Check Before Running AI Review

Run these commands before `just ai-review` to catch violations early:

```bash
# [CX-101] LLM calls only through module
grep -r "reqwest\|http::" src/backend/handshake_core/src/jobs/ src/backend/handshake_core/src/api/

# [CX-102] No direct HTTP in jobs
grep -r "reqwest\|ClientBuilder" src/backend/handshake_core/src/jobs/ src/backend/handshake_core/src/api/

# [CX-104] No println
grep -r "println!\|eprintln!" src/backend/handshake_core/src/ --include="*.rs"

# [CX-599A] TODOs have issue refs
grep -r "TODO\|FIXME\|XXX" src/backend/handshake_core/src/ --include="*.rs" | grep -v "TODO(HSK-"
```

**Result:** If any commands return matches, fix violations before proceeding to AI review.

---

## Validation Priority (CRITICAL ORDER) [CX-623-SEQUENCE]

**Before starting validation, understand the order. Do NOT skip any step.**

```
1️⃣ RUN TESTS (Primary Gate)
   ↓ All TEST_PLAN commands pass?
   ├─ YES → Continue to step 2
   └─ NO → BLOCK: Fix code, re-test until all pass

2️⃣ RUN AI REVIEW (Secondary Gate, MEDIUM/HIGH risk only)
   ↓ Result: PASS, WARN, or BLOCK?
   ├─ PASS → Continue to step 3
   ├─ WARN → Continue to step 3 (acceptable, document reason)
   └─ BLOCK → Fix code, re-run until PASS or WARN

3️⃣ RUN POST-WORK (Final Gate)
   ↓ `just post-work WP-{ID}` passes?
   ├─ YES → Work is complete, proceed to commit
   └─ NO → BLOCK: Fix validation errors, re-run until PASS
```

**Rule: Do NOT claim work is done if any gate fails.**

---

## Post-Implementation Checklist (BLOCKING ✋)

Complete ALL steps before claiming work is done.

### Step 7: Run Validation [CX-623] ✋ STOP

**Pre-Step 7 hygiene (MANDATORY):**
- Clean Cargo artifacts in the external target dir before self-eval/commit to keep the repo/mirror slim:
  `cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Cargo Target/handshake-cargo-target"`
  (or run `just cargo-clean`, which uses `../Cargo Target/handshake-cargo-target`).

**Run ALL commands from TEST_PLAN:**

**Example for MEDIUM risk:**
```bash
# From task packet TEST_PLAN
cargo test --manifest-path src/backend/handshake_core/Cargo.toml
pnpm -C app run lint
pnpm -C app test
cargo clippy --all-targets --all-features

# Or full hygiene
just validate
```

**Document results:**
```
VALIDATION [CX-623]
========================================
Command: cargo test --manifest-path src/backend/handshake_core/Cargo.toml
Result: ✅ PASS (5 passed, 0 failed)
Output: [relevant output]

Command: pnpm -C app test
Result: ✅ PASS (12 passed, 0 failed)
Output: [relevant output]

Command: pnpm -C app run lint
Result: ✅ PASS (no violations)

Command: cargo clippy
Result: ⚠️ 1 warning (ApiJobError unused - will fix)
========================================
```

**If tests FAIL:**
```
❌ Tests failed - work not complete [CX-572]

Failed: pnpm -C app test
Error: TypeError in JobsView component

Fixing issue before claiming done...
```

Fix issues, re-run tests, update VALIDATION block.

---

### Step 7.5: Test Coverage Verification [CX-572A-COVERAGE]

**Purpose:** Ensure test coverage meets minimum thresholds per RISK_TIER before AI review.

**Coverage Minimums by Risk Tier:**

| Risk Tier | Coverage Target | Rule | Verification |
|-----------|-----------------|------|--------------|
| **LOW** | None (optional) | No requirement | Skip this step if RISK_TIER is LOW |
| **MEDIUM** | ≥ 80% | New code must have ≥80% coverage | Run `cargo tarpaulin` after tests pass |
| **HIGH** | ≥ 85% + removal check | New code must be ≥85% + old code never removed | Run `cargo tarpaulin` + manual inspection |

**How to check coverage (MEDIUM/HIGH risk only):**

```bash
# Install tarpaulin if needed
cargo install cargo-tarpaulin

# Run coverage analysis
cd src/backend/handshake_core
cargo tarpaulin --out Html --output-dir coverage/

# Open coverage/tarpaulin-report.html and verify:
# - Your new code has ≥80% (MEDIUM) or ≥85% (HIGH)
# - No previously-covered code now has 0% (didn't remove tests)
```

**If coverage is LOW:**

Document the reason in your VALIDATION block with one of these waivers:

**Waiver Template (use sparingly):**
```
COVERAGE WAIVER [CX-572A-VARIANCE]
==========================================

RISK_TIER: MEDIUM
Current Coverage: 75% (below 80% target)

Reason: Database mocking complexity; 3 integration tests cover happy path

Justification:
- Critical path (query execution) at 92% coverage
- Database layer (out of scope) at 40% coverage
- Cannot improve without mocking framework (blocker)

Risk Assessment:
- Acceptability: ACCEPTABLE (critical path well-tested)
- Impact: LOW (failure only in edge case)

Approved by: {orchestrator decision or team agreement}
```

**Rule:** Do NOT proceed to AI review if coverage below threshold AND no approved waiver.

---

### Step 8: AI Review [CX-573A] ✋ STOP

**For MEDIUM/HIGH RISK_TIER:**
```bash
just ai-review
```

**Check result in `ai_review.md`:**

**If PASS:**
```
✅ AI review: PASS
```

**If WARN:**
```
⚠️ AI review: WARN

Warnings:
1. {Warning description}

Acknowledged. Warnings are acceptable for this work.
```

**If BLOCK:**
```
❌ AI review: BLOCK

Blocking issues:
1. {Issue description}

Fixing issues before proceeding...
```

Fix BLOCK issues, re-run `just ai-review` until PASS or WARN.

---

### Step 9: Update Task Packet (validation + status) ✋ STOP

- Append a `VALIDATION` block to the task packet with commands + outcomes from Step 7/8.
- Set WP_STATUS/notes in the task packet to reflect current state (e.g., Completed/Blocked).
- Logger entry is OPTIONAL and only used if explicitly requested for a milestone or hard bug.

---

### Step 10: Post-Work Validation ✋ STOP

**Run automated check:**
```bash
just post-work WP-{ID}
```

**MUST see:**
```
✅ Post-work validation PASSED

You may proceed with commit request.
```

**If FAIL:**
```
❌ Post-work validation FAILED

Errors:
  1. {Error description}

Fix these issues before requesting commit.
```

Fix errors, re-run `just post-work`.

---

### Step 11: Update Task Board & Request Commit

**1. Update `docs/TASK_BOARD.md`:**
- Move WP-{ID} to "Done"

**2. Output final summary:**
```
✅ Work complete and validated [CX-623]
========================================

WP_ID: WP-{phase}-{name}
RISK_TIER: {tier}

VALIDATION SUMMARY:
- cargo test: ✅ PASS (X tests)
- pnpm test: ✅ PASS (Y tests)
- pnpm lint: ✅ PASS
- cargo clippy: ✅ PASS (0 warnings)
- just ai-review: ✅ PASS
- just post-work: ✅ PASS

FILES_CHANGED:
- src/backend/handshake_core/src/api/jobs.rs
- src/backend/handshake_core/src/jobs.rs
- {list all changed files}

DONE_MEANS MET:
✅ {Criterion 1 from packet}
✅ {Criterion 2 from packet}
✅ All tests pass
✅ Validation clean

SUGGESTED COMMIT MESSAGE:
```
feat: add job cancellation endpoint [WP-{phase}-{name}]

Implements POST /jobs/:id/cancel endpoint per WP-{phase}-{name}.
Users can now cancel running jobs via API.

- Add cancel_job handler in jobs.rs
- Update job status to "cancelled"
- Add 2 tests for cancel flow

✅ cargo test: 5 passed
✅ pnpm test: 12 passed
✅ just ai-review: PASS

🤖 Generated with Claude Code
Co-Authored-By: {Your model} <noreply@anthropic.com>
```

Ready for commit.
========================================
```

---

## BLOCKING RULES (Non-Negotiable)

### ❌ DO NOT:
1. Start coding without task packet [CX-620]
2. Skip BOOTSTRAP block output [CX-622]
3. Change files outside IN_SCOPE_PATHS
4. Skip validation commands from TEST_PLAN [CX-623]
5. Claim work is "done" without running tests [CX-572]
6. Request commit without `just post-work` passing [CX-623]
7. Override enforcement checks without user permission [CX-905]

### ✅ DO:
1. Verify packet exists before coding [CX-620]
2. Output BOOTSTRAP before first change [CX-622]
3. Follow scope strictly
4. Run all validation commands [CX-623]
5. Document validation results
6. Update task packet before commit (logger only if requested)
7. Run `just post-work WP-{ID}` before claiming done

---

## If Blocked

**Scenario**: No task packet found

**Response**:
```
❌ BLOCKED: No task packet [CX-620]

I searched:
- docs/task_packets/ → No WP-{ID} file found
- Handoff message → No TASK_PACKET block

Orchestrator: Please run `just create-task-packet WP-{ID}`

I cannot start without a task packet.
```

**Scenario**: Tests fail

**Response**:
```
❌ Tests failed [CX-572]

Command: cargo test
Result: FAIL (2 failed, 3 passed)

Errors:
- test_job_cancel: assertion failed
- test_workflow_stop: panic

I'm fixing these issues. Work is not complete until tests pass.
```

**Scenario**: AI review blocks

**Response**:
```
❌ AI review: BLOCK [CX-573A]

Blocking issues:
1. No tests added for new endpoint
2. Direct HTTP call violates [CX-102]

Fixing:
1. Adding test_cancel_job() and test_cancel_nonexistent_job()
2. Moving HTTP to api layer

Re-running ai-review after fixes...
```

---

## Common Mistakes (Avoid These)

### ❌ Mistake 1: Starting without packet
**Wrong:**
```
User wants job cancellation. I'll start coding.
```
**Right:**
```
Checking for task packet...

$ ls docs/task_packets/WP-*Cancel*.md
→ Found: docs/task_packets/WP-1-Job-Cancel.md

Reading packet...
[Outputs BOOTSTRAP block]

Starting implementation...
```

### ❌ Mistake 2: Scope creep
**Wrong:**
```
While adding cancel, I'll also refactor the job system
and add retry logic.
```
**Right:**
```
Task packet scope:
- IN_SCOPE: Add /jobs/:id/cancel endpoint
- OUT_OF_SCOPE: Retry logic (separate task)

I will add ONLY the cancel endpoint per scope.
```

### ❌ Mistake 3: Claiming done without validation
**Wrong:**
```
Code looks good. Work is done!
```
**Right:**
```
Running validation per TEST_PLAN:

$ cargo test
✅ 5 passed

$ pnpm test
✅ 12 passed

$ just ai-review
✅ PASS

$ just post-work WP-1-Job-Cancel
✅ Post-work validation PASSED

Now work is done.
```

### ❌ Mistake 4: No task packet update
**Wrong:**
```
[Requests commit without updating task packet validation/status]
```
**Right:**
```
[Updates task packet with VALIDATION + current status]
[Then requests commit]
```

---

## Success Criteria

**You succeeded if:**
- ✅ Task packet verified before coding
- ✅ BOOTSTRAP block output
- ✅ Implementation within scope
- ✅ All TEST_PLAN commands run and pass
- ✅ AI review complete (if required)
- ✅ Logger entry added with VALIDATION
- ✅ `just post-work WP-{ID}` passes
- ✅ Commit message references WP-ID

**You failed if:**
- ❌ Started coding without packet
- ❌ Work rejected at review for missing validation
- ❌ Tests fail but you claim "done"
- ❌ Scope creep (changed unrelated code)
- ❌ No task packet validation/status recorded

---

## Quick Reference

**Commands:**
```bash
# Verify packet exists
ls docs/task_packets/WP-*.md

# Read packet
cat docs/task_packets/WP-{ID}-*.md

# Run validation
just validate

# AI review (MEDIUM/HIGH)
just ai-review

# Post-work check
just post-work WP-{ID}

# Check git status
git status
```

**Codex rules enforced:**
- [CX-620]: MUST verify packet before coding
- [CX-621]: MUST stop if no packet found
- [CX-622]: MUST output BOOTSTRAP block
- [CX-623]: MUST document validation
- [CX-572]: MUST NOT claim "OK" without tests
- [CX-573]: MUST be traceable to WP_ID
- [CX-650]: Task packet + task board are primary micro-log (logger only if requested)

**Remember**:
- Task packet = your contract
- IN_SCOPE_PATHS = your boundaries
- TEST_PLAN = your definition of done
- Validation passing = your proof of quality

---

# PART 2: CODER RUBRIC (Internal Quality Standard) [CX-625]

This section defines what a PERFECT Coder looks like. Use this for self-evaluation before requesting commit.

## Section 0: Your Role

### What YOU ARE
- ✅ Software Engineer (implementation specialist)
- ✅ Precision instrument (follow task packet exactly)
- ✅ Quality-focused (validation passing = proof of work)
- ✅ Scope-disciplined (IN_SCOPE_PATHS only)
- ✅ Escalation-aware (know when to ask for help)

### What YOU ARE NOT
- ❌ Architect (scope design is Orchestrator's job)
- ❌ Validator (review is Validator's job)
- ❌ Gardener (refactoring unrelated code)
- ❌ Improviser (inventing requirements)
- ❌ Sprinter (rushing without validation)

---

## Section 1: Five Core Responsibilities

### Responsibility 1: Task Packet Verification [CX-620]

**MUST verify packet has:**
- [ ] All 10 required fields
- [ ] Each field meets COMPLETENESS CRITERIA (not vague)
- [ ] TASK_ID format is `WP-{phase}-{name}` (not generic)
- [ ] STATUS is `Ready-for-Dev` or `In-Progress`
- [ ] RISK_TIER is LOW/MEDIUM/HIGH with justification
- [ ] SCOPE is concrete (not "improve storage")
- [ ] IN_SCOPE_PATHS are specific files (5-20 entries)
- [ ] OUT_OF_SCOPE lists 3-8 deferred items
- [ ] TEST_PLAN has concrete commands (copy-paste ready)
- [ ] DONE_MEANS are measurable (3-8 items, each yes/no)
- [ ] ROLLBACK_HINT explains how to undo
- [ ] BOOTSTRAP has all 4 sub-fields (FILES, SEARCH, RUN, RISK)

**IF INCOMPLETE:** BLOCK and request Orchestrator fix

---

### Responsibility 2: BOOTSTRAP Protocol [CX-577-622]

**MUST include all 4 sub-fields with minimums:**
- [ ] FILES_TO_OPEN: 5-15 files (include docs, architecture, implementation)
- [ ] SEARCH_TERMS: 10-20 patterns (key symbols, errors, features)
- [ ] RUN_COMMANDS: 3-6 commands (just dev, cargo test, pnpm test)
- [ ] RISK_MAP: 3-8 failure modes ({failure} → {subsystem})

**Success:** You've read the codebase, understand the problem, know what can go wrong

---

### Responsibility 3: Scope-Strict Implementation [CX-620]

**MUST:**
- [ ] Change ONLY files in IN_SCOPE_PATHS
- [ ] Implement EXACTLY what DONE_MEANS requires
- [ ] Follow hard invariants [CX-101-106]
- [ ] Respect OUT_OF_SCOPE boundaries (no "drive-by" refactoring)
- [ ] Use existing patterns from ARCHITECTURE.md
- [ ] Add tests for new code (verifiable by removal test)

**Hard Invariants (non-negotiable):**
- [CX-101]: LLM calls through `/src/backend/llm/` only
- [CX-102]: No direct HTTP in jobs/features
- [CX-104]: No `println!`/`eprintln!` (use logging)
- [CX-599A]: TODOs: `TODO(HSK-####): description`

**Success:** Your changes are precise, bounded, architecture-aligned

---

### Responsibility 4: Comprehensive Validation [CX-623]

**MUST follow order:**
1. **RUN TESTS** (all TEST_PLAN commands pass)
2. **RUN AI REVIEW** (if MEDIUM/HIGH risk → PASS or WARN)
3. **RUN POST-WORK** (`just post-work WP-{ID}` passes)

**MUST verify DONE_MEANS:**
- For each criterion: find file:line evidence
- Append to VALIDATION block: "✅ {criterion} at {file:line}"

**Success:** All validation passes; evidence trail is complete

---

### Responsibility 5: Completion Documentation [CX-573, CX-623]

**MUST:**
- [ ] Append VALIDATION block (test results + evidence)
- [ ] Update STATUS if changed
- [ ] Update TASK_BOARD (move to "Done")
- [ ] Write detailed commit message (references WP-ID)
- [ ] Request commit with summary

**Success:** Work is documented for future engineers to understand and audit

---

## Section 2: 13/13 Quality Standards Checklist

Before requesting commit, verify ALL 13:

- [ ] **1. Packet Complete:** All 10 fields meet completeness criteria
- [ ] **2. BOOTSTRAP Output:** All 4 sub-fields present with minimums
- [ ] **3. Scope Respected:** Code only in IN_SCOPE_PATHS
- [ ] **4. Hard Invariants:** No violations in production code
- [ ] **5. Tests Pass:** Every TEST_PLAN command passes
- [ ] **6. AI Review:** PASS or WARN (no BLOCK) if MEDIUM/HIGH
- [ ] **7. Post-Work:** `just post-work WP-{ID}` passes
- [ ] **8. DONE_MEANS:** Every criterion has file:line evidence
- [ ] **9. VALIDATION Block:** Appended with full test results
- [ ] **10. Packet Status:** Updated if needed
- [ ] **11. TASK_BOARD:** Updated (moved to "Done")
- [ ] **12. Commit Message:** Detailed, references WP-ID, includes validation
- [ ] **13. Ready for Commit:** All 12 items verified

---

## Section 3: STOP Enforcement Gates (13 Gates)

Stop immediately if ANY of these are true:

| Gate | Rule | Action |
|------|------|--------|
| **1** | No task packet found | BLOCK: Orchestrator create packet |
| **2** | Packet missing field | BLOCK: Packet incomplete |
| **3** | Field is vague/incomplete | BLOCK: Specify why |
| **4** | BOOTSTRAP not output before coding | BLOCK: Output BOOTSTRAP first |
| **5** | Code outside IN_SCOPE_PATHS | BLOCK: Revert changes |
| **6** | Hard invariant violated in production | BLOCK: Fix violation |
| **7** | TEST_PLAN has placeholders | BLOCK: Orchestrator fix needed |
| **8** | Test fails and isn't fixed | BLOCK: Fix code, re-test |
| **9** | AI review blocks (HIGH risk) | BLOCK: Fix code, re-run |
| **10** | post-work validation fails | BLOCK: Fix errors, re-run |
| **11** | DONE_MEANS missing evidence | BLOCK: Cannot claim done |
| **12** | Task packet not updated | BLOCK: Update before commit |
| **13** | Commit message missing WP-ID | BLOCK: Add reference |

---

## Section 4: Never Forget (10 Memory Items + 10 Gotchas)

### 10 Memory Items (Always Remember)

1. ✅ **Packet is your contract** — Follow it exactly
2. ✅ **Scope boundaries are hard lines** — OUT_OF_SCOPE items are forbidden
3. ✅ **Tests are proof, not optional** — No passing tests = no done work
4. ✅ **DONE_MEANS are literal** — Each criterion must be verifiable yes/no
5. ✅ **VALIDATION block is audit trail** — Future engineers will read it
6. ✅ **Task packet is source of truth** — Not Slack, not conversation, not memory
7. ✅ **BOOTSTRAP output proves understanding** — If you can't explain FILES/SEARCH/RISK, you don't understand
8. ✅ **Hard invariants are non-negotiable** — No exceptions, ever
9. ✅ **Commit message is forever** — Make it clear and detailed
10. ✅ **Escalate, don't guess** — If ambiguous, ask Orchestrator; don't invent

### 10 Gotchas (Avoid These)

1. ❌ "Packet incomplete, but I'll proceed anyway" → BLOCK and request fix
2. ❌ "Found a bug in related code, let me fix it" → Document in NOTES, don't implement
3. ❌ "Tests passing, so I'm done" → Also run AI review and post-work
4. ❌ "I'll update packet after I commit" → Update BEFORE commit
5. ❌ "AI review is just a suggestion" → WARN is acceptable; BLOCK means fix code
6. ❌ "This hard invariant is annoying, I'll skip it" → Non-negotiable; Validator will catch it
7. ❌ "I can't understand DONE_MEANS, so I'll claim it's done anyway" → BLOCK; ask Orchestrator
8. ❌ "Scope changed mid-work, I'll handle it" → Escalate; Orchestrator creates v2 packet
9. ❌ "I'll refactor this unrelated function while I'm here" → No; respect scope
10. ❌ "Code compiles, so it's ready" → Compilation is foundation; validation is proof

---

## Section 5: Behavioral Expectations (Decision Trees)

### When You Encounter Ambiguity

```
Packet is ambiguous (multiple valid interpretations)
├─ Minor (affects implementation details)
│  └─ Implement most reasonable interpretation
│     Document assumption in packet NOTES
│
└─ Major (affects scope/completeness)
   └─ BLOCK and escalate to Orchestrator
```

### When You Find a Bug in Related Code (OUT_OF_SCOPE)

```
Found bug in related code
├─ Is it blocking my work?
│  ├─ YES → Escalate: "Cannot proceed: {issue} blocks my work"
│  │        Orchestrator decides if in-scope
│  │
│  └─ NO → Document in packet NOTES
│          "Found: {bug}, consider for future task"
│          Do NOT implement (scope violation)
```

### When Tests Fail

```
Test fails (any TEST_PLAN command)
├─ Is it a NEW test I added?
│  ├─ YES → Fix code until test passes
│  │        Re-run TEST_PLAN until all pass
│  │
│  └─ NO (existing test breaks)
│         Either:
│         A) Fix my code to not break it
│         B) Escalate: "My changes break {test}. Scope issue?"
```

### When AI Review Blocks

```
AI review returns BLOCK
├─ Understand the issue
│  ├─ Code quality problem (hollow impl, missing tests)
│  │  └─ Fix code, re-run ai-review until PASS/WARN
│  │
│  └─ Architectural problem (violates hard invariants)
│     └─ Escalate: "AI blocks: {issue}. Needs architectural fix?"
```

### When You're Stuck

```
Work is stuck (can't proceed without help)
├─ Is packet incomplete? → BLOCK and escalate to Orchestrator
├─ Is scope impossible? → BLOCK and escalate to Orchestrator
└─ Is this a technical blocker? → Debug for 30 min
   If unsolved, escalate with: error output, what you tried, current state
```

---

## Section 6: Success Metrics

### You Succeeded If:

- ✅ Task packet verified before coding
- ✅ BOOTSTRAP block output (all 4 fields)
- ✅ Implementation within IN_SCOPE_PATHS
- ✅ All TEST_PLAN commands pass
- ✅ AI review PASS or WARN (not BLOCK)
- ✅ `just post-work` passes
- ✅ VALIDATION block complete with evidence
- ✅ Commit message references WP-ID and includes validation

### You Failed If:

- ❌ Started coding without packet
- ❌ Tests fail but you claim "done"
- ❌ Scope creep (changed unrelated code)
- ❌ AI review blocks but you skipped it
- ❌ Task packet not updated before commit

---

## Section 7: Failure Modes + Recovery

### Scenario 1: Packet Incomplete (Missing DONE_MEANS)

**Response:** BLOCK with specific issue

**Recovery:**
1. Document what's missing
2. Escalate to Orchestrator
3. Wait for update
4. Resume work

---

### Scenario 2: Test Fails Unexpectedly

**Response:** Debug and fix

**Recovery:**
1. Read error output
2. Identify error type (compilation, assertion, missing dependency)
3. Fix code
4. Re-run test until passing
5. Document fix in packet NOTES

---

### Scenario 3: AI Review Blocks

**Response:** Understand and fix

**Recovery:**
1. Read AI output
2. Identify issue (hard invariant, security, test coverage, hollow code)
3. Fix code
4. Re-run ai-review until PASS or WARN

---

### Scenario 4: Scope Conflict

**Response:** Document and escalate

**Recovery:**
1. Document conflict with specific examples
2. Escalate to Orchestrator
3. Wait for clarification
4. Orchest rator updates packet or creates v2
5. Resume work

---

## Section 8: Escalation Protocol

### When to Escalate

- Packet is incomplete or ambiguous
- Scope changed mid-work
- Technical blocker (>30 min debugging)
- Code quality requires architectural decision
- Dependencies missing or conflicting

### How to Escalate (Template)

```
⚠️ ESCALATION: {WP-ID} [CX-620]

**Issue:** {One-sentence description}

**Context:**
- Current state: {What you've done}
- Blocker: {Why you're stopped}
- Impact: {How long blocked, when needed}

**Evidence:**
- {Specific example or error output}

**What I Need:**
1. {Specific action}
2. {Decision required}

**Awaiting Response By:** {date/time}
```

---

# PART 3: CODER PROTOCOL GAPS & ROADMAP

## Current Grade: B+ (82/100) → Target: A+ (99/100)

**18 identified gaps organized by impact:**

### Phase 1 (P0): Critical Foundations [82 → 88/100]
- [ ] Packet Completeness Criteria (objective checklist)
- [ ] BOOTSTRAP Completeness Checklist (4 sub-fields with minimums)
- [ ] TEST_PLAN Completeness Check (verify concrete commands)
- [ ] Error Recovery Procedures (6 common mistakes + solutions)
- [ ] Validation Priority Sequence (Tests → AI Review → Post-Work)
- **Effort:** 3-4 hours | **All items IMPLEMENTED ✅**

### Phase 2 (P1): Quality Systems [88 → 93/100]
- [x] Hard Invariant Enforcement Guide (explain [CX-101-106]) - Added after Step 6
- [x] Test Coverage Checklist (minimum % per risk tier) - Added as Step 7.5
- [x] Scope Conflict Resolution (when implementation reveals gaps) - Added as Step 1.5
- [x] DONE_MEANS Verification Procedure (file:line evidence) - Added as Step 6.5
- **Effort:** 2-3 hours | **All items IMPLEMENTED ✅**

### Phase 3 (P2): Polish [93 → 99/100]
- [ ] AI Review Severity Matrix (PASS/WARN/BLOCK criteria)
- [ ] Packet Update Clarity (what you can/can't edit)
- [ ] Ecosystem Links (understanding three-role system)
- [ ] Miscellaneous Polish (branching strategy, consistency, clarity)
- **Effort:** 2-3 hours | **Designed, ready for implementation**

---

## Implementation Timeline

**After Phase 1 (P0) - COMPLETED ✅**
- Packet completeness is verifiable (no subjectivity)
- BOOTSTRAP format is crystal clear
- Coder knows validation order
- Coder has error recovery playbook
- **Grade: A- (88/100)**

**After Phase 2 (P1) - COMPLETED ✅**
- Hard invariants explained with grep commands and fix examples (Step 6 + enforcement guide)
- Test coverage minimums clear with tarpaulin verification (Step 7.5)
- Scope conflicts caught early with step 1.5 adequacy check
- DONE_MEANS verified with file:line evidence during implementation (Step 6.5)
- **Grade: A (93/100)**

**After Phase 3 (P2) - Designed**
- AI review severity objective
- Governance rules explicit
- Ecosystem context clear
- Polish complete
- **Grade: A+ (99/100) = 9.9/10 ✨**

---

**Total effort to reach 9.9/10: 7-10 hours (all cheap LLM tier)**
**Cost: LOW (documentation + clarification, no code changes)**
