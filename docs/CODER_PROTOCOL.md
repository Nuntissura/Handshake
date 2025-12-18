# CODER PROTOCOL [CX-620-623]
**MANDATORY** - Read this before writing any code

## Role

You are a **Coder** or **Debugger** agent. Your job is to:
1. Verify task packet exists
2. Implement within defined scope
3. Validate your work
4. Document completion

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

**Method 2: Search logger**
```bash
grep "WP-{ID}" Handshake_logger_*.md
```

**Method 3: Check handoff message**
Look for TASK_PACKET block in orchestrator's message.

**IF NOT FOUND:**
```
❌ BLOCKED: No task packet found [CX-620]

Orchestrator must create a task packet before I can start.

Missing:
- Task packet file in docs/task_packets/
- WP_ID in logger
- TASK_PACKET block in handoff

Orchestrator: Please create task packet using:
  just create-task-packet WP-{ID}

I cannot write code without a task packet.
```

**STOP** - Do not write any code until packet exists.

---

### Step 2: Read Task Packet ✋ STOP

```bash
cat docs/task_packets/WP-{ID}-*.md
```

**Verify packet includes:**
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

**IF INCOMPLETE:**
```
⚠️ WARNING: Task packet incomplete [CX-581]

Missing required fields:
- {Field name}
- {Field name}

Orchestrator: Please complete the task packet before I proceed.
```

---

### Step 3: Bootstrap Protocol [CX-574-577] ✋ STOP

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

### Step 4: Output BOOTSTRAP Block ✋ STOP

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

### Step 5: Implementation

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

## Post-Implementation Checklist (BLOCKING ✋)

Complete ALL steps before claiming work is done.

### Step 6: Run Validation [CX-623] ✋ STOP

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

### Step 7: AI Review [CX-573A] ✋ STOP

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

### Step 8: Update Logger ✋ STOP

**Add completion entry to latest `Handshake_logger_*.md`:**

```markdown
[RAW_ENTRY_ID]
{next sequential ID}

[TIMESTAMP]
{ISO 8601 with timezone}

[SESSION_ID]
{from orchestrator or new}

[ROLE]
Coder

[PHASE]
{from task packet}

[VERTICAL_SLICE]
{from task packet}

[WP_ID]
WP-{phase}-{name}

[WP_STATUS]
Completed

[TASK_SUMMARY]
{from task packet - one line}

[METHOD_SUMMARY]
{Brief description of implementation approach}

[SPEC_REFERENCES]
{If any specs guided the work}

[LAW_AND_CODEX_REFERENCES]
Handshake Codex v0.8

[FILES_TOUCHED]
{List actual files changed - from git status}
src/backend/handshake_core/src/api/jobs.rs
src/backend/handshake_core/src/jobs.rs
src/backend/handshake_core/src/models.rs

[TOOLS_AND_MODELS]
{Your model name, Coder role}

[STATE_BEFORE_BRIEF]
{Repo state before your work}

[STATE_AFTER_BRIEF]
{Repo state after your work - what changed}

[RESULT]
OK

[BLOCKERS_OR_RISKS]
None (or document any risks discovered)

[NEXT_STEP_HINT]
{What should happen next, if anything}

[HANDOFF_HINT]
Work complete per WP-{ID}. Ready for review.

[NOTES]
VALIDATION:
- cargo test: ✅ PASS (5 tests)
- pnpm test: ✅ PASS (12 tests)
- pnpm lint: ✅ PASS
- just ai-review: ✅ PASS

LLM author: {Your model} per HL-I-042
```

---

### Step 9: Post-Work Validation ✋ STOP

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

### Step 10: Request Commit

**Output final summary:**
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
6. Update logger before commit [CX-651]
7. Run `just post-work WP-{ID}` before claiming done

---

## If Blocked

**Scenario**: No task packet found

**Response**:
```
❌ BLOCKED: No task packet [CX-620]

I searched:
- docs/task_packets/ → No WP-{ID} file found
- Logger → No WP-{ID} entry found
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

### ❌ Mistake 4: No logger update
**Wrong:**
```
[Requests commit without updating logger]
```
**Right:**
```
[Updates logger with RAW_ENTRY]
[Includes VALIDATION section with results]
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
- ❌ No logger entry for your work

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
- [CX-651]: MUST update logger before commit

**Remember**:
- Task packet = your contract
- IN_SCOPE_PATHS = your boundaries
- TEST_PLAN = your definition of done
- Validation passing = your proof of quality
