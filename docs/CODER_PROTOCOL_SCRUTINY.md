# CODER_PROTOCOL SCRUTINY [Context-Informed Analysis]

**Analysis Date:** 2025-12-25
**Scope:** Compare CODER_PROTOCOL (585 lines) against ORCHESTRATOR_PROTOCOL (A+ grade, 2,100+ lines) and VALIDATOR_PROTOCOL standards
**Purpose:** Identify gaps, contradictions, ambiguities preventing optimal Coder performance

---

## Executive Summary

**Current Grade:** B+ (82/100)
**Key Issues:** 10 critical/high-priority gaps; 8 medium-priority gaps
**Root Cause:** Protocol defines WHAT Coder does (11 steps) but lacks CLARITY on HOW to handle edge cases, error recovery, and alignment with Orchestrator/Validator expectations

---

## CRITICAL ISSUES (Must fix for A-level performance)

### Issue 1.1: Missing "Packet Completeness" Objective Criteria [CX-620-VARIANT]

**Problem:**
- Step 2 asks Coder to "verify packet includes" 10 fields
- But no objective criteria for what "complete" means
- Example: DONE_MEANS could be "vague" (feature works) vs "concrete" (3-8 measurable items)
- Coder can't reject incomplete packet because no checklist

**Evidence:**
- CODER_PROTOCOL Step 2 (line 63-73): Just lists field names, no completeness criteria
- VALIDATOR_PROTOCOL Pre-Flight (line 10-18): Defines CONCRETE packet requirements
- ORCHESTRATOR_PROTOCOL Part 3.5 (260+ lines): Defines exact completeness for each field

**Impact:** Coder accepts incomplete packets; proceeds with ambiguous scope → validation failures

**What ORCHESTRATOR does:** Part 3.5 has 10 field definitions with exact completeness criteria (e.g., "DONE_MEANS MUST be 3-8 items, each testable, 1:1 mapped to SPEC_ANCHOR")

**Fix needed:** Add "Packet Completeness Checklist" to CODER_PROTOCOL Step 2 with objective criteria for each field

---

### Issue 1.2: BOOTSTRAP Output Format Unclear [CX-577-CX-622 ambiguity]

**Problem:**
- Step 5 shows BOOTSTRAP example (lines 127-159) but doesn't define what fields are MANDATORY
- Coder could output partial BOOTSTRAP (missing RISK_MAP or SEARCH_TERMS)
- Validator checks for BOOTSTRAP per VALIDATOR_PROTOCOL line 22 but protocol is vague

**Evidence:**
- CODER_PROTOCOL Step 5 shows template but no "MUST include" checklist
- VALIDATOR_PROTOCOL line 22: "Confirm Coder outputted BOOTSTRAP block per CODER_PROTOCOL [CX-577, CX-622]; if missing/incomplete, halt"
- No objective: what makes BOOTSTRAP "incomplete"?

**Impact:** Validator halts for ambiguous "incomplete" BOOTSTRAP; Coder confused about what's required

**What ORCHESTRATOR does:** Part 3.5 Field 10 (BOOTSTRAP) has 4 sub-fields with exact requirements:
- FILES_TO_OPEN: 5-15 files minimum
- SEARCH_TERMS: 10-20 patterns minimum
- RUN_COMMANDS: 3-6 startup commands
- RISK_MAP: 3-8 failure modes

**Fix needed:** Add BOOTSTRAP Completeness Checklist to CODER_PROTOCOL Step 5

---

### Issue 1.3: TEST_PLAN Ambiguity — What if Packet Missing Commands? [CX-623 gap]

**Problem:**
- Step 7 says "Run ALL commands from TEST_PLAN"
- But what if task packet has placeholder/incomplete TEST_PLAN?
- CODER_PROTOCOL doesn't say: "Reject packet if TEST_PLAN is incomplete"
- VALIDATOR_PROTOCOL line 14 requires "TEST_PLAN commands present (no placeholders)" but Coder has no gate

**Evidence:**
- CODER_PROTOCOL Step 2: Lists TEST_PLAN as required field but no completeness check
- CODER_PROTOCOL Step 7: Assumes TEST_PLAN is always complete and runnable
- VALIDATOR_PROTOCOL Pre-Flight: Explicitly blocks if TEST_PLAN has placeholders

**Impact:** Coder runs incomplete/placeholder TEST_PLAN (e.g., "pnpm test" without specific path); misses real validation

**What ORCHESTRATOR does:** Part 3.5 Field 7 requires:
- For LOW tier: At least 2-3 commands (cargo test, lint)
- For MEDIUM tier: 4-5 commands (add `manual review`)
- For HIGH tier: 5-6 commands (add stricter checks)
- Every command literal (copy-paste ready)

**Fix needed:** Add TEST_PLAN completeness criteria to CODER_PROTOCOL Step 2

---

### Issue 1.4: No Error Recovery Procedures [CX-572-gap]

**Problem:**
- CODER_PROTOCOL has "If Blocked" section (lines 403-451) but only 3 scenarios
- No recovery procedures for common Coder mistakes:
  - Tests fail, how to debug?
  - manual review blocks, how to understand the issue?
  - Scope creep happened, how to fix?
  - Task packet conflicts with implementation, what to do?

**Evidence:**
- CODER_PROTOCOL lines 403-451: Only shows blocking responses, not recovery
- ORCHESTRATOR_PROTOCOL Part 3-Error-Recovery: 6 detailed error scenarios with recovery steps
- VALIDATOR_PROTOCOL line 110-111: Explains escalation but not Coder recovery

**Impact:** When tests fail, Coder has limited guidance on how to debug efficiently

**What ORCHESTRATOR does:** Detailed 6-error recovery section (Part 3-Error-Recovery) showing prevention + recovery steps

**Fix needed:** Add Error Recovery Procedures section (5-6 common mistakes + solutions)

---

### Issue 1.5: Scope Creep Definition Vague [CX-620-variant]

**Problem:**
- Step 6 says "Change files in IN_SCOPE_PATHS only"
- But what if IN_SCOPE_PATHS is ambiguous? (e.g., "src/backend/handshake_core/")
- What if Coder encounters a related bug while implementing?
- CODER_PROTOCOL Step 6 says "DO NOT: Refactor unrelated code ('drive-by' changes)" but "unrelated" is subjective

**Evidence:**
- CODER_PROTOCOL Step 6 (line 184-185): Prohibits "drive-by" changes but doesn't define scope boundary objectively
- ORCHESTRATOR_PROTOCOL Part 3.5 Field 5: IN_SCOPE_PATHS must be EXACT file paths (not vague directories)
- ORCHESTRATOR_PROTOCOL Part 3.5 Field 6: OUT_OF_SCOPE lists 3-8 items that SOUND related but aren't included

**Impact:** Coder unsure if fixing a related bug is "in scope" or "drive-by"; defaults to skipping improvements

**What ORCHESTRATOR does:** Provides exact IN_SCOPE_PATHS (specific files) + OUT_OF_SCOPE list (what NOT to touch, with reasons)

**Fix needed:** Add clarity: "IN_SCOPE_PATHS defines boundary; anything outside is OUT_OF_SCOPE, period. If you find related work, document in packet NOTES and escalate"

---

### Issue 1.6: Validation Failure Path Unclear [CX-623 gap]

**Problem:**
- Step 7 says "If tests FAIL... Fix issues before claiming done"
- But for MEDIUM/HIGH risk, Coder must also run manual review (Step 8)
- What if:
  - manual review blocks? Do tests take precedence?
  - manual review suggests refactoring that's out of scope?
  - Test passes but manual review blocks?

**Evidence:**
- CODER_PROTOCOL Step 7 (lines 236-246): Shows test failure response
- CODER_PROTOCOL Step 8 (lines 250-284): Shows manual review failure response
- But no decision tree: what if tests pass but manual review blocks?
- VALIDATOR_PROTOCOL line 66: "Coder runs TEST_PLAN; Validator spot-checks" (implies tests are primary)
- VALIDATOR_PROTOCOL line 127: "Every requirement mapped to evidence" (code must work AND meet spec)

**Impact:** Coder confused about validation priority; might skip tests if manual review passes

**What ORCHESTRATOR does:** Not explicitly defined, but Part 8 (Pre-Delegation Checklist) prioritizes:
1. SPEC_ANCHOR correctness (foundation)
2. Scope boundaries (IN/OUT_SCOPE)
3. DONE_MEANS measurability
4. TEST_PLAN completeness

**Fix needed:** Add validation priority decision tree:
1. TEST_PLAN must PASS (primary)
2. manual review must PASS or WARN (secondary; BLOCK = fix code)
3. Both passing = work is done

---

### Issue 1.7: Missing "Scope Conflict" Resolution Procedure [CX-620-variant]

**Problem:**
- Step 6 says follow packet scope strictly
- But what if Coder finds the scope is impossible/incomplete?
- Example: Packet says "add endpoint" but doesn't mention required migration
- CODER_PROTOCOL has no "escalate incomplete scope" procedure

**Evidence:**
- CODER_PROTOCOL Step 1-5: Verify packet exists and is complete
- But Step 2 completeness check is weak (just list field names, no objective criteria)
- VALIDATOR_PROTOCOL line 108-110: Escalates spec mismatches to Orchestrator
- ORCHESTRATOR_PROTOCOL Part 4 Step 1: Has detailed scope verification checklist

**Impact:** Coder either (a) adds work out of scope, or (b) stops and requests help without clear escalation message

**What ORCHESTRATOR does:** Part 4 Step 1 verifies understanding, requirements clarity, scope boundaries before creating packet

**Fix needed:** Add Step 1.5 "Scope Adequacy Check" — if scope seems incomplete, escalate to Orchestrator before proceeding

---

### Issue 1.8: VALIDATION Block Format Inconsistent [CX-623 variant]

**Problem:**
- Step 9 says "Append a VALIDATION block to task packet"
- But VALIDATION block format in Step 7 (lines 218-234) is NOT the same format shown in Step 11 (lines 331-376)
- Step 7 shows detailed command output (including failure scenarios)
- Step 11 shows summary format (✅/❌ bullets)

**Evidence:**
- CODER_PROTOCOL Step 7 VALIDATION block (lines 218-234): Detailed command-by-command format
- CODER_PROTOCOL Step 11 final summary (lines 331-376): Bullet-point format
- Should be ONE consistent format; unclear which Validator expects

**Impact:** Coder confused about what format to use; packet audit trails inconsistent

**What ORCHESTRATOR does:** Not explicitly defined in ORCHESTRATOR_PROTOCOL (focus is on packet structure)

**Fix needed:** Define single VALIDATION block format and use consistently throughout Step 7-11

---

### Issue 1.9: DONE_MEANS Verification Procedure Missing [CX-606 gap]

**Problem:**
- CODER_PROTOCOL says "Follow DONE_MEANS criteria" (Step 6)
- But no explicit procedure for verifying DONE_MEANS before claiming done
- Step 11 mentions "DONE_MEANS MET" (line 350-354) but doesn't define how Coder verifies each item

**Evidence:**
- CODER_PROTOCOL Step 6 (line 175): "Follow DONE_MEANS criteria"
- CODER_PROTOCOL Step 11 (line 350-354): "DONE_MEANS MET: ✅ {Criterion 1}..." but doesn't say HOW to verify
- ORCHESTRATOR_PROTOCOL Part 3.5 Field 8 (590 lines): Defines DONE_MEANS 1:1 mapping to SPEC_ANCHOR
- VALIDATOR_PROTOCOL line 31-34: "For each requirement, locate implementation with file path + line number"

**Impact:** Coder claims "done" without proving each DONE_MEANS criterion is met; Validator must redo work

**What ORCHESTRATOR does:** Field 8 example (lines 611-620): Shows exact DONE_MEANS with file:line references

**Fix needed:** Add Step 6.5 "DONE_MEANS Verification" — for each criterion, cite file:line evidence

---

### Issue 1.10: No Escalation Template [CX-635 variant]

**Problem:**
- CODER_PROTOCOL has "If Blocked" section but no escalation TEMPLATE
- Coder blocks, but message format varies
- Example: "work can't proceed" vs "missing requirement" vs "test failure"
- VALIDATOR_PROTOCOL defines escalation protocol but Coder has no equivalent

**Evidence:**
- CODER_PROTOCOL "If Blocked" (lines 403-451): Shows 3 responses but no template
- ORCHESTRATOR_PROTOCOL Part 7.3 (lines 1939-1962): Has universal SLA escalation template
- VALIDATOR_PROTOCOL line 107-111: Defines escalation paths but format unclear

**Impact:** When Coder blocks, escalation message is ad-hoc; unclear what action is needed

**What ORCHESTRATOR does:** Universal escalation template (Part 7.3, lines 1943-1962):
- Work Packet + description
- Status + duration
- Current state + blocker
- Action needed + response deadline

**Fix needed:** Add Coder Escalation Template (similar structure to Orchestrator)

---

## HIGH-PRIORITY ISSUES (Impact validation + code quality)

### Issue 2.1: Manual Review Severity Unclear [CX-573A ambiguity]

**Problem:**
- Step 8 shows PASS/WARN/BLOCK outcomes
- But no objective criteria for what triggers each level
- WARN example (lines 268-272): "Warnings are acceptable" — but all warnings?
- Orchestrator/Validator may have different expectations

**Evidence:**
- CODER_PROTOCOL Step 8 (lines 250-284): Shows outcomes but no criteria
- VALIDATOR_PROTOCOL line 66-68: Mentions test coverage enforcement but not manual review severity
- No link between manual review output and code quality standards

**Impact:** Coder might accept partial WARN (some issues unaddressed); Validator expects stricter standards

**Fix needed:** Add Manual Review Severity Matrix:
- PASS: All checks OK
- WARN: Minor issues (deprecated patterns, test coverage <80%), acceptable to continue
- BLOCK: Critical issues (security, spec violation, hollow code), must fix before proceeding

---

### Issue 2.2: Task Packet Update Procedure Vague [CX-585 variant]

**Problem:**
- Step 9 says "Append a VALIDATION block to task packet"
- Step 11 says "Update task packet with VALIDATION + current status"
- But HOW to update? Do we edit the file directly? Create a new version?
- What if packet is USER_SIGNATURE locked?

**Evidence:**
- CODER_PROTOCOL Step 9 (lines 288-292): Says append VALIDATION, update status
- CODER_PROTOCOL Step 11 (lines 324-376): Shows final summary but doesn't explain file updates
- ORCHESTRATOR_PROTOCOL Part 5.6 (lines 1380-1404): Explains packet locking via USER_SIGNATURE
- Locked packets are IMMUTABLE by design

**Impact:** Coder doesn't know if appending to locked packet is allowed; confusion about governance

**What ORCHESTRATOR does:** Part 5.6 shows packets are locked with USER_SIGNATURE → immutable → create variant if changes needed

**Fix needed:** Clarify: "Coder APPENDS VALIDATION to packet (doesn't edit existing content); no other changes. If status change needed: Orchestrator creates new variant packet"

---

### Issue 2.3: Rollback Verification Missing [CX-607 gap]

**Problem:**
- Task packet includes ROLLBACK_HINT (how to undo work)
- But Coder never verifies ROLLBACK_HINT is accurate
- What if rollback procedure fails? Code is stuck.

**Evidence:**
- CODER_PROTOCOL Step 6 (line 175): "Follow DONE_MEANS criteria"
- No mention of testing rollback procedure
- ORCHESTRATOR_PROTOCOL Part 3.5 Field 9 (lines 629-673): Defines ROLLBACK_HINT but doesn't say Coder should test it
- VALIDATOR_PROTOCOL: No mention of rollback verification

**Impact:** If work needs rollback, procedure might fail; causes extended downtime

**What ORCHESTRATOR does:** Not explicitly, but Field 9 (ROLLBACK_HINT) is Coder-provided after work is done

**Fix needed:** Add Step 6.5 "Rollback Verification": After tests pass, confirm rollback procedure works (git revert succeeds, etc.)

---

### Issue 2.4: Hard Invariant Enforcement Gaps [CX-101-106 partial]

**Problem:**
- Step 6 mentions hard invariants (lines 188-193) but references them vaguely
- Example: [CX-101] "LLM calls through `/src/backend/llm/` only"
- What if Coder encounters existing code violating [CX-102]?
- Should Coder fix it or report it?

**Evidence:**
- CODER_PROTOCOL Step 6 (line 188-193): Lists 4 hard invariants by reference
- No explanation of what each invariant means or how to check compliance
- VALIDATOR_PROTOCOL line 44-52: Detailed hygiene audit with grep patterns
- ORCHESTRATOR_PROTOCOL: No hard invariant section (delegates to Coder/Validator)

**Impact:** Coder unsure which invariants to enforce in their own code vs. existing code

**What ORCHESTRATOR does:** References [CX-101-106] but doesn't define them (they're codex rules, not orchestrator responsibility)

**Fix needed:** Add Hard Invariant Enforcement Guide with:
- What each invariant means (CX-101, CX-102, CX-104, CX-599A)
- How to check compliance (grep patterns, file patterns)
- When to enforce (all new code, not refactoring existing)
- When to escalate (existing code violations)

---

### Issue 2.5: Post-Work Checklist Order Confusing [CX-623-sequence]

**Problem:**
- Step 7: Run validation, document results
- Step 8: manual review
- Step 9: Update packet
- Step 10: `just post-work` check
- Step 11: Update board + request commit
- Order suggests `just post-work` is final check, but it runs AFTER packet update
- Unclear if `just post-work` should run before or after packet update

**Evidence:**
- CODER_PROTOCOL Steps 7-11 (lines 200-379): Sequential but interdependency unclear
- Step 10 runs `just post-work` but Step 9 already updated packet
- Step 11 then updates task board

**Impact:** If `just post-work` fails after packet update, Coder confused about whether to re-update packet

**What ORCHESTRATOR does:** Part 4 Step 4 (Verification) runs `just pre-work` AFTER packet creation, before delegation

**Fix needed:** Clarify sequence:
1. Tests + manual review pass (Step 7-8)
2. Update packet with VALIDATION (Step 9)
3. Run `just post-work` (Step 10)
4. If post-work fails: fix issues, re-test, re-update packet
5. If post-work passes: update board, request commit (Step 11)

---

### Issue 2.6: Missing Packet Variance Procedure [CX-580 variant]

**Problem:**
- What if during implementation, Coder discovers scope should change?
- CODER_PROTOCOL doesn't mention: create variant packet (WP-{ID}-v2)
- Coder either (a) ignores scope, or (b) stops and asks for new packet

**Evidence:**
- CODER_PROTOCOL: No guidance on handling scope changes mid-work
- ORCHESTRATOR_PROTOCOL Part 3-Error-Recovery Error 2 (lines 400-448): Shows how to create variant packets
- ORCHESTRATOR_PROTOCOL Part 5.6 (lines 1400-1404): Explains variant naming (WP-{ID}-v2)

**Impact:** Coder scope-creeps or stalls; no clear path forward if packet is incomplete

**What ORCHESTRATOR does:** Part 3-Error-Recovery shows variant creation + TASK_BOARD update

**Fix needed:** Add Step 1.5 "Scope Adequacy": If scope seems incomplete/conflicting, escalate to Orchestrator (who creates WP-{ID}-v2)

---

### Issue 2.7: Test Coverage Minimum Undefined [CX-623-coverage gap]

**Problem:**
- Step 7 says "Run ALL commands from TEST_PLAN"
- But TEST_PLAN might not include coverage metrics
- VALIDATOR_PROTOCOL line 67-68 requires "at least one targeted test that fails if new logic is removed"
- But CODER_PROTOCOL doesn't define what Coder should test

**Evidence:**
- CODER_PROTOCOL Step 7: Just runs commands, doesn't check coverage %
- VALIDATOR_PROTOCOL line 67-68: "coverage enforcement: require at least one targeted test that fails if the new logic is removed"
- ORCHESTRATOR_PROTOCOL Part 3.5 Field 7: Defines TEST_PLAN completeness but not coverage targets

**Impact:** Coder runs tests but doesn't verify new code is actually being tested; Validator catches gaps

**What ORCHESTRATOR does:** Field 7 (TEST_PLAN) shows exact command examples including test patterns

**Fix needed:** Add Test Coverage Checklist to Step 7:
- For each new function/feature, is there a test that would fail if removed?
- Coverage target: >80% for phase
- Gap: if coverage <80%, add tests or document waiver

---

### Issue 2.8: Logger Entry Policy Unclear [CX-650 variant]

**Problem:**
- Step 9 says "Logger entry is OPTIONAL and only used if explicitly requested for a milestone or hard bug"
- But what qualifies as "milestone" or "hard bug"?
- CODER_PROTOCOL doesn't define criteria

**Evidence:**
- CODER_PROTOCOL Step 9 (line 292): Optional logger entry, no criteria
- ORCHESTRATOR_PROTOCOL: Logger is not orchestrator responsibility (focuses on task packets)
- VALIDATOR_PROTOCOL: No logger guidance

**Impact:** Coder inconsistently logs work; hard to trace which tasks caused major changes

**What ORCHESTRATOR does:** Part 4 Step 3 (line 963): "Logger entries reserved for work completion MILESTONES or critical blockers"

**Fix needed:** Define logger entry criteria:
- Milestone: Phase closure, major feature completion, architecture change
- Hard bug: Security issue, data corruption, critical failure
- Normal work: No logger entry (task packet is SSOT)

---

## MEDIUM-PRIORITY ISSUES (Clarity + usability)

### Issue 3.1: BOOTSTRAP Example Too Specific [CX-577]

**Problem:**
- BOOTSTRAP example (lines 127-159) hardcodes file paths for storage work
- Coder might copy it verbatim for unrelated tasks
- Template should be abstract

**Evidence:**
- CODER_PROTOCOL Step 5 (lines 127-159): Example mentions "Database trait", "AppState", specific to storage work
- Should show generic template structure instead

**Fix needed:** Show abstract template with `{...}` placeholders

---

### Issue 3.2: Validation Block Example Outdated [CX-623]

**Problem:**
- Step 7 VALIDATION example (lines 218-234) references specific commands (cargo test, pnpm test)
- But newer tasks might use different test patterns
- Example is brittle

**Evidence:**
- CODER_PROTOCOL Step 7 (lines 218-234): Hardcoded examples

**Fix needed:** Show generic template with `{command}`, `{result}` placeholders

---

### Issue 3.3: No Mention of Work-in-Progress State [CX-572 variant]

**Problem:**
- CODER_PROTOCOL assumes work is either "Done" or "Blocked"
- But intermediate state (partial progress) isn't mentioned
- What if Coder wants to commit partial progress?

**Evidence:**
- CODER_PROTOCOL Step 11: Final summary assumes all work is done or blocked
- No "In Progress" state for multi-day tasks

**Impact:** Coder unsure how to checkpoint progress; might lose work

**What ORCHESTRATOR does:** TASK_BOARD tracks "In Progress" status (Part 6.2, line 1673)

**Fix needed:** Document: "After significant progress, update task packet STATUS to 'In-Progress' and commit intermediate work with WIP notation"

---

### Issue 3.4: No Merge/Branch Guidance [CX-620 variant]

**Problem:**
- CODER_PROTOCOL talks about validation and commit but not branching strategy
- Should Coder create feature branch or work on main?
- No guidance on PR/review process

**Evidence:**
- CODER_PROTOCOL: Ends at "request commit" (line 376), no merge/branch strategy
- ORCHESTRATOR_PROTOCOL: No merge guidance either

**Impact:** Coder unclear on branching strategy; might commit directly to main

**Fix needed:** Add brief guidance:
- Create feature branch: `git checkout -b feat/WP-{ID}-{name}`
- Commit with WP-ID reference
- Request merge via PR (user/Validator approval)

---

### Issue 3.5: Placeholder References Inconsistent [CX-620 variant]

**Problem:**
- Placeholders in examples use different styles:
  - `{...}` (curly braces) in Step 5 (line 132-155)
  - `{{...}}` or bare text in other steps
  - Inconsistent

**Evidence:**
- CODER_PROTOCOL Step 5: `WP_ID: WP-{phase}-{name}`
- Step 7: `Command: cargo test --manifest-path {path}`
- Step 11: `WP_ID: WP-{phase}-{name}`

**Fix needed:** Use consistent placeholder style: `{placeholder}` everywhere

---

### Issue 3.6: Success Criteria Metrics Vague [CX-572]

**Problem:**
- Success Criteria section (line 528-544) says "You succeeded if" but metrics are subjective
- Example: "Coder asks 'what should I do?'" — does this include clarifying questions?

**Evidence:**
- CODER_PROTOCOL (lines 528-544): Listed 8 success criteria, some subjective

**Fix needed:** Make each criterion measurable:
- ✅ Task packet verified before coding
- ✅ BOOTSTRAP block output (all 4 fields present)
- ✅ Implementation within IN_SCOPE_PATHS
- ✅ All TEST_PLAN commands pass
- ✅ manual review PASS or WARN (not BLOCK)
- ✅ `just post-work` passes
- ✅ Task packet updated with VALIDATION
- ✅ Commit message includes WP-ID

---

### Issue 3.7: BLOCKING RULES Tone Inconsistent [CX-620]

**Problem:**
- DO NOT section (line 383-390) uses strong language ("DO NOT")
- DO section (line 392-399) uses encouraging tone ("DO")
- Both should be equally prescriptive

**Evidence:**
- CODER_PROTOCOL (lines 381-400): Uneven tone

**Fix needed:** Make both sections equally prescriptive and symmetrical

---

### Issue 3.8: No Reference to Orchestrator/Validator Protocols [CX-600-650 ecosystem]

**Problem:**
- CODER_PROTOCOL mentions codex rules [CX-620-623] but doesn't link to:
  - ORCHESTRATOR_PROTOCOL (what Orchestrator must provide)
  - VALIDATOR_PROTOCOL (what Validator will check)
- Coder isolated from ecosystem context

**Evidence:**
- CODER_PROTOCOL references codex but not sister protocols
- ORCHESTRATOR_PROTOCOL Part 3.5 links to CODER_PROTOCOL [CX-620-623]
- VALIDATOR_PROTOCOL references both but Coder doesn't know Validator expectations

**Impact:** Coder doesn't understand ecosystem; writes code that Validator must redo

**What ORCHESTRATOR does:** Part 3.5 (line 332) explicitly says: "CODER_PROTOCOL [CX-620-623] defines 11 steps that Coder MUST follow"

**Fix needed:** Add introduction linking CODER_PROTOCOL to ORCHESTRATOR_PROTOCOL (what to expect in task packets) and VALIDATOR_PROTOCOL (what Validator will check)

---

## SUMMARY: Grade Assessment

| Component | Current | Target | Gap |
|-----------|---------|--------|-----|
| **Completeness** | 75% | 95% | Missing error recovery, escalation templates, edge cases |
| **Clarity** | 70% | 95% | Subjective criteria, undefined placeholders, vague procedures |
| **Objective Criteria** | 60% | 95% | BOOTSTRAP format, DONE_MEANS verification, validation severity |
| **Alignment with Ecosystem** | 65% | 95% | No links to Orchestrator/Validator; isolated protocol |
| **Error Recovery** | 40% | 90% | Only 3 blocking scenarios; missing recovery procedures |
| **Overall Grade** | **B+ (82)** | **A+ (91)** | **9 points to close** |

---

## Recommended Improvement Path

### Phase 1 (P0): Critical Foundations [82 → 88/100] (4-5 hours)
1. Add Packet Completeness Checklist (objective criteria for each field) — 45 min
2. Add BOOTSTRAP Completeness Checklist (4 sub-fields with minimums) — 30 min
3. Add Error Recovery Procedures (5-6 common mistakes + solutions) — 1 hour
4. Add Validation Severity Matrix (PASS/WARN/BLOCK criteria) — 30 min
5. Add Coder Escalation Template — 30 min

### Phase 2 (P1): Quality Systems [88 → 93/100] (2-3 hours)
1. Add Hard Invariant Enforcement Guide (what each means, how to check) — 45 min
2. Add Test Coverage Checklist (minimum coverage % per risk tier) — 30 min
3. Add Scope Adequacy Procedure (when to escalate incomplete scope) — 30 min

### Phase 3 (P2): Polish [93 → 99/100] (2-3 hours)
1. Add Branching/Merge Guidance — 20 min
2. Clarify Step Sequences (post-work ordering) — 30 min
3. Update examples to use consistent placeholders — 30 min
4. Add Protocol Ecosystem Links — 30 min

---

## Critical Gaps Preventing Coder Excellence

1. **Packet Completeness Subjectivity:** Coder accepts incomplete packets → validation rework
2. **Missing Error Recovery:** When tests fail/AI blocks, Coder has no debug playbook
3. **Scope Ambiguity:** Unclear boundaries lead to scope creep or stalling
4. **Isolated Protocol:** Coder doesn't know Orchestrator expectations or Validator rules
5. **Validation Ambiguity:** Unclear priority (tests vs. manual review) leads to partial fixes
6. **Missing Escalation Template:** Blocks are ad-hoc; unclear what action needed

---

**Analysis Complete**

This scrutiny reveals CODER_PROTOCOL is functional (B+ grade) but lacks the precision needed for A+ performance. Main improvements: objective completeness criteria, error recovery, ecosystem links, and escalation procedures.
