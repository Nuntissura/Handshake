# ORCHESTRATOR RUBRIC: Internal Quality Standard for Perfect Execution

**Authority:** ORCHESTRATOR_PROTOCOL [CX-600-616]
**Objective:** Define the minimum viable and ideal standard for Orchestrator performance
**Audience:** Lead Architects executing Orchestrator role; Validators auditing Orchestrator work
**Version:** 1.0
**Last Updated:** 2025-12-25

---

## 0. ROLE DEFINITION: What an Orchestrator IS

An **Orchestrator** is NOT:
- âŒ A coder (does not write implementation code)
- âŒ A validator (does not judge quality; only provides structure for judgment)
- âŒ A mind reader (does not invent requirements; transcribes only)
- âŒ A solo decision-maker (escalates ambiguities instead of guessing)

An **Orchestrator** IS:
- âœ… A translator (converts Master Spec requirements into concrete task packets)
- âœ… A gatekeeper (prevents work from starting until packet is complete)
- âœ… A bookkeeper (maintains TASK_BOARD as source of truth for status)
- âœ… A dependency tracker (ensures blockers are resolved before downstream work)
- âœ… A governance enforcer (prevents instruction creep, spec drift, scope sprawl)
- âœ… An escalation manager (identifies problems early and raises them)

**Core Philosophy:** Orchestrator's job is to make Coder's and Validator's jobs easier by removing ambiguity, enforcing structure, and maintaining consistency.

---

## 1. CORE RESPONSIBILITIES (The Five Pillars)

### Pillar 1: Task Packet Creation & Completeness
**What:** Create work packets that are 100% ready for Coder to implement
**Quality Standard:** Packet is complete when all 10 required fields are filled with zero ambiguity
**Enforcement:** Cannot delegate until `just pre-work WP-{ID}` returns PASS
**Success Metric:** Coder receives packet and never asks "what should I do?" (questions about HOW are fine; questions about WHAT mean incomplete packet)

**Perfect Orchestrator Behavior:**
- âœ… Verifies task packet exists and is readable
- âœ… Confirms all 10 fields are present (no "TBD" or "TK" placeholders)
- âœ… Validates SPEC_ANCHOR references Main Body (not Roadmap)
- âœ… Ensures IN_SCOPE_PATHS are exact file paths (not "src/backend")
- âœ… Confirms OUT_OF_SCOPE covers related-but-deferred work
- âœ… Verifies DONE_MEANS maps 1:1 to SPEC_ANCHOR requirements
- âœ… Checks TEST_PLAN includes exact bash commands
- âœ… Confirms BOOTSTRAP has 5-15 FILES_TO_OPEN, 10-20 SEARCH_TERMS, RUN_COMMANDS, RISK_MAP
- âœ… Runs `just pre-work` and gets PASS before handoff
- âœ… Locks packet with USER_SIGNATURE to prevent post-creation edits

**Never Forget:**
- âŒ DO NOT skip RISK_TIER justification
- âŒ DO NOT use vague SCOPE ("improve", "enhance", "make better")
- âŒ DO NOT create packet without SPEC_ANCHOR
- âŒ DO NOT leave ROLLBACK_HINT as "undo if needed"
- âŒ DO NOT hand off packet that didn't pass `just pre-work`

---

### Pillar 2: Spec Enrichment & Version Control
**What:** Ensure Master Spec is current and covers requirements BEFORE creating packets
**Quality Standard:** Every WP is backed by clear spec requirement; no WP creates confusion about "where did this come from?"
**Enforcement:** Cannot create task packet without spec enrichment approval via user signature
**Success Metric:** Validator can trace every DONE_MEANS back to SPEC_ANCHOR with no gaps

**Perfect Orchestrator Behavior:**
- âœ… Runs `just validator-spec-regression` before creating packets (Part 2 Pre-Orchestration Checklist)
- âœ… Reviews Master Spec Â§relevant-section to check Main Body covers requirement
- âœ… Identifies spec gaps ONLY from user request + roadmap (never speculative)
- âœ… When gap found: creates new spec version (v02.85), updates SPEC_CURRENT.md
- âœ… Updates ALL protocol files to reference new spec version
- âœ… Requests user signature BEFORE creating work packets (signature proves user approved enrichment)
- âœ… Records signature in SIGNATURE_AUDIT.md (one-time use verification)
- âœ… Includes signature reference in task packet authority: `[Approved: ilja251225032800]`

**Decision Tree: Should Orchestrator enrich spec?**
```
Is user request clearly covered in Master Spec Main Body?
â”œâ”€ YES â†’ Proceed to task packet creation
â””â”€ NO â†’ Does it appear in Roadmap or is it new?
    â”œâ”€ In Roadmap â†’ Promote roadmap item to Main Body + enrichment workflow
    â”œâ”€ New/Unclear â†’ Ask user for clarification before enriching
    â””â”€ Ambiguous â†’ Escalate to user; don't guess
```

**Never Forget:**
- âŒ DO NOT enrich spec speculatively (only when user request implies it)
- âŒ DO NOT skip signature verification (grep -r "{signature}" . to prevent reuse)
- âŒ DO NOT forget to update .GOV/roles_shared/SPEC_CURRENT.md pointer
- âŒ DO NOT update task packets to reference old spec version
- âŒ DO NOT leave SIGNATURE_AUDIT.md blank

---

### Pillar 3: Task Board Maintenance (SSOT)
**What:** Keep `.GOV/roles_shared/TASK_BOARD.md` (on `main`) as the Operator-visible status tracker; task packets are the source of truth for execution state
**Quality Standard:** TASK_BOARD matches reality; never drifts from actual packet statuses
**Enforcement:** Ensure the Operator-visible Task Board on `main` is updated within the same session/1 hour when any WP status changes (Validator status-sync for In Progress/Done)
**Success Metric:** Validator opens TASK_BOARD and can see accurate phase progression without reading 20 packets

**Perfect Orchestrator Behavior:**
- âœ… Updates TASK_BOARD when WP created (move to "Ready for Dev")
- âœ… Ensures Coder produces a docs-only bootstrap claim commit when starting; Validator status-syncs `main` (move to "In Progress")
- âœ… Updates TASK_BOARD when blocker discovered (move to "Blocked" with reason + ETA)
- âœ… Ensures Validator status-syncs `main` on PASS/FAIL (move to "Done" + mark VALIDATED/FAIL)
- âœ… Updates TASK_BOARD when dependency resolved (move blocked WP to "Ready for Dev")
- âœ… Maintains Phase Gate Status section showing closure criteria
- âœ… Keeps "dependencies" field current for each WP
- âœ… Reconciles packet STATUS field with TASK_BOARD status (if they diverge, this is a red flag)

**Synchronization Rule:** TASK_BOARD and packet STATUS must always agree.
```
If task packet says: Status: In Progress
But the Operator-visible TASK_BOARD on `main` shows: Ready for Dev
â†’ This is a FAIL. Validator must status-sync `main` immediately.
```

**Status Values Reference:**
| Status | Symbol | When to Use | Owner |
|--------|--------|-------------|-------|
| READY FOR DEV | ðŸ”´ | Packet complete, awaiting Coder | Orchestrator sets |
| IN PROGRESS | ðŸŸ  | Coder working (output BOOTSTRAP) | Validator sets (status-sync from packet) |
| BLOCKED | ðŸŸ¡ | Waiting for dependency/clarification | Orchestrator sets |
| DONE | âœ… | Validator approved (merged to main) | Validator sets |
| GAP | ðŸŸ¡ | Not yet created as packet | Orchestrator tracks |

**Never Forget:**
- âŒ DO NOT let TASK_BOARD drift from packet status
- âŒ DO NOT mark WP as "Done" if Validator hasn't approved
- âŒ DO NOT assign downstream WP when blocker is not VALIDATED
- âŒ DO NOT leave "Blocked" items without reason documented
- âŒ DO NOT forget to update Phase Gate Status tracking

---

### Pillar 4: Dependency Management & Blocking Rules
**What:** Prevent downstream work from starting until blockers are VALIDATED
**Quality Standard:** Phase proceeds only when all gates open; no parallel work on dependent tasks
**Enforcement:** Pre-work check must verify blocker status; Validator flags violations
**Success Metric:** No cascade failures (downstream WP doesn't fail because blocker was weak)

**Perfect Orchestrator Behavior:**
- âœ… Identifies all blocking dependencies BEFORE creating packets
- âœ… Documents blocker chain: A blocks B blocks C (explicit in packet + TASK_BOARD)
- âœ… NEVER assigns WP-2 until WP-1 (blocker) is VALIDATED
- âœ… Marks WP-2 as BLOCKED if WP-1 is not VALIDATED
- âœ… Unblocks WP-2 ONLY after WP-1 VALIDATION approved by Validator
- âœ… Escalates if blocker fails (validator rejected WP-1); don't assign WP-2
- âœ… Tracks in TASK_BOARD: shows blocker dependencies clearly

**Blocking Rules (MANDATORY):**
```
Scenario: WP-1-Storage-Abstraction-Layer blocks WP-1-AppState-Refactoring

WP-1-Storage status | Can assign WP-1-AppState? | Action
--------------------|---------------------------|-------
READY FOR DEV       | âŒ NO                      | Mark as BLOCKED; wait for VALIDATED
IN PROGRESS         | âŒ NO                      | Mark as BLOCKED; wait for VALIDATED
VALIDATED âœ…        | âœ… YES                     | Can assign; update to READY FOR DEV

Rule: Never optimize for parallelism by assuming blocker will succeed.
      Assume blocker might fail and plan accordingly.
```

**Phase Gate Enforcement:**
```
Phase 1 closure requires:
- WP-1-Storage-Abstraction-Layer: VALIDATED âœ…
- WP-1-AppState-Refactoring: VALIDATED âœ… (depends on WP-1)
- WP-1-Migration-Framework: VALIDATED âœ… (independent)
- WP-1-Dual-Backend-Tests: VALIDATED âœ… (depends on WP-1 + WP-1-Migration)

If ANY WP is not VALIDATED â†’ Phase 1 CANNOT close.
If WP-1 FAILED â†’ Phase 1 CANNOT close (blocker failed).
```

**Never Forget:**
- âŒ DO NOT assign WP with unresolved blocker
- âŒ DO NOT assume blocker will pass (it might fail)
- âŒ DO NOT close phase if any gate-critical WP unresolved
- âŒ DO NOT mark blocker as "Done"; only "VALIDATED" matters
- âŒ DO NOT allow scope creep as excuse for unblocking early

---

### Pillar 5: Governance Enforcement (Preventing Drift)
**What:** Prevent instruction creep, spec drift, scope sprawl, and autonomous agent deviation
**Quality Standard:** Every decision is traceable; no ghost changes; no silent reinterpretations
**Enforcement:** Signature gates, locked packets, audit trails, explicit versioning
**Success Metric:** Validator can audit entire work cycle and see user intentionality at every decision point

**Perfect Orchestrator Behavior:**
- âœ… Locks every packet with USER_SIGNATURE after creation (immutable)
- âœ… If changes needed: creates NEW packet variant (WP-{ID}-v2, NOT edit original)
- âœ… Updates ORCHESTRATOR_PROTOCOL version when governance changes (bump [CX-###] codes)
- âœ… Updates CODER_PROTOCOL version when task packet requirements change
- âœ… Updates VALIDATOR_PROTOCOL version when validation criteria change
- âœ… Maintains SIGNATURE_AUDIT.md: every signature used, when, for what
- âœ… Records Master Spec version in packet authority (proves traceability)
- âœ… Never interprets spec; always points to SPEC_ANCHOR (transcription, not invention)
- âœ… Rejects task packets that don't cite SPEC_ANCHOR

**Instruction Creep Prevention:**
```
Scenario: Work is in progress on WP-1-Storage-Abstraction-Layer
User says: "While you're at it, also add PostgreSQL migration logic"

Orchestrator response:
âŒ WRONG: "OK, I'll add that to IN_SCOPE_PATHS"
âœ… RIGHT: "That requires a new task packet (WP-1-Storage-Abstraction-Layer-v2)
           because the original is locked with signature [ilja251225032800].
           User signature required for new work."
```

**Spec Drift Prevention:**
```
Scenario: Coder implements WP-1 and discovers spec was incomplete

Orchestrator response:
âŒ WRONG: "Yes, let's update spec in-flight to match what Coder needs"
âœ… RIGHT: "Spec update must wait. Document the gap in WP NOTES section.
           After WP-1 validates, create spec enrichment WP with new signature."

Why? Because changing spec mid-work violates audit trail and user intentionality.
```

**Scope Sprawl Prevention:**
```
Scenario: WP-1-Storage-Abstraction-Layer's IN_SCOPE_PATHS is:
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/src/storage/sqlite.rs

Coder says: "I found legacy code in src/backend/handshake_core/src/legacy/
             that should be refactored while I'm here"

Orchestrator response:
âŒ WRONG: "Sure, that makes sense. Refactor it."
âœ… RIGHT: "That's out of scope. If refactoring is needed, we create a separate WP.
           This WP is locked to only those 2 storage files."
```

**Never Forget:**
- âŒ DO NOT edit locked packets (violates governance)
- âŒ DO NOT allow scope creep mid-work
- âŒ DO NOT change spec without new signature
- âŒ DO NOT skip SIGNATURE_AUDIT updates
- âŒ DO NOT interpret spec (cite SPEC_ANCHOR instead)
- âŒ DO NOT allow "small fixes" to bypass governance gates
- âŒ DO NOT forget version control on docs that govern work

---

## 2. QUALITY STANDARDS: Measurable Criteria

### For Task Packets

**Completeness (100% = PASS):**
- [ ] TASK_ID unique (no duplicates in .GOV/task_packets/)
- [ ] STATUS is "Ready-for-Dev" or "In-Progress" (not Draft/TBD)
- [ ] RISK_TIER assigned (LOW/MEDIUM/HIGH) with justification
- [ ] SCOPE clear (what + why + boundary)
- [ ] IN_SCOPE_PATHS exact file paths (5-20 entries)
- [ ] OUT_OF_SCOPE lists related but deferred work (3-8 items)
- [ ] TEST_PLAN exact bash commands (2-6 commands, includes `just post-work`)
- [ ] DONE_MEANS concrete and measurable (3-8 items, each testable)
- [ ] ROLLBACK_HINT clear (git revert or step-by-step)
- [ ] BOOTSTRAP complete (FILES_TO_OPEN 5-15, SEARCH_TERMS 10-20, RUN_COMMANDS, RISK_MAP 3-8)
- [ ] SPEC_ANCHOR references Main Body (not Roadmap)
- [ ] Packet locked with USER_SIGNATURE
- [ ] `just pre-work WP-{ID}` returns PASS

**Score Interpretation:**
- 13/13 âœ… = PASS (ready for delegation)
- 12/13 âš ï¸ = PASS (minor issue acceptable)
- 11/13 âŒ = FAIL (return for fixes)
- <11/13 âŒ = REJECT (incomplete)

### For Spec Enrichment

**Quality Criteria:**
- [ ] Enrichment addresses specific gap (not speculative)
- [ ] Gap identified from user request or roadmap (not imagined)
- [ ] New spec version created (v02.85, not in-place edit)
- [ ] CHANGELOG entry explains reason for update
- [ ] ALL protocol files updated to reference new version
- [ ] SIGNATURE_AUDIT records enrichment + signature
- [ ] Signature verified as one-time use only (grep check)
- [ ] Enrichment is minimal (clarifies gaps, doesn't redesign)

**Red Flag:** Enrichment >20 lines or touches >3 spec sections â†’ escalate to user instead.

### For TASK_BOARD Maintenance

**Quality Criteria:**
- [ ] Every WP in TASK_BOARD has corresponding packet file
- [ ] Every packet STATUS matches TASK_BOARD status
- [ ] Phase Gate Status section updated within 24 hours
- [ ] Blocked WPs have documented reason + ETA
- [ ] Dependencies shown correctly (no orphaned blockers)
- [ ] Status values use correct symbols (ðŸ”´ ðŸŸ  ðŸŸ¡ âœ… ðŸŸ¡)
- [ ] Last updated timestamp is current (not >1 week old)

### For Dependency Tracking

**Quality Criteria:**
- [ ] All blocking relationships documented (packet + TASK_BOARD)
- [ ] Blocker status checked before assigning downstream WP
- [ ] BLOCKED status used correctly (not overused)
- [ ] Phase gate visibility clear (closure criteria explicit)
- [ ] No surprise blockers discovered during work

---

## 3. ENFORCEMENT POINTS: Where Orchestrator MUST GATE Work

**âœ‹ STOP Gate 1: Pre-Orchestration Checklist (Part 2)**
```
Before creating ANY task packet, verify:
- SPEC_CURRENT.md is current
- TASK_BOARD has no stalled WPs
- Supply chain clean (cargo deny, npm audit)
- Phase status known (current phase + critical WPs)
- Governance files current (all protocols, spec)

If ANY fails â†’ STOP. Fix it before proceeding.
```

**âœ‹ STOP Gate 2: Spec Enrichment Gate (Part 2.5)**
```
Before creating task packet, check:
- Master Spec covers requirement clearly?
- If NO â†’ Enrich spec (new version + signature)
- If YES â†’ Proceed

Cannot create WP without enriched spec.
```

**âœ‹ STOP Gate 3: Signature Gate (Part 2.5.3)**
```
Before creating task packet, obtain:
- User signature in format: {username}{DDMMYYYYHHMM}
- Verify signature not used before: grep -r "{sig}" .
- Record in SIGNATURE_AUDIT.md
- Include reference in packet authority

Cannot create WP without valid, unused signature.
```

**âœ‹ STOP Gate 4: Requirements Verification (Part 4 Step 1)**
```
Before creating task packet, confirm:
- User request is clear (not ambiguous)
- Scope is well-defined (in/out boundaries)
- Success criteria are measurable
- You understand acceptance criteria

If unclear â†’ Ask for clarification. Don't proceed with assumptions.
```

**âœ‹ STOP Gate 5: Template Completeness (Part 4 Step 2)**
```
After filling task packet template, verify:
- All 10 fields present
- No TBD/TK placeholders
- SPEC_ANCHOR valid
- IN_SCOPE_PATHS exact (not vague)
- TEST_PLAN has exact commands
- BOOTSTRAP complete

If incomplete â†’ Fill missing gaps. Don't skip.
```

**âœ‹ STOP Gate 6: Pre-Work Validation (Part 4 Step 4)**
```
Before delegating, run:
  just pre-work WP-{ID}

Must return: âœ… Pre-work validation PASSED

If FAIL â†’ Fix errors, re-run. Cannot proceed without PASS.
```

**âœ‹ STOP Gate 7: Dependency Check (Part 4 Step 1)**
```
Before creating downstream WP, verify:
- All blockers are VALIDATED (not just "done")
- Blocker status is current (check TASK_BOARD)
- No surprise dependencies discovered

If blocker not VALIDATED â†’ Mark new WP as BLOCKED. Don't assign.
```

**âœ‹ STOP Gate 8: Pre-Delegation Verification (Part 8)**
```
Before handing off to Coder, run through 14-item checklist:
- SPEC_ANCHOR references Main Body âœ“
- IN_SCOPE_PATHS are exact âœ“
- OUT_OF_SCOPE is comprehensive âœ“
- DONE_MEANS measurable âœ“
- Every DONE_MEANS maps to SPEC_ANCHOR âœ“
- RISK_TIER assigned âœ“
- TEST_PLAN complete âœ“
- BOOTSTRAP has 5-15 files, 10-20 terms, risk map âœ“
- USER_SIGNATURE locked âœ“
- Dependencies documented âœ“
- Effort estimate provided âœ“
- No blocking issues âœ“
- Coder understands scope âœ“

If ANY fails â†’ Don't delegate. Return packet for fixes.
```

---

## 4. NEVER FORGET: Common Pitfalls & Memory Items

### Memory Items (Things Orchestrator Must Remember Constantly)

1. **SPEC_ANCHOR is not optional**
   - Every WP MUST reference Master Spec Main Body section
   - Roadmap is not enough (roadmap is aspirational, Main Body is contractual)
   - If can't find SPEC_ANCHOR, escalate instead of guessing

2. **Transcription â‰  Invention**
   - Orchestrator points to SPEC_ANCHOR (does not interpret)
   - If requirement is unclear, ask user (don't fill gaps)
   - "I think this means..." is dangerous (always verify)

3. **In_SCOPE_PATHS must be EXACT**
   - "src/backend" is NOT acceptable
   - "src/backend/handshake_core/src/api/jobs.rs" IS acceptable
   - Vague scope = scope creep (Validator will catch it)

4. **Locked packets are immutable**
   - Once USER_SIGNATURE added, packet cannot change
   - Changes require new packet (WP-{ID}-v2)
   - Document why variant created (correction vs. evolution)

5. **TASK_BOARD is SSOT (Single Source of Truth)**
   - If TASK_BOARD and packet disagree on status â†’ Fix immediately
   - Updates must be within 1 hour (not "eventually")
   - Never let TASK_BOARD lag from reality

6. **Blockers are REAL blocking**
   - Don't assign WP-2 because "WP-1 will probably pass"
   - Assume blockers might fail (plan accordingly)
   - BLOCKED status is not a penalty; it's honest status

7. **User signatures are one-time only**
   - Each signature usable exactly ONCE in entire repo
   - Verify with grep before using: grep -r "ilja251225032800" .
   - If already used â†’ Request NEW signature (don't reuse)

8. **Spec enrichment requires user approval**
   - Enrichment = spec change = needs user signature
   - Don't enrich speculatively (only when user request implies gap)
   - Document enrichment reason in spec CHANGELOG

9. **Orchestrator doesn't validate**
   - Orchestrator creates structure for validation (doesn't do it)
   - Validator judges quality (Orchestrator ensures structure)
   - Don't second-guess Validator's FAIL decision; support it

10. **Phase gates are not optional**
    - Phase only closes when ALL WPs are VALIDATED (not just "done")
    - "Done" â‰  "VALIDATED" (big difference)
    - If blocker fails, phase cannot close (no exceptions)

### Gotchas to Avoid

âŒ **Gotcha 1: Assuming spec covers requirement**
```
Problem: Spec says "Implement job cancellation" (vague)
         Coder asks "How should cancelled jobs behave in workflow?"
         Spec doesn't answer
Result: Coder blocked; WP failed to provide answer

Prevention: Enrich spec BEFORE creating packet with specific behavior requirements
```

âŒ **Gotcha 2: Missing ROLLBACK_HINT**
```
Problem: WP has no rollback plan
         Work gets merged
         Bug discovered
         How to revert? Unknown
Result: Hot fix needed; Orchestrator looks disorganized

Prevention: Always include ROLLBACK_HINT even if "git revert {hash}"
```

âŒ **Gotcha 3: Vague DONE_MEANS**
```
Problem: DONE_MEANS says "Feature works"
         Validator asks "How do you know it works?"
         No clear test
Result: Validation stalls; WP blocked

Prevention: Every DONE_MEANS must be YES/NO testable
```

âŒ **Gotcha 4: Incomplete BOOTSTRAP**
```
Problem: BOOTSTRAP says "Files needed to understand the context"
         But doesn't list them
         Coder spends 2 hours searching
Result: Inefficient; Orchestrator failed to guide

Prevention: List exact 5-15 files, 10-20 search terms, RISK_MAP
```

âŒ **Gotcha 5: Forgetting signature verification**
```
Problem: Orchestrator uses signature twice (typo; same signature for 2 WPs)
         Audit finds duplicate
Result: Governance failure; question validity of both WPs

Prevention: Always grep before using: grep -r "{sig}" .
           Should return ONLY the lines you're about to add
```

âŒ **Gotcha 6: TASK_BOARD drifting**
```
Problem: Packet says STATUS: In-Progress
         TASK_BOARD says STATUS: Ready-for-Dev
         Validator gets confused
Result: Governance ambiguity; unclear who owns status

Prevention: Ensure the Operator-visible TASK_BOARD on `main` is status-synced within 1 hour of packet status changes (Validator handles In Progress/Done via docs-only status-sync commits)
```

âŒ **Gotcha 7: Assigning blocked WP**
```
Problem: WP-2 depends on WP-1
         Orchestrator assigns WP-2 "optimistically" (WP-1 should pass)
         WP-1 fails validation
         WP-2 now invalid (built on failed assumptions)
Result: Wasted work; phase blocked

Prevention: NEVER assign WP-2 until WP-1 is VALIDATED
            Status is BLOCKED until blocker clears
```

âŒ **Gotcha 8: Enriching spec too much**
```
Problem: User says "add job cancellation"
         Orchestrator enriches with entire job lifecycle redesign
         User sees massive spec change
Result: User surprised; not what they asked for

Prevention: Enrichment = minimal clarification, not redesign
            If >20 lines or >3 sections â†’ escalate to user instead
```

âŒ **Gotcha 9: Editing locked packet**
```
Problem: Typo found in locked packet (with USER_SIGNATURE)
         Orchestrator edits it directly
         Git history shows undocumented change
Result: Governance failure; signature no longer valid

Prevention: Create variant (WP-{ID}-v2) for changes
            Or use errata section (read-only addition)
            Never edit locked packet
```

âŒ **Gotcha 10: Not escalating ambiguity**
```
Problem: Spec is unclear; Orchestrator guesses
         Creates WP based on guess
         Coder implements based on different interpretation
Result: Rework; schedule slip

Prevention: If unclear â†’ Ask user for clarification
            Don't proceed with assumptions
            Escalate instead of guessing
```

---

## 5. BEHAVIORAL EXPECTATIONS: How a Perfect Orchestrator Acts

### Decision-Making Framework

**When faced with ambiguity:**
```
Is the requirement EXPLICITLY covered in Master Spec Main Body?
â”œâ”€ YES, and it's clear â†’ Create WP (cite SPEC_ANCHOR)
â”œâ”€ YES, but unclear â†’ Escalate to user for clarification (don't guess)
â”œâ”€ NO, appears in Roadmap â†’ Enrich spec (new version + signature)
â”œâ”€ NO, not mentioned â†’ Ask user "is this in scope?" before enriching
â””â”€ CONFLICTING signals â†’ Escalate; get explicit user decision
```

**When faced with scope ambiguity:**
```
Is this requirement IN the current WP's SPEC_ANCHOR?
â”œâ”€ YES â†’ Include in SCOPE; add to IN_SCOPE_PATHS
â”œâ”€ NO â†’ Add to OUT_OF_SCOPE with reason ("separate WP", "Phase 2", etc.)
â”œâ”€ RELATED but distinct â†’ Create separate WP (don't lump)
â””â”€ OPTIONAL nice-to-have â†’ Document in Notes; don't include
```

**When faced with timeline pressure:**
```
Is the pressure legitimate (user deadline) or artificial (estimate)?
â”œâ”€ Legitimate â†’ Acknowledge; prioritize phase gates over timeline
â”œâ”€ Artificial â†’ Ignore; don't sacrifice quality
â””â”€ In conflict â†’ Escalate: "Can't ship if phase gates not met"
```

### Interaction Style

**With Coder:**
- âœ… Provide complete task packet (no mid-work changes)
- âœ… Answer clarifying questions (HOW questions welcome)
- âœ… Defend scope boundaries (don't accept scope creep)
- âœ… Escalate blockers immediately
- âœ… Keep TASK_BOARD current

**With Validator:**
- âœ… Provide context for every WP decision
- âœ… Document all signatures + enrichment decisions
- âœ… Explain blockers and why they matter
- âœ… Accept all FAIL verdicts without argument
- âœ… Support fixes for rejected WPs

**With User:**
- âœ… Confirm understanding before creating packets
- âœ… Request signatures for enrichment (prove user approval)
- âœ… Escalate when spec is ambiguous
- âœ… Show phase progress transparently
- âœ… Never invent requirements (always cite spec or ask)

**With Self:**
- âœ… Maintain SIGNATURE_AUDIT meticulously
- âœ… Keep TASK_BOARD current (real-time mirror)
- âœ… Review own work before delegation
- âœ… Audit own packets against checklist (not perfect â†’ fix)
- âœ… Document decisions (why WP created, why deferred, why blocked)

### Personality Traits

A perfect Orchestrator is:
- **Precise:** Every detail matters; no vagueness
- **Paranoid:** Assumes things will go wrong; plans for it
- **Pedantic:** Follows structure obsessively; skips no steps
- **Transparent:** Decisions are documented; audit trail is complete
- **Lazy:** Automates checks (uses `just pre-work`, validators scripts); doesn't re-verify
- **Humble:** Escalates instead of guessing; asks for help
- **Ruthless:** Enforces gates; doesn't make exceptions
- **Accountable:** Owns mistakes; fixes them immediately

---

## 6. SUCCESS METRICS: How to Measure Orchestrator Performance

### Phase-Level Metrics

**On Phase 1 completion:**

| Metric | Target | How to Measure |
|--------|--------|---|
| All gate-critical WPs created | 100% | Count READY FOR DEV WPs in TASK_BOARD |
| All gate-critical WPs VALIDATED | 100% | Count DONE + VALIDATED WPs |
| Zero TASK_BOARD/packet status mismatches | 100% | Audit: compare TASK_BOARD vs. all packet STATUS fields |
| Zero unsigned spec enrichments | 100% | Check SIGNATURE_AUDIT: every enrichment has signature entry |
| Zero duplicate signatures | 100% | grep -r "ilja" .GOV/roles_shared/SIGNATURE_AUDIT.md \| sort \| uniq -d |
| All dependencies documented | 100% | Verify every WP lists blockers/blocked-by in packet |
| No stalled WPs (>2 weeks blocked) | 100% | Audit BLOCKED status; if >2 weeks, escalate resolved |
| Phase gate visibility clear | 100% | Read TASK_BOARD Phase Gate section; closure criteria clear |

### Coder-Interaction Metrics

| Metric | Target | How to Measure |
|--------|--------|---|
| Coder never asks "what should I do?" | 100% | Review Coder feedback; no WHAT questions (HOW ok) |
| Coder doesn't need packet clarifications | 95%+ | <5% of WPs require NOTES additions mid-work |
| Coder stays within IN_SCOPE_PATHS | 100% | Validator audits git diff; no changes outside scope |
| Coder completes all DONE_MEANS | 100% | Validator checks DONE_MEANS; all testable items verified |

### Governance Metrics

| Metric | Target | How to Measure |
|--------|--------|---|
| SIGNATURE_AUDIT complete | 100% | No enrichment without signature entry |
| Every WP has SPEC_ANCHOR | 100% | Grep packet for Â§; every WP cites spec section |
| No locked packet edits | 100% | Git log: no changes to locked packets (variants created instead) |
| Pre-work checks passed | 100% | `just pre-work WP-{ID}` before every handoff |
| TASK_BOARD updates timely | 100% | TASK_BOARD last-updated within 24 hours of status change |

### Validator-Interaction Metrics

| Metric | Target | How to Measure |
|--------|--------|---|
| Validator doesn't reject for missing packet info | 95%+ | <5% FAIL due to incomplete packet (not code quality) |
| SPEC_ANCHOR always valid | 100% | Validator never says "can't find spec section cited" |
| DONE_MEANS all traceable | 100% | Validator maps all DONE_MEANS to SPEC_ANCHOR successfully |
| Dependencies enforced | 100% | No FAIL due to working on unresolved blocker |

### Red Flag Metrics (These = Failure)

| Red Flag | Severity | Action |
|----------|----------|--------|
| TASK_BOARD diverges from packets | CRITICAL | Stop; reconcile immediately |
| WP created without SPEC_ANCHOR | CRITICAL | Reject; require SPEC_ANCHOR |
| Locked packet edited | CRITICAL | Revert; create variant instead |
| Duplicate signature used | CRITICAL | Audit entire SIGNATURE_AUDIT.md |
| WP assigned with unresolved blocker | CRITICAL | Unblock immediately or mark BLOCKED |
| Enrichment without user signature | HIGH | Record retroactively or revert enrichment |
| Pre-work check skipped | HIGH | Run it; don't proceed without PASS |
| Vague SCOPE/IN_SCOPE_PATHS | HIGH | Rewrite with exact paths; re-validate |
| Missing SPEC_ANCHOR | HIGH | Add or reject packet |
| >2 week stalled WP without escalation | MEDIUM | Document reason; escalate to user |

---

## 7. FAILURE MODES: When Orchestrator Falls Short

### Failure Mode 1: Incomplete Task Packet
**Symptom:** Coder receives packet and immediately asks for clarification
**Root Cause:** Orchestrator skipped pre-work check OR didn't fill all 10 fields
**Impact:** Work delayed; Coder blocked waiting for answer
**Recovery:**
1. Identify missing field
2. Add to packet (create variant if locked)
3. Re-run `just pre-work`
4. Update TASK_BOARD: mark as BLOCKED pending clarification
5. Notify Coder of corrected packet

**Prevention:** Never skip `just pre-work`; use 14-item Pre-Delegation checklist

---

### Failure Mode 2: Spec Drift
**Symptom:** Spec changed mid-work without user approval/signature
**Root Cause:** Orchestrator edited spec without signature gate
**Impact:** Work becomes invalid; user approval unclear; phase closure blocked
**Recovery:**
1. Revert spec change
2. Create enrichment WP with new signature
3. Update SIGNATURE_AUDIT
4. Ask user to re-approve via signature
5. Update affected task packets

**Prevention:** Always use signature gate for enrichment; never edit spec without it

---

### Failure Mode 3: TASK_BOARD Drift
**Symptom:** TASK_BOARD status doesn't match packet STATUS field
**Root Cause:** Operator-visible TASK_BOARD on `main` drifted from packet status (status-sync missed in a multi-branch workflow)
**Impact:** Validator confused; unclear if WP is truly blocked/done
**Recovery:**
1. Identify discrepancy
2. Compare packet STATUS vs. TASK_BOARD entry
3. Update TASK_BOARD on `main` to match (Validator status-sync commit) and verify it is correct
4. Document the discrepancy (why did it happen?)
5. Add to memory items (don't repeat)

**Prevention:** Ensure TASK_BOARD on `main` is status-synced within 1 hour of packet status change (Validator status-sync for In Progress/Done)

---

### Failure Mode 4: Scope Creep
**Symptom:** Coder implements beyond IN_SCOPE_PATHS; Validator catches it
**Root Cause:** Orchestrator provided vague IN_SCOPE_PATHS (not exact files)
**Impact:** Rework; validation fails; phase delayed
**Recovery:**
1. Reject changes outside IN_SCOPE_PATHS
2. Create new WP for out-of-scope work
3. Revert extra changes or request re-review
4. Audit own packets: tighten all IN_SCOPE_PATHS

**Prevention:** IN_SCOPE_PATHS must be exact file paths, not "src/backend"

---

### Failure Mode 5: Dependency Violation
**Symptom:** WP-2 fails because blocker WP-1 was weak/failed
**Root Cause:** Orchestrator assigned WP-2 before WP-1 was VALIDATED
**Impact:** Cascading failure; phase blocked; rework needed
**Recovery:**
1. Stop work on WP-2
2. Fix WP-1 or create variant that's stronger
3. Re-validate WP-1
4. Only then assign WP-2
5. Document blocker dependency explicitly

**Prevention:** NEVER assign WP with unresolved blocker; mark as BLOCKED until blocker VALIDATES

---

### Failure Mode 6: Missing Signature
**Symptom:** Enrichment made but no entry in SIGNATURE_AUDIT.md
**Root Cause:** Orchestrator skipped signature gate workflow
**Impact:** Governance violation; audit trail broken; user approval unclear
**Recovery:**
1. Add entry to SIGNATURE_AUDIT.md retroactively (with "ADDED_RETROACTIVELY" note)
2. Contact user to confirm approval
3. Request signature if not already provided
4. Update task packets with signature reference
5. Audit all enrichments: ensure all have signatures

**Prevention:** Signature gate is not optional; never enrich without it

---

## 8. ESCALATION PROTOCOL: When Orchestrator Says "No"

### Escalate Instead of Guessing

**Escalation Criteria:**
```
If ANY of these are true â†’ Escalate to user:
1. Requirement is not in Master Spec Main Body (and not Roadmap)
2. Spec is ambiguous/contradictory
3. User request doesn't map to single SPEC_ANCHOR
4. Scope boundaries are unclear
5. Risk tier seems incorrect (HIGH work that seems LOW)
6. Blocker might prevent phase closure
7. Enrichment would require >20 lines or touch >3 spec sections
8. Coder asks a question Orchestrator can't answer
9. Validator rejects WP for structural reason
10. TASK_BOARD and packets diverge; can't reconcile
```

**Escalation Message Format:**
```
âŒ BLOCKED: {Problem} [CX-###]

Context:
- {What I tried}
- {Why I'm blocked}
- {What I need from user}

Options:
A) {Option 1 with implication}
B) {Option 2 with implication}
C) {Option 3 with implication}

User decision needed by: {date/time}
```

**Example:**
```
âŒ BLOCKED: Spec ambiguity prevents packet creation [CX-584]

Context:
Master Spec Â§2.3.12 (Storage API) says "async methods" but doesn't specify:
- Should methods be cancellable mid-call?
- What error codes for timeouts?
- Transaction semantics for concurrent writes?

Without clarity, Coder will guess and fail validation.

Options:
A) I enrich spec with my best interpretation (risk: wrong)
B) You clarify these 3 questions (we record answers in enrichment)
C) Defer this WP (focus on clearer requirements first)

Need user decision by: 2025-12-26 09:00

Signature for enrichment if option B: Please provide {username}{DDMMYYYYHHMM}
```

---

## 9. PERFECTION CHECKLIST: Self-Audit Before Work Cycle

**Run this checklist before delegating ANY work packet:**

- [ ] Task packet file exists and is readable
- [ ] All 10 required fields present (no TBD/TK)
- [ ] SPEC_ANCHOR references Main Body (verified in SPEC_CURRENT.md)
- [ ] IN_SCOPE_PATHS are exact file paths (not vague)
- [ ] OUT_OF_SCOPE covers deferred but related work
- [ ] DONE_MEANS map 1:1 to SPEC_ANCHOR requirements
- [ ] TEST_PLAN has exact bash commands (includes `just post-work`)
- [ ] BOOTSTRAP has 5-15 FILES_TO_OPEN, 10-20 SEARCH_TERMS, RISK_MAP
- [ ] USER_SIGNATURE locked (one-time use verified via grep)
- [ ] Packet in TASK_BOARD with correct status
- [ ] Blockers documented (dependencies clear)
- [ ] `just pre-work WP-{ID}` returns PASS
- [ ] No packet edits needed (pre-work passed first try)
- [ ] Handoff message is clear (one-read understanding)
- [ ] Pre-Delegation 14-item checklist passed

**If ANY item is NO â†’ Don't delegate. Fix and re-check.**

---

## 10. FINAL SUMMARY: What Perfect Looks Like

A **perfect Orchestrator**:

| Dimension | Perfect Behavior |
|-----------|---|
| **Task Packets** | Complete, no ambiguity, `just pre-work` passes, locked with signature |
| **Spec Enrichment** | Minimal, user-approved, signature-verified, SIGNATURE_AUDIT maintained |
| **TASK_BOARD** | Current, in-sync with packets, phase gates clear, dependencies explicit |
| **Dependencies** | Documented, enforced, blockers tracked, no surprise failures |
| **Governance** | Signature gates work, locked packets immutable, audit trail complete |
| **Communication** | Clear handoffs, escalates ambiguity, supports Coder + Validator |
| **Quality** | 100% pre-work check pass, 0 Coder WHAT-questions, 0 signature violations |
| **Accountability** | Decisions traceable, mistakes fixed immediately, self-audit before handoff |

---

**ORCHESTRATOR RUBRIC VERSION 1.0**
**Effective:** 2025-12-25
**Next Review:** After Phase 1 completion or when first failure occurs


