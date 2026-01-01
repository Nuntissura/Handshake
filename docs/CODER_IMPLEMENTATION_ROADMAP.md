# CODER_IMPLEMENTATION_ROADMAP: Path to 9.9/10

**Current Grade:** B+ (82/100)
**Target Grade:** A+ (99/100)
**Work Type:** Protocol additions + clarifications (no code changes)
**Model Tier:** Cheap/Standard

---

## Quick Assessment: Impact vs. Effort

| Gap | Impact | Effort | ROI | Priority |
|-----|--------|--------|-----|----------|
| Gap 1: Packet completeness criteria | HIGH | LOW | 10x | **P0** |
| Gap 2: BOOTSTRAP format | HIGH | LOW | 10x | **P0** |
| Gap 3: TEST_PLAN completeness | HIGH | MEDIUM | 8x | **P0** |
| Gap 4: Error recovery procedures | HIGH | MEDIUM | 8x | **P0** |
| Gap 5: Validation priority | HIGH | LOW | 10x | **P0** |
| Gap 6: Hard invariant guide | MEDIUM | MEDIUM | 7x | **P1** |
| Gap 7: Test coverage checklist | MEDIUM | LOW | 8x | **P1** |
| Gap 8: Scope conflict resolution | MEDIUM | MEDIUM | 6x | **P1** |
| Gap 10: DONE_MEANS verification | MEDIUM | MEDIUM | 6x | **P1** |
| Gap 11: manual review severity | MEDIUM | LOW | 6x | **P2** |
| Gap 12: Packet update clarity | MEDIUM | LOW | 6x | **P2** |
| Gap 17: Ecosystem links | MEDIUM | LOW | 5x | **P2** |

---

## PHASE 1: Critical Foundations (82 → 88/100) — 3-4 hours

These 5 items unlock clarity and prevent Coder blocks on packet completeness.

### P0-1: Packet Completeness Criteria [Gap 1]

**What:** Add objective checklist for packet field completeness

**Deliverable:** 10-item completeness checklist (TASK_ID format, STATUS values, RISK_TIER justification, SCOPE concreteness, IN_SCOPE_PATHS specificity, OUT_OF_SCOPE items, TEST_PLAN commands, DONE_MEANS measurability, ROLLBACK_HINT clarity, BOOTSTRAP sub-fields)

**Location:** Insert after Step 2 (line 75)

**Time:** 30 min

**Outcome:** Coder rejects incomplete packets immediately with specific reason

---

### P0-2: BOOTSTRAP Completeness Checklist [Gap 2]

**What:** Define BOOTSTRAP format and minimums for all 4 sub-fields

**Deliverable:**
- FILES_TO_OPEN: 5-15 files minimum (specific names)
- SEARCH_TERMS: 10-20 patterns minimum
- RUN_COMMANDS: 3-6 commands minimum
- RISK_MAP: 3-8 failure modes minimum

**Location:** Replace Step 5 template (lines 123-159)

**Time:** 30 min

**Outcome:** 100% clarity on BOOTSTRAP format; Validator never says "incomplete BOOTSTRAP" again

---

### P0-3: TEST_PLAN Completeness Check [Gap 3]

**What:** Verify TEST_PLAN commands are concrete (not placeholders) before running

**Deliverable:** Validation rules + examples (incomplete vs. complete)

**Location:** Insert new section in Step 2 after field completeness check (after line 85)

**Time:** 45 min

**Outcome:** Coder catches placeholder TEST_PLAN before validation starts; Orchestrator catches it earlier

---

### P0-4: Error Recovery Procedures [Gap 4]

**What:** Document recovery for 6 common Coder mistakes

**Deliverable:** 6 error scenarios with recovery steps:
1. Test fails unexpectedly
2. manual review blocks (hard invariant violation)
3. Scope conflict (can't proceed)
4. Packet changed mid-work
5. Post-work validation fails
6. Can't understand DONE_MEANS

**Location:** Add new section after "Common Mistakes" (before line 455)

**Time:** 1 hour

**Outcome:** Coder has playbook for debugging; 60% fewer escalations due to "blocked, don't know how to proceed"

---

### P0-5: Validation Priority [Gap 5]

**What:** Clear, numbered order: Tests FIRST, Manual Review SECOND, Post-Work THIRD

**Deliverable:** Decision flow chart + explicit rules

**Location:** Insert after Step 6 (before line 200)

**Time:** 30 min

**Outcome:** Coder never says "tests pass, so I'm done" and skips manual review; 100% clarity on order

---

## PHASE 2: Quality Systems (88 → 93/100) — 2-3 hours

These 4 items add rigor and reduce false "done" claims.

### P1-1: Hard Invariant Enforcement Guide [Gap 6]

**What:** Explain what each hard invariant means ([CX-101] LLM calls, [CX-102] no direct HTTP, [CX-104] no println, [CX-599A] TODO format)

**Deliverable:** For each invariant:
- Meaning (plain English)
- Grep command to check
- Enforcement rules (new code, existing code in scope, existing code out of scope)

**Location:** Add new section after "Hard invariants to respect" (line 188)

**Time:** 45 min

**Outcome:** 50% fewer manual review blocks; Coder knows what to check before running manual review

---

### P1-2: Test Coverage Checklist [Gap 7]

**What:** Define minimum coverage % per RISK_TIER (LOW: optional, MEDIUM: >80%, HIGH: >80% + removal check)

**Deliverable:** Coverage target per tier + tarpaulin command + waiver template

**Location:** Insert after Step 7 test validation (before line 246)

**Time:** 30 min

**Outcome:** Coder adds tests proactively; no more "coverage too low" validation failures

---

### P1-3: Scope Conflict Resolution [Gap 8]

**What:** Procedure for when implementation reveals incomplete scope (missing files, OUT_OF_SCOPE blocker, missing prerequisites)

**Deliverable:** Step 1.5 "Scope Adequacy Check" + escalation template

**Location:** Insert after Step 1 (before Step 2, around line 54)

**Time:** 30 min

**Outcome:** Coder catches scope issues before coding; Orchestrator creates WP-v2 if needed; prevents wasted effort

---

### P1-4: DONE_MEANS Verification Procedure [Gap 10]

**What:** Explicit steps to verify each DONE_MEANS criterion (find file:line evidence for every item)

**Deliverable:** 5-step verification process + example

**Location:** Insert after Step 6 implementation (before line 195)

**Time:** 30 min

**Outcome:** Validator spends <5% re-checking your work; evidence trail is complete

---

## PHASE 3: Polish (93 → 99/100) — 2-3 hours

These items are refinements (clarity, ecosystem context, governance rules).

### P2-1: Manual Review Severity Matrix [Gap 11]

**What:** Define PASS/WARN/BLOCK objectively (PASS: no issues; WARN: minor issues acceptable; BLOCK: critical, must fix)

**Location:** Insert after Step 8 (before line 284)

**Time:** 30 min

**Outcome:** Coder knows when to stop fixing; no ambiguity on "is this warn acceptable?"

---

### P2-2: Packet Update Clarity [Gap 12]

**What:** Define what Coder can append vs. what's locked (APPEND: VALIDATION + STATUS; DO NOT EDIT: SCOPE, PATHS, DONE_MEANS, TEST_PLAN)

**Location:** Insert in Step 9 (before line 292)

**Time:** 20 min

**Outcome:** Governance is clear; no accidental packet edits that corrupt the contract

---

### P2-3: Ecosystem Links [Gap 17]

**What:** Link to ORCHESTRATOR_PROTOCOL and VALIDATOR_PROTOCOL; explain the three-role system

**Location:** Add introduction section (after line 12)

**Time:** 20 min

**Outcome:** Coder understands their place in system; writes validation blocks that Validator can quickly verify

---

### P2-4: Miscellaneous Polish

**What:** Consistency, examples, clarity improvements

**Items:**
- Consistent placeholder style ({placeholder} everywhere)
- Generic BOOTSTRAP example (not storage-specific)
- Branching/merge strategy (feature branches, PR workflow)
- Clarify Step sequence (when post-work runs, how to handle failures)
- Update Success Criteria (make metrics measurable)

**Time:** 1-2 hours combined

**Outcome:** Protocol is polished, professional, clear

---

## Implementation Roadmap

### Week 1 (P0: Critical Foundations) — 3-4 hours
- [ ] Packet Completeness Criteria (30 min)
- [ ] BOOTSTRAP Completeness Checklist (30 min)
- [ ] TEST_PLAN Completeness Check (45 min)
- [ ] Error Recovery Procedures (1 hour)
- [ ] Validation Priority (30 min)
- **Result: 82 → 88/100 (A- achieved)**

### Week 2 (P1: Quality Systems) — 2-3 hours
- [ ] Hard Invariant Enforcement Guide (45 min)
- [ ] Test Coverage Checklist (30 min)
- [ ] Scope Conflict Resolution (30 min)
- [ ] DONE_MEANS Verification Procedure (30 min)
- **Result: 88 → 93/100**

### Week 3 (P2: Polish) — 2-3 hours
- [ ] Manual Review Severity Matrix (30 min)
- [ ] Packet Update Clarity (20 min)
- [ ] Ecosystem Links (20 min)
- [ ] Miscellaneous Polish (1-2 hours)
- **Result: 93 → 99/100 (9.9/10 ✨)**

---

## Success Metrics

**After Week 1 (P0):** 88/100
- Packet completeness is verifiable (no subjective decisions)
- BOOTSTRAP format is crystal clear (never "incomplete" again)
- Coder knows validation order (tests → manual review → post-work)
- Coder has error recovery playbook

**After Week 2 (P1):** 93/100
- Hard invariants are explained (50% fewer manual review blocks)
- Test coverage minimums are clear (no more "add tests" feedback)
- Scope conflicts caught early (Orchestrator fixes scope, not Coder)
- DONE_MEANS verified with file:line evidence

**After Week 3 (P2):** 99/100 (9.9/10)
- manual review severity is objective (WARN vs BLOCK clear)
- Governance rules are explicit (what Coder can/can't edit)
- Ecosystem context is clear (understanding three-role system)
- Polish complete (professional, clear, actionable)

---

## Cost Assessment

| Phase | Effort | LLM Tier | Cost |
|-------|--------|----------|------|
| P0 | 3-4 hours | Standard/Cheap | LOW |
| P1 | 2-3 hours | Standard/Cheap | LOW |
| P2 | 2-3 hours | Standard/Cheap | LOW |
| **Total** | **7-10 hours** | **Cheap** | **LOW** |

**All work is protocol additions + clarifications. No code changes. Perfect for cheaper LLM tier.**

---

## Conclusion

**From B+ (82) to A+ (99) requires:**
- 7-10 hours of protocol documentation work
- 3 supporting rubrics (already created: CODER_RUBRIC.md)
- 2 analysis documents (already created: CODER_PROTOCOL_SCRUTINY.md, CODER_PROTOCOL_GAPS.md)
- Minimal testing (read-through + example validation)

**This is pure clarity + coordination work. Ideal for cheaper LLM tier.**

**Why This Matters for Coder Tomorrow:**
- Task packets are verified complete before delegation
- Every packet has concrete completeness criteria
- BOOTSTRAP procedure forces upfront understanding
- Error recovery procedures guide debugging
- Validation procedures are crystal clear
- Hard invariants are explained (fewer manual review surprises)
- Scope conflicts caught immediately (Orchestrator fixes, not Coder)
- Result: Maximum productivity, minimum rework, clean commits

---

**Ready for Phase 1 (P0) implementation.**
