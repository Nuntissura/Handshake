# ORCHESTRATOR_IMPLEMENTATION_ROADMAP: Path to 9.9/10

**Current Grade:** A- (87/100)
**Target Grade:** A+ (99/100)
**Model Tier:** Cheap/Standard (no expensive LLM required)
**Work Type:** Documentation + simple validation scripts (no product code)

---

## Quick Assessment: Impact vs. Effort

| Gap | Impact | Effort | ROI | Priority |
|-----|--------|--------|-----|----------|
| Gap 2: "Clearly covers" definition | HIGH | LOW | 10x | **P0** |
| Gap 3: Error recovery procedures | HIGH | MEDIUM | 8x | **P0** |
| Gap 4: Phase closure criteria | HIGH | LOW | 10x | **P0** |
| Gap 5: SLA for gates | MEDIUM | LOW | 8x | **P1** |
| Gap 12: Risk tier matrix | MEDIUM | LOW | 8x | **P1** |
| Gap 11: Escalation path + SLA | MEDIUM | LOW | 7x | **P1** |
| Gap 6: Audit trail (Decision Log) | MEDIUM | MEDIUM | 6x | **P2** |
| Gap 1: Compliance checker script | MEDIUM | MEDIUM | 5x | **P2** |
| Gap 8: Variant management | MEDIUM | MEDIUM | 5x | **P2** |
| Gap 14: ERRATA for minor fixes | LOW | LOW | 4x | **P3** |
| Gap 13: User context preservation | LOW | LOW | 3x | **P3** |
| Gap 15: Signature batching rules | LOW | LOW | 3x | **P3** |
| Gap 7: Alignment checker script | LOW | MEDIUM | 2x | **P3** |
| Gap 10: Dependency visualization | LOW | MEDIUM | 2x | **P3** |
| Gap 9: Spec version checker | LOW | MEDIUM | 2x | **P3** |

---

## PHASE 1: Critical Foundations (87 â†’ 93/100) â€” 2-3 hours

These 4 items unlock clarity and prevent major ambiguities.

### P0-1: "Clearly Covers" Definition [Gap 2]

**What:** Add objective 5-point checklist to Part 2.5.1 decision tree

**Current State:**
```
Decision tree asks: "Does Master Spec Main Body clearly cover this requirement?"
Response: ??? (subjective)
```

**Fix (30 minutes):**
```markdown
DEFINITION: "Clearly covers" means requirement satisfies ALL:
1. âœ… Appears in Main Body (not Roadmap, not aspirational)
2. âœ… Explicitly named (reader immediately finds it)
3. âœ… Specific (not "SHOULD be portable" but "MUST implement X trait")
4. âœ… Measurable acceptance criteria (verifiable yes/no)
5. âœ… No ambiguity (single valid interpretation)

PASS: All 5 âœ… â†’ Proceed without enrichment
FAIL: Any âŒ â†’ Ask user for clarification OR enrich spec

Examples:
CLEARLY COVERS: "Â§2.3.12.1: Database trait MUST have async fn get_blocks(&self, id: &str)"
- All 5 criteria met; unambiguous
- Proceed without enrichment

DOES NOT CLEARLY COVER: "Â§2.3.12: Storage abstraction SHOULD be portable"
- Criteria 3 fails (not specific); criteria 4 fails (no acceptance criteria)
- Requires user clarification OR enrichment
```

**Where:** Add to ORCHESTRATOR_PROTOCOL Part 2.5.1 (replace vague decision tree)

---

### P0-2: Phase Closure Criteria [Gap 4]

**What:** Add explicit Phase Closure Gate checklist

**Current State:**
```
Part 1 says: "Phase closes when all WPs VALIDATED"
Reality: What else? No other criteria defined.
```

**Fix (45 minutes):**
```markdown
## Phase Closure Gate (Explicit Requirements)

**MUST criteria (all required):**
- [ ] All phase-critical WPs: VALIDATED verdict (âœ… PASS)
- [ ] Spec regression check: `just validator-spec-regression` â†’ PASS
- [ ] Supply chain audit: `cargo deny check` + `npm audit` â†’ 0 violations
- [ ] No unresolved blockers (all dependencies resolved)
- [ ] Git commit audit trail complete (all commits signed/traced)

**SHOULD criteria (strong recommendations):**
- [ ] No open escalations from Validator
- [ ] No "deferred work" notes in WPs (all planned work done)
- [ ] Test coverage metrics on target (>80% for phase)
- [ ] Security audit clean (if phase touches security)

**Example: Phase 1 Closure Gate**
```
Phase 1 Gate Status:
- WP-1-Storage-Abstraction-Layer: VALIDATED âœ…
- WP-1-AppState-Refactoring: VALIDATED âœ…
- WP-1-Migration-Framework: VALIDATED âœ…
- WP-1-Dual-Backend-Tests: VALIDATED âœ…
- Spec regression: PASS âœ…
- Supply chain: PASS âœ…
- No escalations pending âœ…
â†’ Phase 1 READY TO CLOSE âœ…
```
```

**Where:** Add to ORCHESTRATOR_PROTOCOL Part 6 (after TASK_BOARD section)

---

### P0-3: Error Recovery Procedures [Gap 3]

**What:** Document how to recover from common Orchestrator mistakes

**Current State:**
```
Mistake: Signature used twice
Recovery: "Oops, can't undo. Governance violation."
```

**Fix (1 hour):**
```markdown
## ERROR RECOVERY PROCEDURES

### Error 1: Signature Used Twice (Typo/Mistake)
**Prevention:** Always grep before using: `grep -r "{sig}" .`
**If error occurs:**
1. Mark signature INVALID in SIGNATURE_AUDIT.md
   (Add: "STATUS: INVALID (used twice by mistake)")
2. Request NEW signature from user (different timestamp)
3. Update work packet with new signature reference
4. Document in packet NOTES: "Original signature invalid; replaced with..."

### Error 2: Wrong SPEC_ANCHOR in Locked Packet
**Prevention:** Verify SPEC_ANCHOR exists in SPEC_CURRENT.md before locking
**If error occurs:**
1. Check severity:
   - CRITICAL (wrong scope): Create variant (WP-{ID}-v2)
   - MINOR (wrong section, same scope): Add ERRATA
2. If variant: Create WP-{ID}-v2 with correct SPEC_ANCHOR
3. If ERRATA: Add read-only ERRATA section to locked packet:
   ```
   ## ERRATA
   - Original SPEC_ANCHOR: Â§X was incorrect
   - Correct SPEC_ANCHOR: Â§Y
   - Reason: Typo (scope unchanged)
   - Date corrected: YYYY-MM-DD
   ```
4. Update TASK_BOARD to reference active version

### Error 3: TASK_BOARD Out of Sync with Packets
**Prevention:** Use docs-only status-sync commits (Coder bootstrap claim commit + Validator updates TASK_BOARD on `main` within 1 hour of packet status changes)
**If error occurs:**
1. Compare Operator-visible TASK_BOARD on `main` vs. packet STATUS field
2. Identify discrepancies
3. Update TASK_BOARD on `main` to match packet reality (task packets are source of truth)
4. Log in decision log (optional): "Status-sync: TASK_BOARD was X days out of sync"
5. Review: Why did sync break? (What to do differently?)

### Error 4: Blocker Status Missed in Step 1
**Prevention:** Check TASK_BOARD blocker status before creating WP
**If error occurs:**
1. Immediately mark new WP as BLOCKED in TASK_BOARD
2. Document: "Discovered blocker after creation; should have been caught"
3. Add to packet: "Blocker: WP-X (status: {current status})"
4. Review: Why was blocker missed? Improve your checklist.

### Error 5: Enrichment Without User Signature
**Prevention:** Always request signature before enriching spec
**If error occurs:**
1. Retroactively request user signature for enrichment
2. Add to SIGNATURE_AUDIT.md: "Retroactive approval: signature {sig}"
3. Update task packet with signature reference
4. Note: "This is debt; avoid in future by requesting signature BEFORE enriching"

### Error 6: Missing Signature in SIGNATURE_AUDIT.md
**Prevention:** Record every signature immediately upon use
**If error occurs:**
1. Add missing entries to SIGNATURE_AUDIT.md
2. Verify signature format is correct
3. Note: "Added retroactively; ensure all future signatures recorded immediately"
```

**Where:** Add to ORCHESTRATOR_PROTOCOL as new Part "Error Recovery" (after Critical Rules)

---

### P0-4: SLA for Gates [Gap 5]

**What:** Add time commitments for gate resolution

**Current State:**
```
WP can be BLOCKED forever with no escalation
```

**Fix (30 minutes):**
```markdown
## SLA for Work States

**BLOCKED Status:**
- MAX duration: 5 work days
- If exceeded: Escalate to user with blocker status
- Action: "Blocker WP-X has been unresolved for 6 days. What's the plan?"

**READY FOR DEV Status:**
- MAX duration: 10 work days
- If exceeded: Flag as risk (why hasn't Coder started?)
- Action: "WP in Ready state for 12 days. Is there a blocker?"

**IN PROGRESS Status:**
- MAX duration: 30 work days
- If exceeded: Assess if estimate was wrong
- Action: "WP in progress for 35 days; original estimate was 20 hours"

**Escalation Template:**
```
âš ï¸ SLA EXCEEDED: {WP-ID} [CX-###]

Status: {BLOCKED|READY|IN PROGRESS}
Duration: {X days} (SLA: {Y days})

Current state: {description}

Action needed: {what must happen to unblock}

Escalation: Please respond by {date/time}
```
```

**Where:** Add to ORCHESTRATOR_PROTOCOL Part 7 (Dependency Management)

---

## PHASE 2: Quality Systems (93 â†’ 96/100) â€” 2-3 hours

These 3 items add rigor and auditability.

### P1-1: Decision Log Template [Gap 6]

**What:** Enable audit trail of Orchestrator decisions

**Fix (1 hour):** Create template showing:
- Pre-Orchestration Checklist verification
- Spec enrichment decision + reasoning
- Blocker status check
- Pre-Delegation Checklist sign-off

**Where:** Add to ORCHESTRATOR_PROTOCOL Part 8 or create separate ORCHESTRATOR_DECISION_LOG_TEMPLATE.md

---

### P1-2: Risk Tier Objective Matrix [Gap 12]

**What:** Replace vague risk definitions with objective criteria

**Fix (45 minutes):**
```markdown
## Risk Tier Matrix (Objective)

| Dimension | LOW | MEDIUM | HIGH |
|-----------|-----|--------|------|
| Files changed | 1-5 | 6-15 | >15 |
| Modules affected | 1 | 2 | >2 |
| Migrations needed | No | No | Yes |
| IPC changes | No | No | Yes |
| Security impact | None | Low | High |
| Test new code | Existing | +50% | +80% |
| Manual review required | Optional | Required | Required |
| Effort hours | <4 | 4-20 | >20 |

**Rule: Take MAXIMUM tier across dimensions**

Example: 10 files (MEDIUM) + 1 migration (HIGH) + security impact low (MEDIUM)
â†’ Result: **HIGH tier** (max of all dimensions)
```

**Where:** Add to ORCHESTRATOR_PROTOCOL Part 4 (Task Packet Creation)

---

### P1-3: Escalation Path + SLA [Gap 11]

**What:** Define where and how to escalate ambiguity

**Fix (45 minutes):**
```markdown
## Escalation Protocol

**When to escalate:**
- Requirement ambiguous (multiple valid interpretations)
- Spec unclear (doesn't answer key questions)
- Scope boundary disputed
- Risk tier disagreement
- Blocker unresolved beyond SLA

**How to escalate:**
1. Create GitHub issue: `[ESCALATION] {WP-ID}: {issue}`
2. Include:
   - Problem statement
   - Options (if multiple valid paths exist)
   - Decision deadline
3. Tag: `escalation:orchestrator`

**SLA for escalation response:**
- User responds within 24 hours
- If no response: Phase is blocked (cannot proceed)
- Escalation board status: BLOCKED (reason: awaiting user decision)

**Example:**
```
Title: [ESCALATION] WP-1-Storage: Spec ambiguity on cancellation semantics

BLOCKED: Cannot create WP without spec clarity

Question: "When user cancels a storage operation mid-flight, should..."
Option A: Cancel immediately, lose partial data
Option B: Cancel after current block, return partial results
Option C: Deny cancel, queue request (risky)

Spec is silent on this. Which is correct?

Decision needed by: 2025-12-26 09:00 EST
Signature will be required to enrich spec with chosen approach.
```
```

**Where:** Add to ORCHESTRATOR_PROTOCOL Part "Escalation & Blockers"

---

## PHASE 3: Edge Cases & Polish (96 â†’ 99/100) â€” 2-3 hours

These 6 items handle special cases and improve usability.

### P2-1: Variant Management Procedure [Gap 8]

**What:** Clear process for handling packet versions

**Fix (30 minutes):**
- Original packet is v1 (no suffix)
- If changes needed: Create v2 (add "REASON" in packet)
- Original locked packet gets REDIRECT: "Superseded by v2"
- Only latest version in TASK_BOARD

---

### P2-2: Audit Trail System [Gap 1]

**What:** Simple validation script to verify Orchestrator compliance

**Fix (1 hour):** Create shell script:
```bash
just orchestrator-audit WP-{ID}
# Checks:
- SPEC_ANCHOR exists in SPEC_CURRENT.md âœ“
- Signature format correct âœ“
- Signature not used before (grep) âœ“
- Blocker status in TASK_BOARD âœ“
- TASK_BOARD = packet STATUS âœ“
- Pre-Delegation checklist items present âœ“
```

**Where:** Create .GOV/scripts/validation/orchestrator-audit.sh

---

### P2-3: User Context Preservation [Gap 13]

**What:** Clarify how to preserve user-provided context in locked packets

**Fix (20 minutes):**
- Add section: "User Context = any notes user provided about work"
- Rule: "Preserve in packet NOTES; don't delete when locking"
- Example format shown

---

### P3-1 through P3-3: Polish Items [Gaps 14, 15, others]

Low-impact improvements (ERRATA for minor fixes, signature batching rules, etc.)

---

## Implementation Roadmap

### Week 1 (P0: Critical Foundations) â€” 3-4 hours
- [ ] Add "Clearly Covers" definition (30 min)
- [ ] Add Phase Closure Criteria (45 min)
- [ ] Add Error Recovery Procedures (1 hour)
- [ ] Add SLA for Gates (30 min)
- **Result: 87 â†’ 91/100 (A+ achieved)**

### Week 2 (P1: Quality Systems) â€” 2-3 hours
- [ ] Create Decision Log Template (1 hour)
- [ ] Add Risk Tier Matrix (45 min)
- [ ] Add Escalation Path + SLA (45 min)
- **Result: 91 â†’ 95/100**

### Week 3 (P2: Edge Cases) â€” 2-3 hours
- [ ] Add Variant Management Procedure (30 min)
- [ ] Create orchestrator-audit script (1 hour)
- [ ] Add User Context Preservation rules (20 min)
- [ ] Polish remaining items (30 min)
- **Result: 95 â†’ 99/100 (9.9/10 âœ¨)**

---

## Success Metrics

**After Week 1 (P0):** 91/100
- Clear definitions (no subjective decisions)
- Phase gates defined (know when to close)
- Error recovery documented (no panic on mistakes)

**After Week 2 (P1):** 95/100
- Audit trail enabled (verify Orchestrator compliance)
- Risk assessment objective (no disputes)
- Escalation path clear (know how to unblock)

**After Week 3 (P2):** 99/100 (9.9/10)
- Variant management clear (handle changes)
- User context preserved (governance complete)
- Edge cases handled (completeness)

---

## Cost Assessment

| Phase | Effort | LLM Tier | Cost |
|-------|--------|----------|------|
| P0 | 3-4 hours | Standard/Cheap | LOW |
| P1 | 2-3 hours | Standard/Cheap | LOW |
| P2 | 2-3 hours | Standard/Cheap | LOW |
| **Total** | **7-10 hours** | **Cheap** | **LOW** |

**All work is documentation + simple validation scripts. No product code. Perfect for cheaper LLM tier.**

---

## Conclusion

**From A- (87) to A+ (99) requires:**
- 15 hours of documentation work
- 3 simple shell scripts for validation
- No product code changes
- No expensive LLM tier needed

**This is pure coordination + clarity work. Ideal for cheaper LLM tier.**


