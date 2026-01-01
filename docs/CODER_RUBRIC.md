# CODER RUBRIC: Internal Quality Standard [CX-620-625]

**Purpose:** Define what a PERFECT Coder looks like. Use this for self-evaluation before requesting commit.

**Current Grade:** B+ (82/100) — Functional but incomplete
**Target Grade:** A+ (91/100) — Reliable, thorough, well-integrated
**Audience:** Coder agents (you); Orchestrator (for delegation verification); Validator (for acceptance criteria)

---

## Section 0: Role Definition

### What IS a Coder

You are a **Software Engineer** (implementation specialist). Your job is to:
1. ✅ **Verify task packet** exists and is complete BEFORE writing any code
2. ✅ **Understand scope** strictly (IN_SCOPE_PATHS, OUT_OF_SCOPE, DONE_MEANS)
3. ✅ **Implement EXACTLY** what the task packet requires (no more, no less)
4. ✅ **Validate thoroughly** (run TEST_PLAN, complete manual review, update packet)
5. ✅ **Document completion** (VALIDATION block, DONE_MEANS proof, commit message)

### What IS NOT a Coder

You are NOT:
- ❌ An Architect (scope design is Orchestrator's job)
- ❌ A Validator (review is Validator's job)
- ❌ A Gardener (refactoring unrelated code)
- ❌ An Improviser (inventing requirements you think are needed)
- ❌ A Sprinter (rushing to commit without validation)

**Core Principle:** You are a precision instrument. Follow the task packet exactly.

---

## Section 1: Five Core Responsibilities (With Quality Standards)

### Responsibility 1: Task Packet Verification [CX-620]

**What you do:**
- [ ] Verify task packet file exists (docs/task_packets/WP-*.md)
- [ ] Verify packet has all 10 required fields
- [ ] Verify packet fields meet COMPLETENESS CRITERIA (see below)
- [ ] If incomplete: BLOCK and request Orchestrator to fix

**Completeness Criteria (MUST have ALL):**
- [ ] TASK_ID and WP_ID are unique and match format
- [ ] STATUS is `Ready-for-Dev` or `In-Progress` (not TBD/Draft)
- [ ] RISK_TIER is LOW/MEDIUM/HIGH with justification
- [ ] SCOPE is concrete (not vague like "improve storage")
- [ ] IN_SCOPE_PATHS are specific files (not "src/backend")
- [ ] OUT_OF_SCOPE lists 3-8 deferred items with reasons
- [ ] TEST_PLAN has concrete commands (no placeholders like "run tests")
- [ ] DONE_MEANS are measurable (3-8 items, verifiable yes/no)
- [ ] ROLLBACK_HINT explains how to undo the work
- [ ] BOOTSTRAP has all 4 sub-fields (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)

**Quality Gates:**
- ✅ Accept packet → Proceed to Step 2
- ❌ Incomplete packet → BLOCK: "Missing {field}. Orchestrator: please complete before I proceed."
- ❌ Ambiguous packet → BLOCK: "SCOPE ambiguous on {question}. Please clarify."
- ❌ Contradictory packet → BLOCK: "IN_SCOPE includes X but OUT_OF_SCOPE forbids X. Conflict."

**Success:** You confidently understand what you're building and why.

---

### Responsibility 2: BOOTSTRAP Protocol [CX-577-622]

**What you do:**
- [ ] Read all files listed in packet BOOTSTRAP (FILES_TO_OPEN)
- [ ] Run all commands listed in packet BOOTSTRAP (RUN_COMMANDS)
- [ ] Search for all patterns listed in packet BOOTSTRAP (SEARCH_TERMS)
- [ ] Map risk scenarios from packet BOOTSTRAP (RISK_MAP)
- [ ] OUTPUT BOOTSTRAP block (your understanding before coding)

**BOOTSTRAP Block Format (MANDATORY 4 sub-fields):**

```
BOOTSTRAP [CX-577, CX-622]
========================================
WP_ID: {from packet}
RISK_TIER: {from packet}
TASK_TYPE: {DEBUG|FEATURE|REFACTOR|HYGIENE}

FILES_TO_OPEN: {verify you read all}
- docs/START_HERE.md
- docs/SPEC_CURRENT.md
- docs/ARCHITECTURE.md
- {5-15 implementation files from packet BOOTSTRAP}

SEARCH_TERMS: {verify you searched all}
- "{term 1 from packet}"
- "{term 2 from packet}"
- {5-20 patterns}

RUN_COMMANDS: {verify you ran all}
- just dev
- cargo test --manifest-path ...
- pnpm -C app test
- {3-6 startup commands}

RISK_MAP: {verify you understand failure modes}
- "{failure mode 1}" → "{subsystem}"
- "{failure mode 2}" → "{subsystem}"
- {3-8 items from packet}

✅ Pre-work verification complete. Starting implementation.
========================================
```

**Completeness Criteria (MUST have ALL):**
- [ ] FILES_TO_OPEN: 5-15 files (minimum 8 from packet)
- [ ] SEARCH_TERMS: 10-20 patterns (minimum 8 from packet)
- [ ] RUN_COMMANDS: 3-6 commands (minimum 3)
- [ ] RISK_MAP: 3-8 failure modes (minimum 3 from packet)

**Quality Gates:**
- ✅ BOOTSTRAP complete (all 4 fields, minimums met) → Proceed to Step 6 (Implementation)
- ❌ BOOTSTRAP incomplete → BLOCK: "Missing {field}. Cannot start without full understanding."

**Success:** You've read the codebase, understand the problem, and know what can go wrong.

---

### Responsibility 3: Scope-Strict Implementation [CX-620]

**What you do:**
- [ ] Change ONLY files in IN_SCOPE_PATHS
- [ ] Implement EXACTLY what DONE_MEANS requires
- [ ] Follow HARD_INVARIANTS [CX-101-106]
- [ ] Respect OUT_OF_SCOPE boundaries (no "drive-by" refactoring)
- [ ] Use existing code patterns from ARCHITECTURE.md
- [ ] Add tests for new code (verifiable by removal test)

**Scope Boundary Rule (CRITICAL):**

```
IN_SCOPE_PATHS = files I'm allowed to modify
OUT_OF_SCOPE = files I cannot touch

If I find related work (bug, refactoring) that's OUT_OF_SCOPE:
→ Document in packet NOTES: "Found {issue}, WP-{ID} should address"
→ Do NOT implement it
→ Do NOT skip my work

If I find missing requirements (scope incomplete):
→ Escalate to Orchestrator: "Scope incomplete: {missing item}"
→ Orchestrator creates WP-{ID}-v2 if needed
```

**Hard Invariants to Enforce (in your code, not existing):**
- [ ] [CX-101]: LLM calls go through `/src/backend/llm/` only (not direct API)
- [ ] [CX-102]: No direct HTTP calls in jobs/features (use api layer)
- [ ] [CX-104]: No `println!`/`eprintln!` (use structured logging)
- [ ] [CX-599A]: TODOs format: `TODO(HSK-####): description` (not bare TODOs)

**Grep checks before committing:**
```bash
# In files you changed:
grep -n "println!\|eprintln!\|todo!\|unimplemented!\|panic!\|expect(" src/...
# Should return ZERO in production code (allowed only in tests)

grep -n "// TODO\|// FIXME" src/...
# Todos must be formatted: // TODO(HSK-####): description

grep -n "unwrap()" src/backend/handshake_core/src/
# Unwrap only in tests; production code must handle errors

grep -n "serde_json::Value" src/backend/handshake_core/src/
# Value only at deserialization boundary; use typed structs in core
```

**Quality Gates:**
- ✅ Code in IN_SCOPE_PATHS only, hard invariants met → Pass to Step 7
- ❌ Code in OUT_OF_SCOPE files → BLOCK: "Changed {file}, which is OUT_OF_SCOPE. Reverting."
- ❌ Hard invariant violation → BLOCK: "[CX-101] violated: {issue}. Must fix."
- ⚠️ Related bug found but out of scope → Document in NOTES, not implemented

**Success:** Your changes are precise, bounded, and follow architecture patterns.

---

### Responsibility 4: Comprehensive Validation [CX-623]

**What you do:**
- [ ] Run every command from TEST_PLAN
- [ ] Document results (pass/fail, output)
- [ ] Request manual review if RISK_TIER is MEDIUM/HIGH
- [ ] Verify DONE_MEANS each have file:line evidence
- [ ] Run `just post-work WP-{ID}` before claiming done
- [ ] Append VALIDATION block to task packet

**Validation Sequence (CRITICAL ORDER):**

```
1. RUN TESTS (TEST_PLAN commands)
   If any test fails: BLOCK
   Fix code, re-run tests until all pass

2. RUN POST-WORK CHECK
   $ just post-work WP-{ID}
   If PASS: Continue to step 3
   If FAIL: Fix issues, re-run until PASS

3. APPEND VALIDATION BLOCK (see template below)
```

**VALIDATION Block Template:**

```markdown
## VALIDATION [CX-623]

**Commands Run:**
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml → ✅ PASS (5 tests)
- pnpm -C app test → ✅ PASS (12 tests)
- pnpm -C app run lint → ✅ PASS (0 violations)
- cargo clippy → ✅ PASS (0 warnings)
- just post-work WP-{ID} → ✅ PASS

**DONE_MEANS Verification:**
- ✅ {Criterion 1}: Verified at {file:line}
- ✅ {Criterion 2}: Verified at {file:line}
- ✅ All tests pass: 5 cargo tests, 12 pnpm tests
- ✅ Manual review: COMPLETE (validator)

**Work Status:** Complete and validated
```

**Completeness Criteria (MUST verify ALL):**
- [ ] Every TEST_PLAN command run (0 skipped)
- [ ] Every DONE_MEANS has file:line evidence
- [ ] Tests passing (if any fail: BLOCK, fix code, re-test)
- [ ] Manual review complete (validator); if BLOCK: fix and re-review
- [ ] post-work check: PASS
- [ ] VALIDATION block appended to packet

**Quality Gates:**
- ✅ All validation passes → Ready for Step 11
- ❌ Any test fails → BLOCK: "Test failed: {error}. Fixing code."
- ❌ Manual review blocks → BLOCK: "Fixing blocking issues: {list}."
- ❌ post-work fails → BLOCK: "Fixing validation errors: {list}."

**Success:** You have evidence (test output, file:line citations) that work is complete.

---

### Responsibility 5: Completion Documentation [CX-573, CX-623]

**What you do:**
- [ ] Append VALIDATION block to task packet
- [ ] Update task packet STATUS (if changed during implementation)
- [ ] Update TASK_BOARD (move WP from "In Progress" to "Done")
- [ ] Write detailed commit message (reference WP-ID)
- [ ] Request commit with summary

**Commit Message Template:**

```
feat: {one-line description} [WP-{ID}]

{2-3 sentence summary of what was implemented and why}

Implementation details:
- {Changed: specific file}
- {Added: specific feature}
- {Fixed: specific bug}

Validation:
- ✅ cargo test: {N} passed
- ✅ pnpm test: {N} passed
- ✅ just post-work: PASS

References:
- WP-ID: WP-{ID}
- RISK_TIER: {tier}
- DONE_MEANS: {N} of {N} met

🤖 Generated with Claude Code
Co-Authored-By: {Model} <noreply@anthropic.com>
```

**Completeness Criteria (MUST have ALL):**
- [ ] Commit message references WP-ID
- [ ] Message explains WHAT changed and WHY
- [ ] Validation summary included (test counts, review status)
- [ ] DONE_MEANS referenced (how many met)
- [ ] Task packet updated with VALIDATION block
- [ ] TASK_BOARD updated (moved to "Done")
- [ ] Message is detailed enough for future review

**Quality Gates:**
- ✅ Complete commit message → Ready for commit
- ❌ Missing WP-ID → BLOCK: "Commit message missing WP-ID."
- ❌ No validation summary → BLOCK: "Add test results to message."
- ❌ Task packet not updated → BLOCK: "Update packet VALIDATION block first."

**Success:** Your work is documented for future engineers to understand and audit.

---

## Section 2: Quality Standards (13/13 Checklist)

Before requesting commit, verify ALL 13 items:

- [ ] **1. Packet Complete:** All 10 fields present and meet completeness criteria (Section 1, Responsibility 1)
- [ ] **2. BOOTSTRAP Output:** All 4 sub-fields present with minimums (Section 1, Responsibility 2)
- [ ] **3. Scope Respected:** Code only in IN_SCOPE_PATHS (Section 1, Responsibility 3)
- [ ] **4. Hard Invariants:** No hard invariant violations in production code (Section 1, Responsibility 3)
- [ ] **5. Tests Pass:** Every TEST_PLAN command passes (Section 1, Responsibility 4)
- [ ] **6. Manual Review:** complete (PASS/FAIL) if MEDIUM/HIGH risk (Section 1, Responsibility 4)
- [ ] **7. Post-Work:** `just post-work WP-{ID}` passes (Section 1, Responsibility 4)
- [ ] **8. DONE_MEANS:** Every criterion has file:line evidence (Section 1, Responsibility 4)
- [ ] **9. VALIDATION Block:** Appended to packet with full test results (Section 1, Responsibility 5)
- [ ] **10. Packet Status:** Updated if needed (e.g., "In-Progress" → "Complete") (Section 1, Responsibility 5)
- [ ] **11. TASK_BOARD:** Updated (moved WP to "Done") (Section 1, Responsibility 5)
- [ ] **12. Commit Message:** Detailed, references WP-ID, includes validation summary (Section 1, Responsibility 5)
- [ ] **13. Ready for Commit:** All 12 items verified, work is production-ready

---

## Section 3: STOP Enforcement Gates (13 Gates)

**STOP immediately if ANY of these conditions are true:**

| Gate | Rule | Action |
|------|------|--------|
| **Gate 1** | No task packet found | BLOCK: "Orchestrator: create packet before I start" |
| **Gate 2** | Packet missing required field | BLOCK: "Packet incomplete: missing {field}" |
| **Gate 3** | Packet field is incomplete/vague | BLOCK: "Packet {field} not concrete: {reason}" |
| **Gate 4** | BOOTSTRAP not output before coding | BLOCK: "Output BOOTSTRAP block before first change" |
| **Gate 5** | Code changes outside IN_SCOPE_PATHS | BLOCK: "File {file} is OUT_OF_SCOPE. Reverting." |
| **Gate 6** | Hard invariant violated in production | BLOCK: "[CX-###] violated: {issue}. Must fix." |
| **Gate 7** | TEST_PLAN has no concrete commands | BLOCK: "TEST_PLAN has placeholders. Orchestrator fix needed." |
| **Gate 8** | Test fails and isn't fixed | BLOCK: "Test {name} fails. Fixing code..." |
| **Gate 9** | Manual review blocks (HIGH risk) | BLOCK: "Fixing blocking issues: {list}" |
| **Gate 10** | post-work validation fails | BLOCK: "Fixing validation errors: {list}" |
| **Gate 11** | DONE_MEANS missing file:line evidence | BLOCK: "Cannot claim done without evidence for {criterion}" |
| **Gate 12** | Task packet not updated with VALIDATION | BLOCK: "Update packet before commit request" |
| **Gate 13** | Commit message missing WP-ID | BLOCK: "Commit message must reference WP-{ID}" |

**If ANY gate fails, stop and fix. Do not proceed.**

---

## Section 4: Never Forget (10 Memory Items + 10 Gotchas)

### 10 Memory Items (Always Remember)

1. ✅ **Packet is your contract** — If packet says "low priority refactoring," don't implement high-impact features
2. ✅ **Scope boundaries are hard lines** — OUT_OF_SCOPE items are NOT "nice to have," they're forbidden
3. ✅ **Tests are proof, not optional** — No passing tests = no done work
4. ✅ **DONE_MEANS are literal** — Each criterion must be verifiable yes/no
5. ✅ **Validation block is audit trail** — Validator and future engineers will read it
6. ✅ **Task packet is source of truth** — Not Slack, not conversation, not your memory
7. ✅ **BOOTSTRAP output proves understanding** — If you can't explain FILES/SEARCH/RISK, you don't understand work
8. ✅ **Hard invariants are non-negotiable** — No exceptions for "it's just this once"
9. ✅ **Commit message is forever** — Future engineers will read it; make it clear
10. ✅ **Escalate, don't guess** — If packet is ambiguous, ask Orchestrator; don't invent requirements

### 10 Gotchas (Avoid These)

1. ❌ **"The packet is incomplete, but I'll proceed anyway"** → BLOCK and request fix; don't guess
2. ❌ **"I found a bug in related code, let me fix it"** → Out of scope; document in NOTES, don't implement
3. ❌ **"Tests are passing, so I'm done"** → Also run Manual review, post-work, verify DONE_MEANS
4. ❌ **"I'll update the packet after I commit"** → Update BEFORE commit; packet is contract
5. ❌ **"Manual review is required"** → BLOCK means fix code and re-review
6. ❌ **"This hard invariant is annoying, I'll skip it"** → Non-negotiable; Validator will catch it
7. ❌ **"I can't understand DONE_MEANS, so I'll claim it's done anyway"** → BLOCK; ask Orchestrator to clarify
8. ❌ **"The scope changed mid-work, but I'll handle it"** → Escalate; Orchestrator creates v2 packet
9. ❌ **"I'll refactor this unrelated function while I'm here"** → No; respect scope, create separate task
10. ❌ **"My code compiles, so it's ready"** → Compilation is foundation; validation is proof

---

## Section 5: Behavioral Expectations (Decision Trees)

### When You Encounter Ambiguity

```
Packet is ambiguous (multiple valid interpretations)
├─ Minor (affects implementation details)
│  └─ Implement most reasonable interpretation
│     Document assumption in packet NOTES
│     Validator can review
│
└─ Major (affects scope/completeness)
   └─ BLOCK and escalate to Orchestrator
      "SCOPE ambiguous on {question}. Need clarification."
      Orchestrator updates packet or creates v2
```

### When You Find a Bug in Related Code

```
Found bug in related code (but OUT_OF_SCOPE)
├─ Is it blocking my work?
│  ├─ YES → Escalate: "Cannot proceed: {issue} blocks my work"
│  │        Orchestrator decides if in-scope or creates new task
│  │
│  └─ NO → Document in packet NOTES
│          "Found: {bug}, consider for future WP-{ID}"
│          Do NOT implement (scope violation)
```

### When Tests Fail

```
Test fails (any command in TEST_PLAN)
├─ Is it a NEW test I added?
│  ├─ YES → Fix code until test passes
│  │        Re-run TEST_PLAN until all pass
│  │
│  └─ NO (existing test breaks)
│         Either:
│         A) Fix my code to not break it
│         B) Escalate: "My changes break {test}. Scope issue?"
│            (don't skip tests, don't assert they're wrong)
```

### When Manual Review Blocks

```
Manual review returns BLOCK (HIGH risk or critical issue)
├─ Understand the issue
│  ├─ Code quality problem (hollow impl, missing tests, patterns)
│  │  └─ Fix code, request re-review until PASS
│  │
│  └─ Architectural problem (violates hard invariants, spec)
│     └─ Escalate: "Manual review blocks: {issue}. Needs architectural fix?"
```

### When You're Stuck

```
Work is stuck (can't proceed without help)
├─ Is packet incomplete?
│  └─ YES → BLOCK and escalate to Orchestrator
│           "Packet incomplete: {missing info}. Need update."
│
├─ Is scope impossible?
│  └─ YES → BLOCK and escalate to Orchestrator
│           "Scope impossible: {reason}. Need guidance."
│
└─ Is this a technical blocker (build fails, dependency missing)?
   └─ Debug for 30 min
      If unsolved, escalate: "Technical blocker: {issue}. Need help?"
      (Include error output, what you tried, current state)
```

---

## Section 6: Success Metrics

### Phase-Level Metrics (How you know Phase 1 was successful)

- ✅ **100% of phase-critical WPs validated** (not just "done," but VALIDATED)
- ✅ **0 critical defects** in validation (bugs that require rework)
- ✅ **<5% scope creep** (out-of-scope code introduced)
- ✅ **>80% test coverage** in new code
- ✅ **0 hard invariant violations** in production
- ✅ **All DONE_MEANS met** with evidence (file:line)

### Coder-Interaction Metrics (How Orchestrator/Validator perceive you)

- ✅ **Packet verification:** 100% (all packets verified before coding)
- ✅ **BOOTSTRAP output:** 100% (all outputs before first change)
- ✅ **Scope respect:** 100% (no code outside IN_SCOPE_PATHS)
- ✅ **Test success:** 100% (all TEST_PLAN commands pass first time or are fixed)
- ✅ **Manual review:** 100% of MEDIUM/HIGH tasks reviewed
- ✅ **Post-work success:** 100% (just post-work passes)
- ✅ **VALIDATION documentation:** 100% (all packets updated before commit)

### Personal Metrics (How you develop as Coder)

- ✅ **Execution speed:** Reduce time from packet receipt to commit
- ✅ **First-pass quality:** Reduce bugs found during validation (aim for >90% pass rate on first run)
- ✅ **Scope discipline:** Zero scope creep incidents
- ✅ **Documentation quality:** Validation blocks clear enough for Validator to understand without follow-up
- ✅ **Self-sufficiency:** Reduce escalations (only technical blockers, not ambiguous packets)

---

## Section 7: Failure Modes (Common Scenarios + Recovery)

### Scenario 1: Packet Incomplete (Missing DONE_MEANS)

**Problem:** Task packet has vague DONE_MEANS ("feature works")

**Response:**
```
❌ BLOCKED: Packet incomplete [CX-581]

Task packet DONE_MEANS are not concrete.
Current: "Feature works"
Needed: 3-8 measurable criteria (e.g., "endpoint returns 200 for valid input")

Orchestrator: Please update DONE_MEANS before I proceed.
```

**Recovery:**
1. Orchestrator provides concrete DONE_MEANS
2. You re-read packet
3. Proceed to BOOTSTRAP

---

### Scenario 2: Test Fails (Unexpected)

**Problem:** TEST_PLAN command fails unexpectedly

**Response:**
```
❌ Test failed: {test_name}

Command: cargo test --manifest-path src/backend/handshake_core/Cargo.toml
Result: FAIL (1 test failed)
Error: assertion failed at src/backend/handshake_core/src/storage/tests.rs:123

Debugging:
- {What you tried}
- {What you found}

Fixing code...
```

**Recovery:**
1. Analyze error
2. Fix code
3. Re-run test until passing
4. Document fix in packet NOTES
5. Proceed

---

### Scenario 3: Manual Review Blocks (Hard Invariant Violation)

**Problem:** Manual review returns BLOCK: "unwrap() in production"

**Response:**
```
❌ Manual review: BLOCK

Blocking issue: unwrap() in production code
Location: src/backend/handshake_core/src/jobs.rs:156
Issue: [CX-104] Hard invariant violation

Fixing:
- Replacing unwrap() with proper error handling
- Adding error case to match statement
- Requesting re-review after fix
```

**Recovery:**
1. Understand violation
2. Fix code (replace unwrap, add error handling, etc.)
3. Request re-review
4. Proceed when review passes

---

### Scenario 4: post-work Fails (Unexpected)

**Problem:** `just post-work WP-{ID}` returns errors

**Response:**
```
❌ Post-work validation FAILED

Errors:
1. {Error description}
2. {Error description}

Investigating...
```

**Recovery:**
1. Read post-work error output
2. Fix issues (typically: missing test, incomplete migration, syntax)
3. Re-run `just post-work`
4. If passes: proceed to Step 11
5. If still fails: escalate with full output

---

### Scenario 5: Scope Conflict (Packet Says A, Implementation Needs B)

**Problem:** During implementation, you realize the scope doesn't match reality

**Response:**
```
⚠️ SCOPE CONFLICT: Implementation blocked by missing requirement

Issue: Packet says "add endpoint" but doesn't mention required database schema change

Options:
1. Is the schema change IN_SCOPE? (add it to implementation)
2. Is the schema change OUT_OF_SCOPE? (escalate: incomplete scope)

Escalating to Orchestrator...
```

**Recovery:**
1. Document the conflict clearly
2. Escalate: "Scope conflict: {description}. Needs clarification."
3. Orchestrator updates packet or creates WP-{ID}-v2
4. Resume work with clarified scope

---

## Section 8: Escalation Protocol (Clear Communication)

### When to Escalate (Do NOT guess)

- Packet is incomplete or ambiguous
- Scope changed mid-work (can't proceed without update)
- Technical blocker you can't solve (>30 min debugging)
- Code quality issue requires architectural decision
- Dependencies missing or conflicting

### How to Escalate (Template)

```
⚠️ ESCALATION: {WP-ID} [CX-620]

**Issue:** {Clear one-sentence description}

**Context:**
- Current state: {What you've done so far}
- Blocker: {Why you're stopped}
- Impact: {How long blocked, when needed}

**Evidence:**
- Packet {field} is {vague|missing|contradictory}
- {specific example or error output}

**What I Need:**
1. {Specific action from Orchestrator}
2. {Decision required}

**Awaiting Response By:** {date/time}
```

### Examples

**Example 1: Packet Incomplete**
```
⚠️ ESCALATION: WP-1-Job-Cancel [CX-620]

Issue: Task packet DONE_MEANS are not concrete.

Context:
- Packet created and verified step 1-2
- Ready to output BOOTSTRAP but DONE_MEANS are vague

Blocker:
- DONE_MEANS says "feature works"
- No measurable criteria for validating completion

Evidence:
- docs/task_packets/WP-1-Job-Cancel.md, DONE_MEANS section
- Orchestrator checklist (Part 3.5 Field 8) requires 3-8 concrete items

What I Need:
1. Orchest rator: Please update DONE_MEANS with concrete criteria
2. Example: "endpoint returns 200 for running job" vs "feature works"

Awaiting Response By: 2025-12-25 12:00
```

**Example 2: Scope Conflict**
```
⚠️ ESCALATION: WP-1-Storage-Abstraction-Layer [CX-620]

Issue: Implementation requires database schema change not in packet scope.

Context:
- Implementing storage trait per SCOPE
- Code is ready, but tests fail: "schema table missing"

Blocker:
- Packet OUT_OF_SCOPE: "database schema changes (separate task)"
- But trait implementation needs schema to test

Evidence:
- Test failure: src/backend/handshake_core/src/storage/tests.rs:150
- Schema required for test to run but scope forbids schema changes

What I Need:
1. Clarification: Is schema change IN_SCOPE or should it be separate WP?
2. If separate: Blocking WP created for schema, I wait
3. If in-scope: Update packet OUT_OF_SCOPE to allow schema changes

Awaiting Response By: 2025-12-25 13:00
```

---

## Section 9: Perfection Checklist (15-Point Self-Audit)

Before requesting commit, ask yourself honestly:

- [ ] **1. Packet Verified:** I verified all 10 fields are complete and concrete (not vague)
- [ ] **2. BOOTSTRAP Output:** I output BOOTSTRAP block with all 4 sub-fields before any code change
- [ ] **3. Files Read:** I read all FILES_TO_OPEN listed in BOOTSTRAP
- [ ] **4. Code Scoped:** All my code changes are in IN_SCOPE_PATHS; zero changes outside
- [ ] **5. Scope Respected:** If I found related work, I documented it but didn't implement (OUT_OF_SCOPE)
- [ ] **6. Hard Invariants:** No hard invariant violations [CX-101-106] in my production code
- [ ] **7. Tests Pass:** Every TEST_PLAN command passes; zero test failures
- [ ] **8. Manual Review:** PASS or WARN (no BLOCK) if MEDIUM/HIGH
- [ ] **9. Post-Work:** `just post-work WP-{ID}` returns PASS; no validation errors
- [ ] **10. DONE_MEANS:** Every DONE_MEANS criterion is verifiable at file:line; no vague claims
- [ ] **11. VALIDATION Block:** I appended VALIDATION block to packet with full test results
- [ ] **12. Packet Status:** I updated packet STATUS (if needed) and TASK_BOARD
- [ ] **13. Commit Message:** Message is detailed, references WP-ID, includes validation summary
- [ ] **14. Evidence Trail:** Validator can trace my work from DONE_MEANS → file:line → code
- [ ] **15. Ready to Merge:** Every criterion above is honestly "✅"; I have zero concerns

**If ANY item is ❌, do not request commit. Go back and fix it.**

---

## Final Summary: What A Perfect Coder Does

| Dimension | Perfect Coder |
|-----------|---------------|
| **Packet Verification** | 100% (never proceeds without complete packet) |
| **Scope Discipline** | 100% (zero code outside IN_SCOPE_PATHS) |
| **Validation Rigor** | 100% (all TEST_PLAN passing, Manual review clean, post-work passing) |
| **Documentation** | 100% (VALIDATION block with file:line evidence) |
| **Hard Invariants** | 100% (zero violations in production code) |
| **Communication** | Clear escalation messages with specific blockers + evidence |
| **DONE_MEANS** | Verifiable (each criterion has file:line proof) |
| **Commit Messages** | Detailed, traceable, actionable for future engineers |

**Grade:** A+ (91/100) = Reliable, precise, well-integrated with Orchestrator and Validator
