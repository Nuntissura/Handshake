# ORCHESTRATOR_PROTOCOL: Remaining Gaps for 9.9/10

**Current Grade:** A- (87/100) after critical issue fixes
**Target Grade:** A+ (99/100)
**Gap Analysis:** What prevents 9.9/10 score

---

## CRITICAL GAPS (Must Fix for A+)

### 🔴 Gap 1: No Automated Compliance Checker
**Problem:** Protocol defines rules but no tool verifies Orchestrator follows them
**Current State:**
- `just pre-work` checks task packet completeness (EXISTS)
- NO check for Pre-Orchestration Checklist execution (5 items)
- NO check for signature verification (grep done manually)
- NO check for blocker status verification (manual grep)
- NO check for TASK_BOARD sync (manual audit)

**Impact:** An Orchestrator could skip 8 of 8 critical gates and delegate anyway

**What's Needed:**
```bash
just orchestrator-compliance WP-{ID}
# Should verify:
- Pre-Orchestration Checklist passed (spec current, supply chain clean, etc.)
- Signature verification done (grep confirmed one-time use)
- Blocker status checked (TASK_BOARD consulted)
- TASK_BOARD synced with packet
- All 14 Pre-Delegation items checked
```

**Effort:** Create validation script checking:
1. SPEC_CURRENT.md points to latest spec
2. Signature not used before (grep -r)
3. Blocker status in TASK_BOARD current
4. Packet status = TASK_BOARD status

---

### 🔴 Gap 2: "Clearly Covers" Undefined for Spec Enrichment
**Problem:** Decision tree says "Does Master Spec Main Body clearly cover this?" but "clearly" is undefined
**Current State:**
```
Decision Tree says: "Does Master Spec Main Body clearly cover this requirement?"
├─ YES → Proceed
└─ NO → Enrich or escalate

But "clearly" is subjective. Examples of ambiguity:
- Spec says "async methods" — does this clearly cover cancellable operations? 🤔
- Spec says "Database trait" — does this clearly cover error handling? 🤔
- Spec has 2 sections partially covering requirement — is it "clear"? 🤔
```

**Impact:** Orchestrators will make different decisions based on interpretation

**What's Needed:**
```
DEFINITION: "Clearly covers" means ALL of the following:
1. Requirement appears in Main Body (not Roadmap)
2. Section explicitly names the requirement
3. MUST/SHOULD statements are specific (not generic)
4. Acceptance criteria are measurable
5. No ambiguity remains (single interpretation possible)

Example of "clearly covers":
§2.3.12.1 says: "Database trait MUST have async fn get_blocks(&self, id: &str)"
→ Clear: specific method name, signature, behavior

Example of "does not clearly cover":
§2.3.12 says: "Storage abstraction SHOULD be portable"
→ Not clear: vague, needs interpretation, multiple valid implementations
```

**Effort:** Add objective 5-point checklist to Part 2.5.1 decision tree

---

### 🔴 Gap 3: No Error Recovery Procedures
**Problem:** If Orchestrator makes mistake (wrong signature, wrong SPEC_ANCHOR, etc.), no documented recovery
**Current State:**
```
Mistake: Orchestrator uses signature twice (typo)
Response: ❌ "Oops, can't undo. We have a governance violation."

Mistake: Orchestrator creates packet with wrong SPEC_ANCHOR (realizes after handoff)
Response: ❌ "Packet is locked. Must create v2. Wasted time."

Mistake: Orchestrator forgets to update TASK_BOARD
Response: ❌ "Status out of sync. Validator confused."
```

**Impact:** Mistakes are permanent; no recovery path

**What's Needed:**
```
Error Recovery Procedures:
1. Signature used twice
   → Immediate: Note in SIGNATURE_AUDIT.md that sig is INVALID
   → Request new signature from user
   → Update WP with new signature reference

2. Wrong SPEC_ANCHOR in packet
   → If not yet delegated: Edit before handing off (unlock, fix, re-lock)
   → If already delegated: Create ERRATA section (read-only addition)
      "ERRATA: Original SPEC_ANCHOR §X was incorrect; should be §Y"
   → Create variant WP with correct SPEC_ANCHOR

3. TASK_BOARD out of sync
   → Immediately reconcile (compare TASK_BOARD vs. all packet STATUS)
   → Document discrepancy (why did it drift?)
   → Add to "Lessons Learned" log

4. Blocker status missed
   → Immediately mark dependent WP as BLOCKED
   → Document "discovered after creation; should have been caught in Step 1"
   → Review your Pre-Orchestration checklist execution
```

**Effort:** Add 2-page "Error Recovery" section to protocol

---

### 🔴 Gap 4: Phase Progression Criteria Undefined
**Problem:** Protocol says "phase closes when all WPs VALIDATED" but doesn't define "ready to close"
**Current State:**
```
Part 1 says: "Phase only closes when ALL WPs are VALIDATED"
Part 6.4 says: "Closure criteria: all WPs validated + spec clean + no blockers"

But what about:
- Performance regressions?
- User satisfaction with phase outcome?
- Technical debt accumulated?
- Security audit results?
- Are these criteria or just "nice to have"?
```

**Impact:** Ambiguity on when phase is actually ready to ship

**What's Needed:**
```
Phase Closure Gate (Explicit):
MUST criteria (blocking):
- [ ] All phase-critical WPs are VALIDATED (verdict: PASS)
- [ ] No unresolved blockers remain
- [ ] `just validator-spec-regression` returns PASS
- [ ] Supply chain clean (cargo deny + npm audit)
- [ ] All git commits signed (audit trail complete)

SHOULD criteria (warning only):
- [ ] No open escalations from Validator
- [ ] No deferred work notes in WPs
- [ ] Coverage metrics on target (>80% for phase WPs)

Example closure checklist for Phase 1:
Phase 1 Closure Gate:
- [ ] WP-1-Storage-Abstraction-Layer: VALIDATED ✅
- [ ] WP-1-AppState-Refactoring: VALIDATED ✅
- [ ] WP-1-Migration-Framework: VALIDATED ✅
- [ ] WP-1-Dual-Backend-Tests: VALIDATED ✅
- [ ] Spec regression: PASS
- [ ] Supply chain: PASS
- [ ] No escalations pending
→ Phase 1 READY TO CLOSE
```

**Effort:** Add Phase Closure Criteria section (1 page)

---

### 🔴 Gap 5: No SLA for Critical Gates
**Problem:** Gates exist but no time commitment; Orchestrator could leave WP BLOCKED for months
**Current State:**
```
TASK_BOARD shows: WP-1-AppState-Refactoring [BLOCKED 🟡]
No one checks: "How long has this been blocked?"
No escalation: "This is taking too long"
Result: Work stalls silently
```

**Impact:** No accountability for gate resolution timing

**What's Needed:**
```
SLA for Critical Gate States:
Status     | Max Duration | If Exceeded
-----------|--------------|------------
BLOCKED    | 5 work days  | Escalate to user (blockers taking too long)
READY DEV  | 10 work days | Escalate to Validator (why not started?)
IN PROG    | 30 work days | Flag as risk (estimate too low?)
DONE       | 1 work day   | Orchestrator moves to VALIDATED (after Validator approves)

Example SLA violation:
Day 1: WP-1 marked BLOCKED (waiting for dependency)
Day 8: Still BLOCKED → Orchestrator: "Escalate immediately"
Message: "WP-1-AppState-Refactoring blocked for 8 days (exceeds 5-day SLA).
          Blocker WP-1-Storage-Abstraction-Layer: Status?"
```

**Effort:** Add SLA table + escalation procedure (1 page)

---

### 🔴 Gap 6: No Audit Trail Generation Mechanism
**Problem:** Protocol defines rules but doesn't generate evidence that rules were followed
**Current State:**
```
Orchestrator claims: "I ran Pre-Orchestration Checklist"
Evidence: None (checklist is in protocol, not logged)

Orchestrator claims: "I verified signature one-time use"
Evidence: grep command (not logged anywhere)

Orchestrator claims: "I checked blocker status"
Evidence: None (manual check, no artifact)
```

**Impact:** Validator cannot audit Orchestrator's decisions; must trust blindly

**What's Needed:**
```
Orchestrator Decision Log (template):

---
## Orchestrator Decision Log

### WP-1-Storage-Abstraction-Layer (2025-12-25 14:30)

**Pre-Orchestration Checklist:**
- ✅ SPEC_CURRENT.md: v02.85 (timestamp 2025-12-25 09:00)
- ✅ `just validator-spec-regression`: PASS (2025-12-25 09:15)
- ✅ Supply chain: clean (cargo deny 0 violations)
- ✅ TASK_BOARD: current, no stalled WPs
- ✅ Governance files: ORCHESTRATOR_PROTOCOL (v1.1), CODER_PROTOCOL (v1.1)

**Spec Enrichment Check:**
- Decision: NO enrichment needed
- Reason: User request maps to §2.3.12.3 (Storage API) — already in Main Body
- Verified: SPEC_CURRENT.md contains §2.3.12.3

**Signature Verification:**
- Signature: ilja251225041500
- Verification: grep -r "ilja251225041500" . → returns 1 match (audit log only) ✅
- Timestamp: 2025-12-25 04:15 (user-provided, matches external clock)

**Blocker Check:**
- Dependencies: None (foundational WP)
- TASK_BOARD review: No blockers

**Pre-Delegation Checklist:**
- ✅ All 14 items verified (see detailed checklist below)
- `just pre-work WP-1-Storage-Abstraction-Layer`: PASS

**Handoff:** Ready to delegate
---
```

**Effort:** Create `ORCHESTRATOR_DECISION_LOG.template.md` (1 page)

---

### 🔴 Gap 7: Cross-Protocol Consistency Not Verified
**Problem:** ORCHESTRATOR, CODER, VALIDATOR protocols are separate; no check they align
**Current State:**
```
ORCHESTRATOR_PROTOCOL Part 3.5 says: "Coder verifies all 10 fields in Step 2"
CODER_PROTOCOL Step 2 says: "Verify TASK_ID, STATUS, RISK_TIER, ... (10 items)"
→ Do they match exactly? Unknown (manual audit required)

If ORCHESTRATOR_PROTOCOL is updated:
→ Does someone check CODER_PROTOCOL still aligns? No
→ Do task packets get updated? Unclear
→ Does everyone know protocol changed? No notification
```

**Impact:** Protocols can drift; Coder and Validator might enforce different standards

**What's Needed:**
```
Protocol Consistency Checker:

just orchestrator-check-alignment
# Verifies:
1. ORCHESTRATOR_PROTOCOL Part 3.5 (10 fields) == CODER_PROTOCOL Step 2 (10 fields)
2. ORCHESTRATOR_PROTOCOL Part 8 (14-item checklist) == VALIDATOR_PROTOCOL (checklist items)
3. All CX codes in all three protocols are consistent
4. Status values (READY FOR DEV, etc.) match across all docs
5. Blocker enforcement in ORCHESTRATOR == enforcement in VALIDATOR

Output:
✅ Protocol alignment check PASSED
OR
❌ MISALIGNMENT DETECTED:
   - ORCHESTRATOR says: 10 fields required
   - CODER says: 11 fields required
   Action: Review and align before proceeding
```

**Effort:** Create alignment checking script (medium effort)

---

## HIGH-PRIORITY GAPS (Would improve to A+, 95+)

### 🟠 Gap 8: Packet Variant Management Procedure
**Problem:** Protocol says "create WP-{ID}-v2 for changes" but no management procedure
**Current State:**
```
WP-1-Storage-Abstraction-Layer (original, locked)
WP-1-Storage-Abstraction-Layer-v2 (variant, fixes typo)
WP-1-Storage-Abstraction-Layer-v2-fix (another variant)

Questions:
- Which version is "active"?
- How does Coder know which to use?
- How are old versions archived?
- How does git history show variant lineage?
```

**What's Needed:**
```
Variant Management Rules:
1. Original packet is v1 (no suffix)
2. If changes needed:
   - Create v2 (suffix: -v2, -v3, etc.)
   - Document reason in variant (e.g., "REASON: Fixed typo in IN_SCOPE_PATHS")
   - Orchestrator creates REDIRECT in v1 (readonly addition):
     "SUPERSEDED by WP-1-Storage-Abstraction-Layer-v2"
   - Only v2 is "active" (added to TASK_BOARD)
3. Archive old versions (keep in git, remove from TASK_BOARD)
4. Coder receives link to latest version in handoff message
```

---

### 🟠 Gap 9: Spec Version Mismatch Detection
**Problem:** Task packets reference spec versions; if spec updated but packets not, inconsistency emerges
**Current State:**
```
Task packets created with spec v02.85
Spec updated to v02.86
Packets still reference v02.85
→ Validator confused: "Which spec is authoritative?"
```

**What's Needed:**
```
just orchestrator-check-spec-versions
# Verifies:
- All active task packets record SPEC_BASELINE (provenance) and SPEC_TARGET (closure/revalidation target)
- If SPEC_BASELINE != resolved SPEC_TARGET: flag drift and require explicit re-anchoring decision (do not auto-fail)
- ORCHESTRATOR_PROTOCOL references current spec version
- All protocol docs updated when spec changes
```

---

### 🟠 Gap 10: Dependency Graph Visualization
**Problem:** TASK_BOARD lists blockers but dependency chain not visible
**Current State:**
```
TASK_BOARD shows:
- WP-1: No dependencies
- WP-2: Depends on WP-1
- WP-3: Depends on WP-1
- WP-4: Depends on WP-2

Graph not visible; easy to miss critical path
```

**What's Needed:**
```
Dependency graph visualization in TASK_BOARD:

WP-1-Storage-Abstraction-Layer [VALIDATED ✅]
  ├→ WP-1-AppState-Refactoring [IN PROGRESS 🟠]
  │   └→ WP-1-Dual-Backend-Tests [BLOCKED 🟡]
  └→ WP-1-Migration-Framework [READY FOR DEV 🔴]

Plus: mermaid diagram or text-based ASCII art showing dependency chain
```

---

### 🟠 Gap 11: Explicit Escalation Path & SLA
**Problem:** Protocol says "escalate" but doesn't define to whom or how long to wait
**Current State:**
```
Protocol: "If unclear, escalate to user"
Reality: How do I escalate? Email? Slack? Merge request?
Reality: How long do I wait for response? 1 hour? 24 hours?
```

**What's Needed:**
```
Escalation Path (template):

TO ESCALATE:
1. Create issue in GitHub with label "escalation:orchestrator"
2. Message format:
   Title: "[ESCALATION] {WP-ID}: {Issue}"
   Body: "BLOCKED: {reason}
          OPTION A: {option with implication}
          OPTION B: {option with implication}
          Decision needed by: {date/time}"
3. SLA: Respond within 24 hours or phase is blocked

Escalation types:
- Type 1 (Spec unclear): Needs user decision on interpretation
- Type 2 (Blocker stalled): Blocker exceeded 5-day SLA
- Type 3 (Scope ambiguous): Can't determine IN_SCOPE_PATHS
- Type 4 (Risk tier disputed): Orchestrator and Validator disagree
```

---

### 🟠 Gap 12: Risk Tier Objective Criteria
**Problem:** Protocol defines LOW/MEDIUM/HIGH but criteria are subjective
**Current State:**
```
LOW: "Docs-only, no behavior change"
MEDIUM: "Code change, one module, no migrations"
HIGH: "Cross-module, migrations, IPC, security"

Edge case: Work touches 2 modules but has no migration — MEDIUM or HIGH?
Edge case: Single module but involves security-sensitive code — LOW or HIGH?
```

**What's Needed:**
```
Objective Risk Tier Matrix:

Dimension           | LOW     | MEDIUM   | HIGH
--------------------|---------|----------|----------
Files changed       | 1-5     | 6-15     | >15
Modules affected    | 1       | 2        | >2
Migrations needed   | No      | No       | Yes
IPC changes         | No      | No       | Yes
Security impact     | None    | Low      | High
Test coverage req   | Existing| +new 50% | +new 80%
Manual review      | Optional| Required | Required
Effort hours       | <4      | 4-20     | >20

Rule: Take MAXIMUM tier across all dimensions
Example: 10 files (MEDIUM) + 1 migration (HIGH) → HIGH tier
```

---

## MEDIUM-PRIORITY GAPS (Would improve to 9.0+)

### 🟡 Gap 13: User Context Preservation in Locked Packets
**Problem:** VALIDATOR_PROTOCOL says preserve User Context sections, but ORCHESTRATOR_PROTOCOL doesn't mention this
**Current State:**
```
Validator says: "Preserve User Context sections in packets"
Orchestrator doesn't know: What are User Context sections?
Result: Orchestrator might delete them when locking packet
```

**What's Needed:**
```
Definition: User Context = any notes user provided about work context
- MUST be preserved in locked packet
- MUST be quoted in packet NOTES section
- Example format:
  ## Notes
  **User Context (from request):** "This is blocking customer X; needs to ship by Friday"
  **Orchestrator Notes:** ...
  **Assumptions:** ...
```

---

### 🟡 Gap 14: No Compensation for Minor Mistakes
**Problem:** Locking is all-or-nothing; any typo requires v2
**Current State:**
```
Typo in BOOTSTRAP FILES_TO_OPEN: forgot one file
Result: Packet locked, can't fix
Solution: Create v2 (expensive)
```

**What's Needed:**
```
Error Severity Classification:

CRITICAL (must create v2):
- Wrong SPEC_ANCHOR
- Wrong IN_SCOPE_PATHS (scope change)
- Missing DONE_MEANS (acceptance criteria change)

MINOR (can add ERRATA):
- Typo in BOOTSTRAP FILES_TO_OPEN
- Grammatical error in SCOPE description
- Wrong effort estimate (hours)

ERRATA format (read-only addition to locked packet):
---
## ERRATA
- Line X: Original said "{text}"; should be "{correct text}"
- Reason: Typo (does not change scope or requirements)
---
```

---

### 🟡 Gap 15: Rapid Change Batching Rules
**Problem:** If user makes 5 requests in same day, does each need signature?
**Current State:**
```
User: "Add feature A" → Signature: ilja251225090000
User: "Add feature B" (15 min later) → New signature needed?
Result: Signature fatigue; user frustrated
```

**What's Needed:**
```
Signature Batching Rules:
1. Same enrichment session = one signature covers all specs updated within 2 hours
2. Different sessions = different signatures (prove user intentionality at each decision)
3. Batching example:
   User enrichment request 09:00 → Signature: ilja251225090000
   Covers enrichment for: features A, B, C (all clarified in same session)
   Decision at 11:30 (>2 hours) → New signature: ilja251225113000
```

---

## ASSESSMENT SUMMARY

**Critical Gaps (1-7):** 7 gaps
**Result if fixed:** 91/100 → A+ (9.1/10)

**High-Priority Gaps (8-12):** 5 gaps
**Result if fixed:** 91/100 → 94/100 (9.4/10)

**Medium-Priority Gaps (13-15):** 3 gaps
**Result if fixed:** 94/100 → 97/100 (9.7/10)

**Minor Polish (not listed):** Various
**Result if fixed:** 97/100 → 99/100 (9.9/10)

---

## Recommendations for A+ Protocol (9.9/10)

**Phase 1 (Essential): Fix critical gaps 1-7**
- Add `just orchestrator-compliance` script
- Define "clearly covers" objective criteria
- Document error recovery procedures
- Add Phase Closure Criteria section
- Add SLA table for gate durations
- Create Decision Log template
- Add protocol alignment checker

**Phase 2 (Important): Address high-priority gaps 8-12**
- Add variant management procedure
- Create spec version mismatch detector
- Add dependency graph visualization to TASK_BOARD
- Document escalation path + SLA
- Add objective Risk Tier matrix

**Phase 3 (Nice-to-Have): Polish medium gaps 13-15**
- Clarify User Context preservation
- Add ERRATA section for minor fixes
- Document signature batching rules

**Estimated Effort:**
- Phase 1: 16 hours (research + implementation)
- Phase 2: 12 hours
- Phase 3: 4 hours
- Total: 32 hours to reach 9.9/10

---

**Current State:** A- (87/100)
**Potential State:** A+ (99/100)
**Gap to Close:** 12 points (7 critical + 5 important gaps)

