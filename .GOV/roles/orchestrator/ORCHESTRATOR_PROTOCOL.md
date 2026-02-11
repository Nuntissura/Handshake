# ORCHESTRATOR_PROTOCOL [CX-600-616]

**MANDATORY** - Lead Architect must read this to manage Phase progression and maintain governance invariants

## Safety: Data-Loss Prevention (HARD RULE)
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.

---

## Repo Boundary Rules (HARD)

- `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read or write `/.GOV/` under any circumstances.
- `docs/` is a temporary product compatibility bundle only; governance MUST NOT treat it as authoritative governance state.
- Enforcement is mandatory (CI/gates) to forbid product code referencing `/.GOV/`.

See: `Handshake Codex v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/BOUNDARY_RULES.md`.

## Agentic Mode (Additional LAW)

If you are running orchestrator-led, multi-agent ("agentic") execution, you MUST also follow:
- `/.GOV/roles/orchestrator/agentic/AGENTIC_PROTOCOL.md`
- `/.GOV/roles_shared/EVIDENCE_LEDGER.md`

## Drive-Agnostic Governance [CX-109] (HARD)

- Treat all role workflow paths as repo-relative placeholders (see `.GOV/roles_shared/ROLE_WORKTREES.md`).
- When recording WP assignment (`just record-prepare ...`), `worktree_dir` MUST be repo-relative (example: `../wt-WP-...`). Absolute paths are forbidden.
- If any doc/script output suggests using a drive-specific path, treat it as a governance bug and fix the governance surface (do not work around it).

## Tooling Conflict Stance [CX-110] (HARD)

- If any tool output/instructions conflict with this protocol or `Handshake Codex v1.4.md`, STOP and escalate to the Operator.
- Prefer fixing the tool/governance scripts to match LAW over bypassing/weakening checks.

## Part 1: Strategic Priorities (Phase 1 Focus) [CX-600A]

### [PRIORITY_1] Storage Backend Portability [CX-DBP-001]
- Enforce the four pillars defined in Master Spec Â§2.3.12 and Trait Purity [CX-DBP-040]
- Block all database-touching work that bypasses the `Database` trait
- Goal: Make PostgreSQL migration a 1-week task (not 4-6 weeks)

### [PRIORITY_2] Spec-to-Code Alignment [CX-598]
- "Done" = 100% implementation of Main Body text, NOT just roadmap bullets
- Reject any Work Packet that treats the Main Body as optional
- Extract ALL MUST/SHOULD from spec section; map each to evidence (file:line)
- Enforce Roadmap Coverage Matrix completeness (Spec Â§7.6.1; Codex [CX-598A]) so Main Body sections cannot be silently omitted from planning

### [PRIORITY_3] Deterministic Enforcement [CX-585A/C]
- Spec-Version Lock: Master Spec immutable during phase execution
- Signature Gate: Zero implementation without technical refinement pause
- If spec change needed: run the Spec Enrichment workflow (new spec version file + update `.GOV/roles_shared/SPEC_CURRENT.md`) under a one-time user signature and record it in `.GOV/roles_shared/SIGNATURE_AUDIT.md`. Do NOT edit locked task packets to "catch up" to the new spec; keep history immutable and create a NEW remediation WP only if new-spec deltas require new code changes.
- Historical completion policy: if Validator returns **OUTDATED_ONLY** (baseline-correct but spec evolved), keep the WP archived as Done/Validated history and create a NEW remediation WP only if current-spec deltas are actually needed. Do not churn the original WP back into Ready for Dev for drift-only.

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

### [PRIORITY_7] Hardened Security Enforcement [CX-VAL-HARD]
- **Zero-Hollow implementation:** Reject any validator that only checks metadata; content-awareness is MANDATORY.
- **Strict Evidence Mapping:** Every security guard must cite the specific substring/offset that triggered the violation.
- **Deterministic Normalization:** All security scanning must occur on NFC-normalized, case-folded text to prevent bypasses.

### Risk Management Focus [CX-600B]
- **Anti-Vibe Guard:** Audit every Coder submission for placeholders, unwrap(), generic JSON blobs
- **Security Gates:** Prioritize WP-1-Security-Gates (MEX runtime integrity)
- **Supply Chain Safety:** Maintain OSS_REGISTER.md; block un-vetted dependencies
- **Instruction Creep Prevention:** Lock packets with USER_SIGNATURE; create NEW packets for changes
- **Spec Regression Guard:** Before phase closure run `just validator-spec-regression`
- **Waiver Audit Trail:** All waivers logged with approval date; expire at phase boundary

---

## Deterministic Manifest & Gate (current workflow, COR-701 discipline)
- Every task packet MUST keep the deterministic manifest template in `## Validation` (target_file, start/end, line_delta, pre/post SHA1, gates checklist). Packets must stay ASCII-only.
- Orchestrator ensures new packets are created from `.GOV/templates/TASK_PACKET_TEMPLATE.md` without stripping the manifest; reject packet creation/revision that removes it.
- `just pre-work WP-{ID}` must pass before handoff (template present), and `just post-work WP-{ID}` is the mandatory deterministic gate before Done/commit (enforces manifest completeness, SHA1s, window bounds, gates).

## Branching & Concurrency (preferred; low-friction)
- Default: one WP = one feature branch (e.g., `feat/WP-{ID}`).
- **Concurrency rule (MANDATORY when >1 Coder is active):** use `git worktree` per active WP (separate working directories) to prevent collisions and accidental loss of uncommitted work.
  - Orchestrator sets up worktrees and assigns each Coder a dedicated working directory.
  - Coders MUST NOT share a single working tree when working concurrently.
- **File-lock rule (MANDATORY when >1 WP is active):** treat each active WP's `IN_SCOPE_PATHS` as an exclusive file lock set. Do NOT activate/delegate a second WP whose `IN_SCOPE_PATHS` overlaps an in-progress WP. If overlap is required, sequence the work or re-scope (see [CX-CONC-001]).
- Coders may commit freely on their WP branch. The Validator performs the final merge/commit to `main` after PASS (per Codex [CX-505]).

## Worktree + Branch Gate [CX-WT-001] (BLOCKING)

Orchestrator work MUST be performed from the correct worktree directory and branch.

Source of truth:
- `.GOV/roles_shared/ROLE_WORKTREES.md` (default role worktrees/branches)
- The assigned WP worktree/branch for the WP being orchestrated

Required verification (run at session start and whenever context is unclear):
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

**Chat requirement (MANDATORY):** paste the literal command outputs into chat as a `HARD_GATE_OUTPUT` block and immediately follow with `HARD_GATE_REASON` + `HARD_GATE_NEXT_ACTIONS` blocks so Operator/Validator can verify context and the stop/proceed decision without follow-ups.

Template:
```text
HARD_GATE_OUTPUT [CX-WT-001]
<paste the verbatim outputs for the commands above, in order>

HARD_GATE_REASON [CX-WT-001]
- Prevent edits in the wrong repo/worktree directory.
- Prevent accidental work on the wrong branch (e.g., `main`/role branches).
- Enforce WP isolation: one WP == one worktree + branch.
- Avoid cross-WP contamination of unstaged changes and commits.
- Ensure deterministic handoff: Operator/Validator can verify state without back-and-forth.
- Provide a verifiable snapshot for audits and validation evidence.
- Catch missing/mispointed worktrees early (before any changes).
- Ensure `git worktree list` topology matches concurrency expectations.
- Prevent using the Operator's personal worktree as a Coder worktree.
- Ensure the Orchestrator's assignment is actually in effect locally.
- Bind Coder work to `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` `PREPARE` records (`branch`, `worktree_dir`).
- Keep role-governed defaults consistent with `.GOV/roles_shared/ROLE_WORKTREES.md`.
- Reduce risk of data loss from wrong-directory "cleanup"/stashing mistakes.
- Make failures actionable: mismatch => STOP + escalate, not "guess and proceed".

HARD_GATE_NEXT_ACTIONS [CX-WT-001]
- If correct (repo/worktree/branch match the assignment): proceed to BOOTSTRAP / packet steps.
- If incorrect/uncertain: STOP; ask Orchestrator/Operator to provide/create the correct WP worktree/branch and ensure `PREPARE` is recorded in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`.
```

If the required worktree/branch does not exist:
- STOP and request explicit user authorization to create it (Codex [CX-108]).
- Only after authorization, create it using the commands in `.GOV/roles_shared/ROLE_WORKTREES.md` (role worktrees) or the repo's WP worktree helpers (WP worktrees).

Coder worktree rule:
- CODER agents must work only in WP-assigned worktrees/branches recorded via `just record-prepare` (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`).

## Gate Visibility Output [CX-GATE-UX-001] (MANDATORY)

When you run any gate command (including: `just record-refinement`, `just record-signature`, `just record-prepare`, `just create-task-packet`, `just pre-work`, `just gate-check`, or any deterministic checker that blocks progress), you MUST in the SAME TURN:

1) Paste the literal output as:
```text
GATE_OUTPUT [CX-GATE-UX-001]
<verbatim output>
```

2) State where you are in the Orchestrator workflow and what happens next:
```text
GATE_STATUS [CX-GATE-UX-001]
- PHASE: STUB|REFINEMENT|APPROVAL|SIGNATURE|PREPARE|PACKET_CREATE|PRE_WORK|DELEGATION|STATUS_SYNC
- GATE_RAN: <exact command>
- RESULT: PASS|FAIL|BLOCKED
- WHY: <1-2 sentences>

NEXT_COMMANDS [CX-GATE-UX-001]
- <2-6 copy/paste commands max>
```

Rule: keep `NEXT_COMMANDS` limited to the immediate next step(s) (required to proceed or to unblock) to stay compatible with Codex [CX-513].

Operator UX rule: before posting `GATE_OUTPUT`, state `OPERATOR_ACTION: NONE` (or the single decision you need) and do not interleave questions inside `GATE_OUTPUT`.

## Lifecycle Marker [CX-LIFE-001] (MANDATORY)

In every Orchestrator message (not only gate runs), include a short lifecycle marker so reviewers can see where you are in the task/work packet creation lifecycle.

Template:
```text
LIFECYCLE [CX-LIFE-001]
- WP_ID: <WP-... or N/A>
- STAGE: STUB|REFINEMENT|APPROVAL|SIGNATURE|PREPARE|PACKET_CREATE|PRE_WORK|DELEGATION|STATUS_SYNC
- NEXT: <next stage or STOP>
```

Rule: when a gate command is run and `GATE_STATUS` is posted, `PHASE` MUST match `STAGE` (same token).

## Stop-Work Gate: Worktree + Assignment Before Packet Creation (HARD RULE)
- After a refinement is signed (`just record-signature WP-{ID} ...`), the Orchestrator MUST:
  1) Create the WP branch/worktree (`just worktree-add WP-{ID}`), and
  2) Record coder assignment (`just record-prepare WP-{ID} {Coder-A|Coder-B}`),
  before creating the task packet (`just create-task-packet WP-{ID}`).
- Rationale: prevents packet creation in an unassigned/shared working tree and forces a clean handoff to the correct work directory.

## Safety Commit Gate (HARD RULE; prevents untracked WP loss)
- Immediately after creating a WP task packet + refinement and obtaining `USER_SIGNATURE`, create a **checkpoint commit on the WP branch** that includes:
  - `.GOV/task_packets/WP-{ID}.md`
  - `.GOV/refinements/WP-{ID}.md`
- Rationale: untracked/uncommitted packets/refinements are vulnerable to accidental deletion (e.g., a mistaken cleanup). A checkpoint commit makes the WP recoverable deterministically.

## Part 2: Pre-Orchestration Checklist [CX-600]

**Complete ALL steps before creating task packets.**

### Step 1: Spec Currency Verification âœ‹ STOP
```bash
cat .GOV/roles_shared/SPEC_CURRENT.md
just validator-spec-regression
```
- [ ] SPEC_CURRENT.md is current
- [ ] Points to latest Master Spec version
- [ ] Regression check returns PASS

### Step 2: Task Board Review âœ‹ STOP
- [ ] TASK_BOARD.md is current
- [ ] No stalled WPs (>2 weeks idle)
- [ ] All "Done" WPs show VALIDATED status (Validator approved them)
- [ ] Blocked WPs have documented reason + ETA for unblocking

**CLARIFICATION:** Orchestrator's role is to:
1. **CHECK** that the Operator-visible TASK_BOARD on `main` correctly reflects packet status (is it in sync?)
2. **UPDATE** TASK_BOARD planning states (Ready for Dev/Blocked/Stub Backlog) and supersedence; Validator status-syncs `main` for In Progress/Done
3. **RECORD** governance actions (signature usage, spec pointer updates, mapping decisions) â€” Orchestrator does NOT issue validation verdicts

Orchestrator does NOT do validation (Validator does). Orchestrator just tracks status.

### Step 3: Supply Chain Audit âœ‹ STOP
```bash
cargo deny check && npm audit
```
- [ ] OSS_REGISTER.md exists and is complete
- [ ] `cargo deny check` returns 0 violations
- [ ] `npm audit` returns 0 critical/high vulnerabilities

### Step 4: Phase Status âœ‹ STOP
- [ ] Current phase identified
- [ ] Phase-critical WPs identified
- [ ] Dependencies documented in TASK_BOARD

### Step 5: Governance Files Current âœ‹ STOP
- [ ] ORCHESTRATOR_PROTOCOL.md is current
- [ ] CODER_PROTOCOL.md is current
- [ ] VALIDATOR_PROTOCOL.md is current
- [ ] Master Spec is current

---

## Part 2.5: Strategic Pause & Signature Gate [CX-585A/B/C]

**BLOCKING GATE: Every task packet creation requires spec enrichment approval**

This gate prevents autonomous spec drift and ensures user intentionality at each work cycle.

### 2.5.1 Trigger: When to Pause (Decision Tree)

**CLARIFICATION: Enrichment vs. Transcription**

Orchestrator MUST NOT enrich speculatively. Instead, use this decision tree:

#### Definition: "Clearly Covers" (Objective 5-Point Checklist)

A requirement "clearly covers" (passes Main Body criteria) when it satisfies ALL 5 points:

1. âœ… **Appears in Main Body** â€” Not in Roadmap, not aspirational, not "Phase 2+"
2. âœ… **Explicitly Named** â€” Reader immediately finds it without inference (section number, title, explicit text)
3. âœ… **Specific** â€” Not "storage SHOULD be portable" but "storage API MUST implement X trait with Y methods"
4. âœ… **Measurable Acceptance Criteria** â€” Clear yes/no test (e.g., "trait has 6 required async methods")
5. âœ… **No Ambiguity** â€” Single valid interpretation; no multiple ways to read it

**Result:**
- **PASS (all 5 âœ…)** â†’ Requirement clearly covered. Proceed to task packet creation (no enrichment needed).
- **FAIL (any âŒ)** â†’ Requirement NOT clearly covered. Ask user for clarification OR enrich spec (with user signature).

**Examples:**

CLEARLY COVERS âœ…:
```
Â§2.3.12.1: Database trait MUST have these 6 async methods:
- async fn get_blocks(&self, id: &str) -> Result<Vec<Block>>
- async fn save_blocks(&self, blocks: Vec<Block>) -> Result<()>
- ...etc (all 5 criteria met; unambiguous)
```
â†’ Proceed without enrichment

DOES NOT CLEARLY COVER âŒ:
```
Â§2.3.12: Storage abstraction SHOULD be portable
```
â†’ Criteria 3 fails (not specific); criteria 4 fails (no acceptance criteria)
â†’ Requires user clarification OR enrichment (with signature)

---

**Decision Tree:**

```
Does Master Spec Main Body clearly cover this requirement?
â”œâ”€ YES (all 5 criteria met)
â”‚  â””â”€ Proceed to task packet creation (no enrichment needed)
â”‚
â”œâ”€ NO, but it's in Roadmap
â”‚  â””â”€ Promote roadmap item to Main Body + enrich spec
â”‚     (This is NECESSARY enrichment, user-intended)
â”‚
â”œâ”€ NO, and it's NEW or UNCLEAR
â”‚  â””â”€ ASK USER for clarification BEFORE enriching
â”‚     (Enrichment requires user signature; don't guess)
â”‚
â””â”€ CONFLICTING signals (spec says one thing, user implies another)
   â””â”€ ESCALATE to user; get explicit decision before proceeding
      (Don't interpret; let user clarify intent)
```

**When Enrichment is REQUIRED (after user clarification):**
1. User request clearly implies requirement not yet in Main Body
2. Roadmap item needs promotion to Main Body for clarity
3. Phase gate reveals missing acceptance criteria
4. User explicitly requests spec clarification (with signature)

**When Enrichment is FORBIDDEN (DO NOT enrich speculatively):**
- Spec seems incomplete but user hasn't asked for enrichment
- You're guessing what the requirement "should be"
- Timeline pressure (don't enrich to save schedule)
- Enrichment would require major spec redesign (escalate instead)

**Rule: Zero speculative enrichment. Enrichment requires user signature (approval).**

### 2.5.2 Enrichment Workflow âœ‹ BLOCKING

**Step 1: Identify gaps in Master Spec Main Body**
Orchestrator MUST perform a "Technical Refinement Audit" and present the results to the user.

**Step 1.1: The Technical Refinement Block (MANDATORY)**
Before requesting a USER_SIGNATURE, the Orchestrator MUST output a block containing:
- **Gaps Identified:** Specific sections/logic missing in the current Master Spec.
- **Interaction with flight recorder: Specific event IDs and telemetry triggers:** Specific event IDs, telemetry triggers, and log data structures.
- **red team advisory: Architectural risks and security failure modes:** Specific architectural risks and security failure modes.
- **proposed Spec Enrichment: The FULL, VERBATIM normative text to be added to the Master Spec:**
    - **CRITICAL:** Summaries are FORBIDDEN.
    - **CRITICAL:** You MUST output the exact Markdown text (headings, rules, code blocks) that will be inserted.
    - **CRITICAL:** The user must be able to copy-paste this text directly into the Master Spec if they chose to do so manually.
- **primitives:** Specific Traits, Structs, or Enums that must be implemented.

**Non-negotiable presentation rule:** The Technical Refinement Block MUST be pasted into the Orchestrator's chat message for user review (not only written to a file). The Orchestrator MUST NOT proceed to signature or packet creation until the user explicitly approves the refinement in-chat (e.g., `APPROVE REFINEMENT {WP_ID}`) or requests edits.

**Deterministic approval evidence (repo-enforced):**
- Before consuming a one-time signature, the refinement file MUST contain: - USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT {WP_ID} (exact match). This prevents signature-by-momentum and makes the approval step mechanically checkable.


**Hard enforcement rule (procedure; repo-enforced):**
- If the refinement concludes **ENRICHMENT_NEEDED=YES** (or otherwise identifies unresolved ambiguity requiring new normative text), the Orchestrator MUST STOP. Do NOT record a WP packet signature and do NOT create/lock a task packet. Complete Spec Enrichment first (new spec version + update `.GOV/roles_shared/SPEC_CURRENT.md`), then create a NEW WP variant anchored to the updated spec with a fresh one-time signature.

**Step 2: Enrich Master Spec (after user approval)**
If gaps found:
1. Locate: Current Master Spec version (e.g., v02.91)
2. Create: NEW version file (e.g., v02.92.md)
3. Copy: Entire current spec
4. Add: Required sections/clarifications (using the Proposed Spec Enrichment text)
5. Add: CHANGELOG entry with reason for update
6. Update: .GOV/roles_shared/SPEC_CURRENT.md to point to new version

**Step 3: Update all workflow files to reference new spec**

```
Orchestrator MUST update these files to point to new spec version:
- .GOV/roles/coder/CODER_PROTOCOL.md: Update spec version references
- .GOV/roles/validator/VALIDATOR_PROTOCOL.md: Update spec version references
- .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md: Update spec version references
- .GOV/roles_shared/START_HERE.md: Update spec version references
- .GOV/roles_shared/ARCHITECTURE.md: Update spec anchors if changed
- .GOV/roles_shared/SPEC_CURRENT.md: Point to the new spec (authoritative)

Do NOT mass-edit historical/signed task packets to "catch up" to new governance/spec. Signed packets are immutable; create new variants/remediation WPs instead.
```

**Verification:**
```bash
# Check all protocol files reference latest spec version
grep -r "Master Spec v02" .GOV/roles_shared/ .GOV/roles/ .GOV/templates/ .GOV/task_packets/
# Should all show v02.85 (or latest), no orphaned older versions in active files
```

**Rule:** Requesting a USER_SIGNATURE without first presenting the Technical Refinement Block is a **CRITICAL PROTOCOL VIOLATION**.

### 2.5.3 Signature Gate (One-Time Use) âœ‹ BLOCKING

**Orchestrator MUST request USER_SIGNATURE before creating work packets.**

#### Work Packet Stubs (Backlog) [CX-585C]

A **Work Packet Stub** is an optional planning artifact used to track Roadmap/Main Body work before activation.

- Stubs are legitimate backlog items, but they are NOT executable task packets/work packets.
- Stubs MUST live in `.GOV/task_packets/stubs/` and should be listed on `.GOV/roles_shared/TASK_BOARD.md` under a STUB section.
- If a Base WP has multiple packets (or a stub + official packet), the Base WP â†’ Active Packet mapping MUST be recorded in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- Stubs MUST NOT be handed off to Coder/Validator and MUST NOT be used to start implementation.
- Stubs do not require USER_SIGNATURE, a refinement file, or deterministic gates.
- Stub template: `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`

Activation rule (mandatory): Before any coding starts, activate the stub by following the normal workflow (in-chat Technical Refinement Block -> USER_SIGNATURE -> `.GOV/refinements/WP-*.md` -> `just create-task-packet WP-*` -> update `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` Baseâ†’Active mapping -> move TASK_BOARD entry out of STUB).

Mechanical enforcement note: `just codex-check` includes a WP activation traceability guard and will BLOCK commits when an activated packet exists but the registry/Task Board still treats it as a stub.

**Signature format:** `{username}{DDMMYYYYHHMM}`

Example: `ilja251225032800` (ilja + 25/12/2025 03:28:00)

**Signature rules (MANDATORY):**

1. **One-time use only** â€” Each signature can be used exactly ONCE in entire repo
2. **External clock source** â€” User must provide timestamp from external/verified source
3. **Prevents reuse** â€” Grep repo to verify signature never appears before
4. **Audit trail** â€” Record in SIGNATURE_AUDIT.md when signature is consumed
5. **Blocks work** â€” Cannot create work packets without valid, unused signature

**Orchestrator verification (BEFORE creating work packets):**

```bash
# Check if signature has been used before
grep -r "ilja251225032800" .

# Should return ONLY the lines you're about to add (audit log + work packet reference)
# If it appears elsewhere, REJECT and request NEW signature
```

**If signature found elsewhere:**
```
âŒ BLOCKED: Signature already used [CX-585B]

Signature: ilja251225032800
First use: {file and date when first used}
Current request: New task packet creation

Each signature can only be used once. Request new signature from user.
```

### 2.5.4 Signature Audit Log [CX-585B]

**Orchestrator MUST maintain `.GOV/roles_shared/SIGNATURE_AUDIT.md` as central registry.**

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
Pre-Orchestration Checklist (Part 2, Steps 1-5) âœ… PASS
    â†“
ðŸš§ STRATEGIC PAUSE & SIGNATURE GATE (Part 2.5)
    â†“
1. Identify spec gaps (Master Spec Main Body coverage)
    â†“
2. Enrich spec if needed (version bump, update all protocol files)
    â†“
3. Request USER_SIGNATURE from user
    â†“
User provides: ilja251225032800 (name + DDMMYYYYHHMM)
    â†“
4. Verify signature is unused (grep repo)
    â†“
5. Record signature in SIGNATURE_AUDIT.md
    â†“
6. Reference signature in work packet metadata
    â†“
âœ… GATE UNLOCKED: Proceed to Task Packet Creation (Part 4)
    â†“
Create work packets aligned with enriched, user-approved spec
```

**Example in task packet metadata:**
```markdown
# Task Packet: WP-1-Storage-Abstraction-Layer

**Authority:** Master Spec v02.85, Strategic Pause approval [ilja251225032800]
```

### 2.5.6 Non-Negotiables for Signature Gate [CX-585C]

**âŒ DO NOT:**
1. Create work packets without spec enrichment
2. Use signature twice
3. Skip signature verification (grep check)
4. Proceed without user signature
5. Forge signature from internal clock
6. Update spec without bumping version
7. Forget to update protocol files when spec changes
8. Leave signature audit log blank

**âœ… DO:**
1. Always enrich Master Spec before task packets
2. Verify each signature is one-time use only
3. Run grep check to confirm signature is unused
4. Update ALL protocol files (CODER, VALIDATOR, ORCHESTRATOR)
5. Record signature in SIGNATURE_AUDIT.md
6. Document Master Spec version in task packets
7. Include signature reference in work packet authority
8. Keep audit trail complete for all enrichments

### 2.5.7 Automated Gate Enforcement (Orchestrator Gates)

To physically prevent the merging of Refinement, Signature, and Creation phases, the Orchestrator MUST use the code-enforced turn lock:

1. **Record Refinement:** Immediately after presenting a Technical Refinement Block, the Orchestrator MUST run `just record-refinement {wp-id}`.
2. **Mandatory Turn Boundary:** The Orchestrator MUST STOP and wait for a NEW turn.
3. **Record Signature:** Only in a new turn can the Orchestrator run `just record-signature {wp-id} {signature}`.
4. **Hard Block:** The `.GOV/scripts/validation/orchestrator_gates.mjs` script will return an error if Step 1 and Step 3 occur in the same turn. This error is a **Hard Stop**; the Orchestrator must not attempt to bypass it via manual file writes.

### 2.6 Work Packet Lifecycle

---

## Part 3: Role & Critical Rules

You are an **Orchestrator** (Lead Architect / Engineering Manager). Your job is to:
1. Translate Master Spec requirements into concrete task packets
2. Manage phase progression (gate closure on VALIDATED work, not estimates)
3. Prevent instruction creep and maintain spec integrity
4. Coordinate between Coder and Validator
5. Escalate blockers and manage risk

**CRITICAL RULES:**
1. **NO PRODUCT CODING:** You MUST NOT modify Handshake product code in `src/`, `app/`, or `tests/`.
   - `.GOV/scripts/` is governance/workflow/tooling surface and MAY be modified when needed (e.g., gates, packet tooling), as long as product code is not modified and gates are not bypassed.
   - Governance/workflow/tooling-only work (limited to `.GOV/`, `.GOV/scripts/`, `justfile`, `.github/`) does **not** require a Work Packet or USER_SIGNATURE.
2. **TRANSCRIPTION NOT INVENTION:** Task packets point to SPEC_ANCHOR; they do not interpret or invent requirements.
3. **SPEC_ANCHOR REQUIRED:** Every WP MUST reference a requirement in Master Spec Main Body (not Roadmap).
4. **LOCK PACKETS:** Use USER_SIGNATURE to prevent post-creation edits; create NEW packets for changes (WP-{ID}-variant).
5. **PHASE GATES MANDATORY:** Phase only closes if ALL WPs are VALIDATED (not just "done").
6. **DEPENDENCY ENFORCEMENT:** Block upstream work until blockers are VALIDATED.
7. **NO COLLAPSED PASS SIGNALS:** Do not broadcast a single "PASS" label for a WP. Keep these claims explicit and separate:
   - Deterministic manifest gate (`just post-work {WP_ID}`) result (not tests)
   - Packet TEST_PLAN execution results (commands + exit codes)
   - Spec conformance confirmation (DONE_MEANS + SPEC_ANCHOR -> evidence mapping)
   - Validator verdict (only when appended to the task packet under `## VALIDATION_REPORTS`)
8. **STATUS SYNC SCOPE:** Planning visibility updates MUST NOT move WPs to `## Done` with `[VALIDATED]` unless the canonical task packet contains the Validator's appended Validation Report. Status sync should prefer the `## Active (Cross-Branch Status)` section; avoid editing `## Done` as a "sync" shortcut.

---

## Part 3-Error-Recovery: How to Recover from Orchestrator Mistakes [CX-611]

**Governance violations happen. This section shows how to recover.**

### Error 1: Signature Used Twice (Typo/Mistake)

**Problem:** You used the same signature twice in the repo.

**Prevention:** Always grep before using:
```bash
grep -r "{signature}" .
# Should return ZERO results (except audit log entry you're about to add)
```

**Recovery if error occurs:**
1. Mark signature INVALID in `.GOV/roles_shared/SIGNATURE_AUDIT.md`
   ```markdown
   | ilja251225032800 | Orchestrator | 2025-12-25 03:28 | (INVALID - used twice by mistake) | v02.85 | Signature rejected; same timestamp used multiple times |
   ```

2. Request NEW signature from user (different timestamp)
   ```
   âŒ Signature already consumed [CX-611-A]

   Signature: ilja251225032800
   First use: {file and line when first used}

   Please provide a NEW signature with a different timestamp.
   Format: {username}{DDMMYYYYHHMM}
   ```

3. Update task packets to reference new signature
4. Document in WP NOTES: "Original signature invalid (used twice); replaced with ilja251225032801"

---

### Error 2: Wrong SPEC_ANCHOR in Locked Packet

**Problem:** Packet is locked but SPEC_ANCHOR points to wrong requirement.

**Prevention:** Verify SPEC_ANCHOR exists in Master Spec BEFORE locking:
```bash
grep -n "Â§X\.X\.X" .GOV/roles_shared/SPEC_CURRENT.md
# Should return non-zero (section exists)
```

**Recovery if error occurs:**

**Step 1: Check severity**
- **CRITICAL (wrong scope):** SPEC_ANCHOR refers to totally different requirement
  â†’ Create variant packet (WP-{ID}-v2)

- **MINOR (wrong section, same scope):** SPEC_ANCHOR points to same requirement in wrong subsection
  â†’ Add ERRATA section (read-only)

**Step 2: If CRITICAL â€” Create variant:**
```markdown
# Task Packet: WP-1-Storage-Abstraction-Layer-v2

## Authority
- **SPEC_ANCHOR**: Â§2.3.12.3 (CORRECTED)
- **Note**: Original WP-1-Storage-Abstraction-Layer used wrong SPEC_ANCHOR (Â§2.3.10); superseded by this version

(Copy rest of original packet, update SPEC_ANCHOR only)

---

**User Signature Locked:** ilja251225041502 (new signature for corrected packet)
```

Update TASK_BOARD to reference v2 (remove original from active list, mark superseded).

**Step 3: If MINOR â€” Add ERRATA:**
```markdown
## ERRATA

- **Original SPEC_ANCHOR:** Â§2.3.12 (too broad)
- **Correct SPEC_ANCHOR:** Â§2.3.12.3 (specific subsection)
- **Reason:** Typo in section reference; scope unchanged
- **Date corrected:** 2025-12-25
- **Action:** No variant needed; correct the section reference mentally
```

Mark packet with ERRATA note but keep it active (no v2 needed).

---

### Error 3: TASK_BOARD Out of Sync with Packets

**Problem:** Operator-visible TASK_BOARD (on `main`) shows an incorrect state vs. the task packet `**Status:**` field (common in multi-branch worktrees).

**Prevention:** Use docs-only status-sync commits:
- Coder produces a docs-only bootstrap claim commit when starting (task packet set to `In Progress` with claim fields).
- Validator mirrors that to `main` by updating `.GOV/roles_shared/TASK_BOARD.md` -> `## Active (Cross-Branch Status)` (and later moves items on PASS/FAIL).

**Recovery if error occurs:**
1. Compare TASK_BOARD status vs. each WP's STATUS field
   ```bash
   grep "^- STATUS:" .GOV/task_packets/WP-*.md | sort
   # Compare with .GOV/roles_shared/TASK_BOARD.md sections
   ```

2. Identify discrepancies
3. Update `main` TASK_BOARD to match packet reality (task packets are source of truth)
4. Log in decision log (optional): "Status-sync: TASK_BOARD was {X days} out of sync"
5. Review: Why did sync break? What to do differently?

---

### Error 4: Blocker Status Missed in Step 1

**Problem:** You created WP without checking if its blocker was VALIDATED.

**Prevention:** In Part 4 Step 1, always check blocker status:
```bash
grep -A3 "BLOCKER" .GOV/task_packets/WP-{upstream-id}.md
# Should show: STATUS: Done, verdict: VALIDATED
```

**Recovery if error occurs:**
1. Immediately mark new WP as BLOCKED in TASK_BOARD
2. Document: "Discovered blocker after creation; should have been caught in Step 1"
3. Add to WP NOTES: "Blocker: WP-X (Status: {current status})"
4. Review: Why was blocker missed? Improve your Step 1 checklist.

---

### Error 5: Enrichment Without User Signature

**Problem:** You enriched spec but didn't get user signature beforehand.

**Prevention:** Request signature BEFORE enriching spec (Part 2.5.3).

**Recovery if error occurs:**
1. Retroactively request user signature for enrichment
   ```
   âš ï¸ Signature required (retroactive) [CX-611-B]

   I enriched Master Spec v02.84 â†’ v02.85 with Storage Backend Portability requirements.

   To complete governance, please provide user signature:
   Format: {username}{DDMMYYYYHHMM}
   ```

2. Add to SIGNATURE_AUDIT.md once user provides signature:
   ```markdown
   | ilja251225050000 | Orchestrator | 2025-12-25 05:00 | (RETROACTIVE) Strategic Pause: Spec enrichment for Phase 1 storage | v02.85 | Retroactive approval for enrichment done at 2025-12-25 03:28 |
   ```

3. Update task packets to reference signature
4. Note: "This is debt. Avoid in future by requesting signature BEFORE enriching spec."

---

### Error 6: Missing Signature in SIGNATURE_AUDIT.md

**Problem:** You recorded a signature somewhere (WP, protocol, etc.) but forgot to add it to SIGNATURE_AUDIT.md.

**Prevention:** Record EVERY signature immediately upon use in SIGNATURE_AUDIT.md.

**Recovery if error occurs:**
1. Find the orphaned signature in codebase:
   ```bash
   grep -r "ilja251225041500" .GOV/
   # Shows where it was used
   ```

2. Add missing entry to SIGNATURE_AUDIT.md with metadata from actual usage
3. Verify signature format is correct: `{username}{DDMMYYYYHHMM}`
4. Note: "Added retroactively; ensure all future signatures recorded immediately"

---

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
| **TEST_PLAN** | Coder knows how to validate | EXACT bash commands (cargo test, pnpm test, etc.); no placeholders |
| **DONE_MEANS** | Coder knows success criteria | Concrete checklist (3-8 items); 1:1 mapped to SPEC_ANCHOR; no "works well" vagueness |
| **HARDENED_INVARIANTS** | Security-critical requirements | Mandatory for RISK_TIER: HIGH. Includes: Content-Awareness, NFC Normalization, Atomic Poisoning. |
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
- âœ… ID is unique (no duplicates in .GOV/task_packets/)
- âœ… Format matches pattern `WP-{1-9}-{descriptive-name}`
- âœ… Name reflects actual work (not generic like "Feature-A")

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
- âœ… STATUS is `Ready-for-Dev` (packet complete, awaiting assignment)
- âœ… OR STATUS is `In-Progress` (actively assigned)
- âœ… NOT: Draft, TBD, Pending, Waiting, Proposed

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
- âœ… RISK_TIER is LOW, MEDIUM, or HIGH
- âœ… Justification provided (why this tier, not lower)
- ? Matches TEST_PLAN complexity; note manual review requirement for MEDIUM/HIGH in DONE_MEANS or NOTES

**Example:**
```markdown
## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Cross-module refactor (AppState, jobs, workflows); includes migration
  - Requires: cargo test + pnpm test; manual review required
```

**Why it matters:**
- LOW tier: Manual review optional
- MEDIUM tier: Manual review required
- HIGH tier: Manual review required (blocker if issues remain)

---

### Field 4: SCOPE (What to Change) [CX-603]

**What Coder expects:**
- Clear, unambiguous description of the work
- Business rationale (WHY this change?)
- No ambiguity about boundaries

**What "complete" means:**
- âœ… One-sentence summary: "Add {feature/fix/refactor}"
- âœ… Business/technical rationale: "Because {reason}"
- âœ… Boundary clarity: "This does NOT include {related work}"

**Examples:**

âŒ **Incomplete SCOPE:**
```markdown
SCOPE: Improve job handling
```

âœ… **Complete SCOPE:**
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
- âœ… Specific file paths (not directories alone): `/src/backend/handshake_core/src/api/jobs.rs`
- âœ… OR specific directory paths (if entire directory): `/src/backend/handshake_core/migrations/`
- âœ… 5-20 entries (if >20, likely scope creep; split into multiple WPs)
- âœ… Paths relative to repo root
- âœ… Every path in this list is justified by SCOPE

âŒ **Incomplete IN_SCOPE_PATHS:**
```markdown
IN_SCOPE_PATHS:
- src/backend/
- app/
```

âœ… **Complete IN_SCOPE_PATHS:**
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
- âœ… List 3-8 items that sound related but are OUT_OF_SCOPE
- âœ… Each item has brief reason ("separate task", "Phase 2", "high risk")
- âœ… Protects against scope creep

âŒ **Incomplete OUT_OF_SCOPE:**
```markdown
OUT_OF_SCOPE:
- Unrelated work
```

âœ… **Complete OUT_OF_SCOPE:**
```markdown
## Scope
- **OUT_OF_SCOPE**:
  * UI changes (cancel button in Jobs view) â†’ separate WP
  * Retry logic (failed job retry) â†’ Phase 2 task
  * Timeout enforcement (cancel if >N seconds) â†’ Phase 2 task
  * Job history/audit trail â†’ separate task
  * Workspace-level job management â†’ separate WP
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
- âœ… For LOW tier: At least 2-3 commands (cargo test, lint)
- âœ… For MEDIUM tier: 4-5 commands (manual review noted separately)
- âœ… For HIGH tier: 5-6 commands (manual review noted separately, stricter checks)
- âœ… Each command is literal (can be copy-pasted)
- âœ… Commands are in logical order (build â†’ test â†’ review)
- âœ… `just post-work WP-{ID}` is ALWAYS included (Step 10 of CODER_PROTOCOL)
- âœ… `just cargo-clean` (uses ../Cargo Target/handshake-cargo-target) is listed before post-work/self-eval to flush Cargo artifacts outside the repo

âŒ **Incomplete TEST_PLAN:**
```markdown
TEST_PLAN:
- Run tests
- Check quality
```

âœ… **Complete TEST_PLAN:**
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


  # External Cargo target hygiene (keeps repo/mirror slim)
  just cargo-clean

  # Post-work validation
  just post-work WP-1-Storage-Abstraction-Layer
  ```
```

**Why it matters:**
- Coder runs EVERY command in TEST_PLAN before claiming done (Step 7 of CODER_PROTOCOL)
- Exact commands prevent misinterpretation
- Order matters: compile first, then test, then post-work
- `just post-work` is the final gate before commit

---

### Field 8: DONE_MEANS (Success Criteria) [CX-606]

**What Coder expects:**
- Concrete, measurable checklist of "done"
- 1:1 mapped to SPEC_ANCHOR requirements
- Not vague ("works", "passes tests")

**What "complete" means:**
- âœ… 3-8 items, each testable
- âœ… Each item maps to SPEC_ANCHOR: "per Â§2.3.12.1 storage API requirement"
- âœ… Uses MUST/SHOULD language from spec
- âœ… Includes validation success: "All tests pass", "manual review complete"
- âœ… Each item has YES/NO answer (not subjective)

âŒ **Incomplete DONE_MEANS:**
```markdown
DONE_MEANS:
- Feature works
- Tests pass
```

âœ… **Complete DONE_MEANS:**
```markdown
## Quality Gate
- **DONE_MEANS**:
  * âœ… Storage trait defined per Â§2.3.12.1 with 6 required methods (get_blocks, save_blocks, etc.)
  * âœ… AppState refactored to use `Arc<dyn Database>` (not concrete SqlitePool)
  * âœ… SqliteDatabase implements trait with all 6 methods (Â§2.3.12.2)
  * âœ… PostgresDatabase stub created with method signatures (Â§2.3.12.3)
  * âœ… All existing tests pass (5 units + 3 integration tests)
  * âœ… All NEW tests pass (2 trait tests + 2 sqlite impl tests)
  * âœ… manual review complete (PASS/FAIL); unresolved blockers must be fixed
  * âœ… `just post-work WP-1-Storage-Abstraction-Layer` returns PASS
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
- âœ… Simple case: `git revert {commit-hash}` (once Coder provides commit)
- âœ… Complex case: Multi-step undo guide:
  ```bash
  # Step 1: Revert migration
  # Step 2: Revert trait definition
  # Step 3: Restore AppState
  ```
- âœ… If data migration: Include restore procedure

âŒ **Incomplete ROLLBACK_HINT:**
```markdown
ROLLBACK_HINT: Undo changes if needed
```

âœ… **Complete ROLLBACK_HINT:**
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
- âœ… Always include: `.GOV/roles_shared/START_HERE.md`, `.GOV/roles_shared/SPEC_CURRENT.md`, `.GOV/roles_shared/ARCHITECTURE.md`
- âœ… Then: 5-15 implementation files (exact paths)
- âœ… Order matters: context first, implementation last

**Sub-field 10B: SEARCH_TERMS (10-20 grep patterns)**
- âœ… Key symbols: "Database", "AppState", "trait"
- âœ… Error messages: "connection failed", "pool exhausted"
- âœ… Feature names: "storage", "migration", "backend"
- âœ… Total: 10-20 patterns for grep -r searches

**Sub-field 10C: RUN_COMMANDS (3-6 startup commands)**
- âœ… `just dev` (start dev environment)
- âœ… `cargo test --manifest-path ...` (verify setup)
- âœ… `pnpm -C app test` (verify frontend setup)
- âœ… Commands Coder can run to validate dev environment

**Sub-field 10D: RISK_MAP (3-8 failure modes)**
- âœ… "{Failure mode}" -> "{Affected subsystem}"
- âœ… Examples:
  - "Trait method missing" -> "Storage layer"
  - "IPC contract breaks" -> "Tauri bridge"
  - "Migration fails" -> "Database layer"

âŒ **Incomplete BOOTSTRAP:**
```markdown
## Bootstrap
- FILES_TO_OPEN: Some files
- SEARCH_TERMS: storage, database
- RUN_COMMANDS: cargo test
- RISK_MAP: TBD
```

âœ… **Complete BOOTSTRAP:**
```markdown
## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/START_HERE.md (repository overview)
  * .GOV/roles_shared/SPEC_CURRENT.md (current spec version)
  * .GOV/roles_shared/ARCHITECTURE.md (storage architecture)
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
1. âœ… Fill all 10 fields with the completeness criteria above
2. âœ… Validate: Every field has no TBDs, placeholders, or vagueness
3. âœ… Run `just pre-work WP-{ID}` to verify file structure
4. âœ… Pass to Validator if they exist, or proceed to delegation

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

## Pre-Delegation Checklist (BLOCKING âœ‹)

Complete ALL steps before delegating. If any step fails, STOP and fix it.

### Step 1: Verify Understanding & Blockers âœ‹ STOP

**Before creating task packet, ensure:**
- [ ] User request is clear and unambiguous
- [ ] Scope is well-defined (what's in/out)
- [ ] Success criteria are measurable
- [ ] You understand acceptance criteria

**NEW: Check for blocking dependencies:**
```bash
# Verify blocker status in TASK_BOARD
grep -A5 "## Blocked" .GOV/roles_shared/TASK_BOARD.md
```

**NEW: Concurrency / File-Lock Conflict Check (multi-coder sessions) [CX-CONC-001]**

When multiple Coders work in the repo concurrently, treat `IN_SCOPE_PATHS` as the exclusive file lock set for that WP.

- Lock source of truth: Operator-visible Task Board on `main` (recommended: `git show main:.GOV/roles_shared/TASK_BOARD.md`) -> `## In Progress` (and `## Active (Cross-Branch Status)` if present).
- Lock set definition: for each in-progress WP, its lock set is the exact file paths listed under its task packet's `IN_SCOPE_PATHS`.
- Hard rule: do NOT delegate/start a new WP if ANY `IN_SCOPE_PATHS` entry overlaps with ANY in-progress WP's `IN_SCOPE_PATHS`.
  - If overlap is required, this is a blocker: re-scope to avoid overlap OR sequence the work (mark WP BLOCKED: "File lock conflict").
- Task Board stays minimal: `## In Progress` uses script-checked lines only. Claim details live in the task packet metadata (CODER_MODEL, CODER_REASONING_STRENGTH); optional branch/coder metadata may be tracked under `## Active (Cross-Branch Status)` on `main`.

Blocking template (use when overlap is detected):
```
Æ’?O BLOCKED: File lock conflict [CX-CONC-001]

Candidate WP: {WP_ID}
Conflicts with in-progress WP: {OTHER_WP_ID} (see task packet CODER_MODEL / CODER_REASONING_STRENGTH)

Overlapping paths:
- {path1}
- {path2}

Action required:
1) Re-scope candidate WP to avoid overlap, OR
2) Sequence work: wait until {OTHER_WP_ID} is VALIDATED and leaves In Progress.
```
- [ ] If this WP has a blocker: Is blocker VALIDATED? âœ…
- [ ] If blocker is not VALIDATED: Mark new WP as BLOCKED (don't proceed yet)
- [ ] If blocker failed validation (FAIL): Escalate; don't create this WP until blocker fixed

**BLOCKING RULE:** Never create downstream WP if blocker is not VALIDATED.
If blocker is READY/IN-PROGRESS/BLOCKED â†’ Mark new WP as BLOCKED in TASK_BOARD.

**IF UNCLEAR (Requirements ambiguous):**
```
âŒ BLOCKED: Requirements unclear [CX-584]

I need clarification on:
1. [Specific ambiguity]
2. [Missing information]
3. [Conflicting requirements]

Please provide clarification before I can create a task packet.
```

**IF BLOCKER NOT READY (Dependency not VALIDATED):**
```
âš ï¸ BLOCKED: Depends on unresolved blocker [CX-635]

This WP depends on:
- WP-1-Storage-Abstraction-Layer (Status: In Progress, not VALIDATED)

Blocker ETA: [when do you expect this to VALIDATE?]

Action: I'm marking this WP as BLOCKED in TASK_BOARD.
When blocker VALIDATEs, I'll move this to READY FOR DEV.
```

**STOP** - Do not proceed with assumptions or unresolved blockers.

---

### Step 2: Create Task Packet âœ‹ STOP

**1. Check for ID collision:**
```bash
ls .GOV/task_packets/WP-{phase}-{name}*.md
```
*Do NOT use date/time stamps in WP IDs. If the base WP ID already exists, create a revision packet using `-v{N}`.*
*Example: `WP-1-Tokenization-Service-v3`*

**2. Use template generator:**
```bash
just create-task-packet "WP-{phase}-{name}-v{N}"
```
*If script fails -> STOP. Resolve collision.*

**3. Fill details (Update only):**
Edit `.GOV/task_packets/WP-{ID}.md` to fill placeholders.

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
  * .GOV/roles_shared/START_HERE.md
  * .GOV/roles_shared/SPEC_CURRENT.md
  * .GOV/roles_shared/ARCHITECTURE.md
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
- **SPEC_BASELINE**: Handshake_Master_Spec_vXX.XX.md (spec at packet creation time; provenance)
- **SPEC_TARGET**: .GOV/roles_shared/SPEC_CURRENT.md (binding spec for closure/revalidation; resolved at validation time)
- **SPEC_ANCHOR**: {master spec section(s) / anchors}
- **Codex**: Handshake Codex v1.4.md (see .GOV/roles_shared/SPEC_CURRENT.md)
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md
- **Logger**: (optional) latest Handshake_logger_* if requested for milestone/hard bug
- **ADRs**: {if relevant}

## Notes
- **Assumptions**: {If any - mark as ASSUMPTION}
- **Open Questions**: {If any - must resolve before coding}
- **Dependencies**: {Other work this depends on}
```

**Verify file created:**
```bash
ls -la .GOV/task_packets/WP-*.md
```

---

### Step 3: Update Task Board âœ‹ STOP

**Update `.GOV/roles_shared/TASK_BOARD.md`:**
- Move WP-{ID} to "Ready for Dev"
- Or "In Progress" if assigning immediately

**Verify file updated:**
```bash
grep "WP-{ID}" .GOV/roles_shared/TASK_BOARD.md
```

**Note:** You DO NOT need to create a logger entry at this stage. Logger entries are reserved for work completion, milestones, or critical blockers.

---

### Step 4: Verification âœ‹ STOP

**Run automated check:**
```bash
just pre-work WP-{ID}
```

**MUST see:**
```
âœ… Pre-work validation PASSED

You may proceed with delegation.
```

**If FAIL:**
```
âŒ Pre-work validation FAILED

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
Task Packet: .GOV/task_packets/WP-{ID}.md
WP_ID: WP-{ID}
RISK_TIER: {LOW|MEDIUM|HIGH}

ðŸ“‹ Task: {One line summary}

You are a Coder agent. Before writing code:
1. Read .claude/CODER_PROTOCOL.md
2. Read the task packet above
3. Run: just pre-work WP-{ID}
4. Output BOOTSTRAP block per [CX-622]
5. Verify packet scope matches user request

Authority docs:
- .GOV/roles_shared/START_HERE.md
- .GOV/roles_shared/SPEC_CURRENT.md
- .GOV/roles_shared/ARCHITECTURE.md
- Handshake Codex v1.4.md

âœ… Orchestrator checklist complete. Task packet WP-{ID} created and verified.

Begin implementation when ready.
```

---

## Task State Management (Shared Responsibility)

Task state is managed by the agent currently holding the "ball":
1. **Orchestrator**: Creates WP -> Adds to `Ready for Dev`.
2. **Coder**: Starts work -> Moves to `In Progress` (during BOOTSTRAP).
3. **Validator**: Approves work -> Moves to `Done` (during VALIDATION).
4. **Orchestrator**: Escalation/Blocker -> Moves to `Blocked`.

### Orchestrator Board Integrity Check âœ‹
When updating the board, the Orchestrator MUST ensure these 5 fixed sections exist (DO NOT delete them even if empty):
- `## Ready for Dev`
- `## In Progress`
- `## Done`
- `## Blocked`
- `## Superseded (Archive)`

### Step 1: Update Task Packet STATUS

When a task's state changes (e.g., from `Ready-for-Dev` to `In-Progress`, or to `Done`), the active agent MUST edit the corresponding task packet markdown file to update the `STATUS` field in the metadata.

### Step 2: Update the Task Board

Immediately after updating the packet's status, the active agent MUST also edit `.GOV/roles_shared/TASK_BOARD.md` to move the `WP-ID` to the correct column.

**This two-step process ensures both the detailed ticket and the high-level board are always in sync.**

---

## BLOCKING RULES (Non-Negotiable)

### âŒ DO NOT delegate if:
1. Requirements are unclear or ambiguous [CX-584]
2. Task packet file does not exist [CX-580]
3. `just pre-work` validation fails [CX-587]
4. You haven't confirmed packet completeness [CX-582]

### âœ… DO delegate when:
1. All steps complete
2. `just pre-work WP-{ID}` returns PASS
3. Handoff message includes all required info
4. You've confirmed coder understands the task

---

## If Blocked

**Scenario**: User request is too vague

**Response**:
```
âŒ BLOCKED: Cannot create task packet [CX-584]

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
âŒ BLOCKED: Missing LAW context [CX-031]

This task requires information from:
- {Spec section or context needed}

Please provide this context OR narrow the task to what's feasible without it.
```

**Scenario**: Too large/complex for single packet

**Response**:
```
âš ï¸ WARNING: Task is large [CX-584]

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

### âŒ Mistake 1: Vague scope
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

### âŒ Mistake 2: Missing DONE_MEANS
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

### âŒ Mistake 3: Incomplete BOOTSTRAP
**Wrong:**
```
FILES_TO_OPEN: Some files
```
**Right:**
```
FILES_TO_OPEN:
- .GOV/roles_shared/START_HERE.md
- .GOV/roles_shared/ARCHITECTURE.md
- src/backend/handshake_core/src/api/jobs.rs
- src/backend/handshake_core/src/jobs.rs
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/models.rs
- src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
```

### âŒ Mistake 4: Delegating without verification
**Wrong:**
```
I created the packet. Coder, start coding.
```
**Right:**
```
Running verification:
$ just pre-work WP-1-Job-Cancel

âœ… Pre-work validation PASSED

Task Packet: .GOV/task_packets/WP-1-Job-Cancel.md
[Full handoff message...]
```

---

## Success Criteria

**You succeeded if:**
- âœ… Task packet file exists and is complete
- âœ… `just pre-work WP-{ID}` passes
- âœ… Coder receives clear handoff message
- âœ… **YOU STOPPED TALKING** after the handoff message

**You failed if:**
- âŒ You wrote code in `src/` or `app/`
- âŒ Coder asks "what should I do?"
- âŒ Coder starts coding without packet
- âŒ Work gets rejected at review for missing packet
- âŒ Scope confusion leads to wrong implementation

---

## Quick Reference

**Commands:**
```bash
# Create packet
just create-task-packet WP-{ID}

# Verify readiness
just pre-work WP-{ID}

# Check packet exists
ls .GOV/task_packets/WP-*.md
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
  * Migrations rewrite (â†’ WP-1-Migration-Framework)

## Quality Gate
- **RISK_TIER**: LOW | MEDIUM | HIGH
- **TEST_PLAN**: {Exact bash commands}
- **DONE_MEANS**: {Measurable criteria - 1:1 mapped to SPEC_ANCHOR}
- **ROLLBACK_HINT**: {How to undo}

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**: {5-15 key files}
- **SEARCH_TERMS**: {10-20 grep targets}
- **RUN_COMMANDS**: {Startup + validation commands}
- **RISK_MAP**: {Failure modes â†’ subsystems (3-8 items)}

## Authority
- **SPEC_ANCHOR**: Â§{section} ({requirement})
- **Codex**: {version}
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md
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

**CLARIFICATION: Orchestrator's Role in SPEC_ANCHOR Verification**

Orchestrator DOES verify (checklist below):
- âœ… SPEC_ANCHOR cites a Main Body section (not Roadmap)
- âœ… Cited section exists in SPEC_CURRENT.md
- âœ… Section number is specific (Â§2.3.12.1, not Â§2.3.12 alone)

Orchestrator DOES NOT verify (Validator verifies this):
- âŒ Whether the cited requirement is the RIGHT interpretation
- âŒ Whether this requirement is complete/correct
- âŒ Whether all MUST/SHOULD from that section are covered

**If SPEC_ANCHOR is ambiguous** (could map to multiple sections):
â†’ ESCALATE to user; get explicit decision before proceeding.
Do not guess which section is correct.

**Valid SPEC_ANCHOR examples:**
- `Â§2.3.12.1 (Four Portability Pillars)`
- `Â§2.3.12.3 (Storage API Abstraction Pattern)`
- `Â§A9.2.1 (Error Code Registry)`

**Invalid (REJECT these):**
- `Â§Future Work (Phase 2+)` â€” Not Main Body
- `Â§Roadmap` â€” Not specific enough
- No SPEC_ANCHOR at all â€” Every WP requires one
- `Â§2.3.12` alone â€” Too broad; need specific subsection

**Orchestrator verification checklist:**
- [ ] SPEC_ANCHOR references MAIN BODY section (before Roadmap)
- [ ] SPEC_ANCHOR exists in latest Master Spec version
- [ ] Section number is specific (Â§X.X.X format)
- [ ] If multiple valid sections exist â†’ ESCALATE to user for clarification

**If FAIL:** Reject WP; request Orchestrator cite spec requirement explicitly or escalate.

### 5.3 IN_SCOPE_PATHS Precision [CX-603]

**Orchestrator MUST be specific (NOT vague).**

```
âŒ WRONG: IN_SCOPE_PATHS: src/backend
âŒ WRONG: IN_SCOPE_PATHS: src/
âŒ WRONG: IN_SCOPE_PATHS: Everything related to storage

âœ… RIGHT: IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/api/jobs.rs
```

**Why:** Coder needs to know EXACTLY which files they can modify. Vague scope = scope creep.

### 5.4 DONE_MEANS Mapping [CX-602]

**Every DONE_MEANS MUST map 1:1 to SPEC_ANCHOR requirement.**

Example:
```markdown
SPEC_ANCHOR: Â§2.3.12.3 (Storage API Abstraction Pattern)

Spec says:
- "MUST: Define Database trait with async methods"
- "MUST: Implement SqliteDatabase wrapper"
- "MUST: Create PostgresDatabase stub"

DONE_MEANS (mapped):
- [ ] MUST: Database trait defined (Â§2.3.12.3, requirement 1)
- [ ] MUST: SqliteDatabase implemented (Â§2.3.12.3, requirement 2)
- [ ] MUST: PostgresDatabase stub created (Â§2.3.12.3, requirement 3)
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
   - Validation commands (`just validate`) + manual review requirement

4. **RISK_MAP (3-8 failure modes)**
   - Specific failure mode
   - Which subsystem breaks
   - Example: `"Hollow trait implementation" â†’ Portability Failure (Phase 1 blocker)`

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
- âœ… Once locked, packet is immutable
- âœ… Prevents instruction creep mid-work
- âœ… Creates audit trail (version history)
- âŒ Cannot edit locked packet (violates governance)
- âŒ If changes needed, must create new packet

**When to create variant packets:**
- WP-1-Storage-Abstraction-Layer (original, locked)
- WP-1-Storage-Abstraction-Layer-v2 (changes needed, new packet)
- OR: WP-1-Storage-Abstraction-Layer-v3 (next revision; no date/time stamps)

**Traceability rule (mandatory when variants exist):**
- Treat `WP-1-Storage-Abstraction-Layer` as the **Base WP ID**.
- If you create `...-v{N}`, update `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` so the Base WP maps to the single Active Packet, and mark the older packet(s) as Superseded on `.GOV/roles_shared/TASK_BOARD.md`.
- When instructing Coders/Validators to run `just pre-work` / `just post-work`, always provide the **Active Packet WP_ID** (often includes `-vN`) to avoid ambiguous matches.

### 5.7 Variant Lineage Audit (ALL versions) [CX-580E] (BLOCKING)

When you create a revision packet (`-v{N}`) for a Base WP, you MUST include a **Lineage Audit** inside the new packet before delegation.

**Goal:** Prevent â€œspecâ†’packetâ†’codeâ€ gaps caused by version churn. A `-v{N}` packet is NOT allowed to validate only â€œwhat changed in v{N}â€; it must prove the **entire Base WP requirement** is satisfied in the repo as of SPEC_TARGET.

**MANDATORY:** Add `## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]` to the new packet and include, at minimum:
- `BASE_WP_ID` and the new `WP_ID` being created.
- Roadmap pointer(s) (if applicable) AND the governing Master Spec Main Body anchors for â€œDoneâ€.
- `SPEC_TARGET` resolved at creation time (from `.GOV/roles_shared/SPEC_CURRENT.md`).
- A list of ALL known prior packet files for the Base WP (v1/v2/...) and their statuses (Superseded/FAIL/Historical/etc.).
- A requirement map showing every governing Main Body MUST/SHOULD translated to current repo evidence:
  - `SPEC_ANCHOR` (exact clause ID)
  - Code evidence (`path:line` in the repo)
  - Provenance (introducing commit via `git blame`, or explicit â€œpresent before v{N}â€)
  - If anything is missing: declare GAP and STOP (create a remediation WP or initiate spec enrichment).

**Suggested commands (examples):**
- `cat .GOV/roles_shared/SPEC_CURRENT.md`
- `rg -n "<forbidden symbols>" src/`
- `git blame -n -L <line>,<line> <path>`
- `git log --oneline --decorate -- <path>`

**Template (copy into the packet):**
```markdown
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-...
- WP_ID: WP-1-...-vN
- SPEC_TARGET: Handshake_Master_Spec_vXX.XXX.md (from .GOV/roles_shared/SPEC_CURRENT.md)
- Roadmap pointer: Â§7.6.x (pointer only; Main Body is authority)
- Prior packets:
  - .GOV/task_packets/WP-1-....md (status: ...)
  - .GOV/task_packets/WP-1-....-v2.md (status: ...)

| SPEC_ANCHOR | Main Body requirement (MUST/SHOULD) | Repo evidence (path:line) | Introduced (commit) | Notes |
|---|---|---|---|---|
| A?.?.? | ... | ... | ... | ... |
```

---

## Part 6: Task Board Maintenance [CX-625-630]

### 6.1 Task Board Structure (Single Source of Truth)

**Orchestrator maintains `.GOV/roles_shared/TASK_BOARD.md` as the authoritative status tracker.**

```markdown
# Handshake Project Task Board

This board is a shared state file updated by the active agent (Orchestrator, Coder, Validator).
Updated whenever WP status changes.

---

## ðŸš¨ PHASE 1 CLOSURE GATES (BLOCKING)

**Authority:** Master Spec Â§2.3.12, Architecture Decision {date}

Storage Backend Portability Foundation (Sequential):

1. **[WP-1-Storage-Abstraction-Layer]** - Define trait-based storage API
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 15-20 hours
   - Status: [READY FOR DEV ðŸ”´]
   - Blocker: None (foundational)

2. **[WP-1-AppState-Refactoring]** - Remove SqlitePool from AppState
   - Lead: Coder (Senior Systems Engineer)
   - Effort: 8-10 hours
   - Status: [GAP ðŸŸ¡]
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST COMPLETE FIRST)

---

## In Progress

- **[WP_ID]** - {VALIDATION_STATUS}

## Ready for Dev

- **[WP_ID]** - {VALIDATION_STATUS}

## Done

- **[WP_ID]** - {VALIDATION_STATUS}

## Blocked

- **[WP_ID]** - BLOCKED: {Reason for block}

## Superseded (Archive)

- **[WP_ID]** - SUPERSEDED: {Reason for archival}
```

### 6.2 Status Values (CX-625)

| Status | Symbol | Meaning | When to Use |
|--------|--------|---------|------------|
| **READY FOR DEV** | ðŸ”´ | Verified, waiting for Coder | After pre-work checklist PASS |
| **IN PROGRESS** | ðŸŸ  | Coder is working | After Coder outputs BOOTSTRAP |
| **BLOCKED** | ðŸŸ¡ | Waiting for dependency/clarification | Document specific reason |
| **DONE** | âœ… | Merged to main | After Validator approves |
| **GAP** | ðŸŸ¡ | Not yet created as packet | Before Orchestrator creates |

### 6.3 Orchestrator Responsibilities for TASK_BOARD

**Ensure TASK_BOARD is updated IMMEDIATELY when:**
1. New WP created â†’ Move to "Ready for Dev"
2. Coder starts work â†’ Ensure the Coder has produced a docs-only bootstrap claim commit; Validator status-syncs `main` (updates `## In Progress`; optionally also `## Active (Cross-Branch Status)`).
3. Blocker discovered â†’ Move to "Blocked" + document reason
4. Validator approves â†’ Validator moves to "Done" (Orchestrator verifies TASK_BOARD reflects reality)
5. Dependency unblocked â†’ Move blocked WP to "Ready for Dev"

**Keep TASK_BOARD in sync with reality:**
```
Never let TASK_BOARD drift from actual WP status.
If the Operator-visible Task Board on `main` does not reflect packet reality, the Validator must run a docs-only status-sync commit to correct it.
```

### 6.4 Phase Gate Status Tracking [CX-609]

**Orchestrator must maintain Phase Gate section:**

```markdown
## ðŸš¨ PHASE 1 CLOSURE GATES (BLOCKING - MUST COMPLETE)

**Status:** HOLDING - 3 of 4 gate-critical WPs not yet created

Gate-critical WPs:
1. âœ… WP-1-Storage-Abstraction-Layer [READY FOR DEV]
2. âŒ WP-1-AppState-Refactoring [GAP - packet not yet created]
3. âŒ WP-1-Migration-Framework [GAP - packet not yet created]
4. âŒ WP-1-Dual-Backend-Tests [GAP - packet not yet created]

Phase closure criteria:
- [ ] All 4 gate-critical WPs are VALIDATED (not just "done")
- [ ] Spec regression check PASS (just validator-spec-regression)
- [ ] All dependencies resolved
- [ ] Waivers audit complete
- [ ] Supply chain clean (cargo deny + npm audit)

Current status: 25% ready (1 of 4 packets created, 0 VALIDATED)
```

### 6.5 Phase Closure Gate (Explicit Requirements) [CX-609B]

**A phase is ready to close ONLY when ALL criteria below are met.**

#### MUST Criteria (All Required)

- [ ] **All phase-critical WPs are VALIDATED** (Validator approved, not just "done")
  - Meaning: Validator returned `verdict: PASS` for each WP
  - Not: "Coder finished coding" or "work merged"

- [ ] **Spec regression check passes**
  ```bash
  just validator-spec-regression
  # Output: âœ… Spec regression check PASSED
  ```

- [ ] **Supply chain audit clean** (zero violations)
  ```bash
  cargo deny check    # Should return 0 violations
  npm audit           # Should return 0 critical/high vulnerabilities
  ```

- [ ] **No unresolved blockers** (all dependencies satisfied)
  - TASK_BOARD shows NO items in "Blocked" state
  - All WPs have clear VALIDATED status for their dependencies

- [ ] **Git commit audit trail complete** (all commits signed/traced)
  - All work-related commits must have proper git metadata (author, timestamp)
  - Optional: If using git signatures, all commits must be signed

#### SHOULD Criteria (Strong Recommendations)

- [ ] **No open escalations from Validator** (all escalations resolved)
- [ ] **No "deferred work" notes in WPs** (all planned work in this phase is done)
- [ ] **Test coverage metrics on target** (>80% for phase)
- [ ] **Security audit clean** (if phase touches security-sensitive code)

#### Example: Phase 1 Closure Gate

```
Phase 1 Closure Gate Status:

MUST Criteria:
âœ… WP-1-Storage-Abstraction-Layer: VALIDATED (PASS)
âœ… WP-1-AppState-Refactoring: VALIDATED (PASS)
âœ… WP-1-Migration-Framework: VALIDATED (PASS)
âœ… WP-1-Dual-Backend-Tests: VALIDATED (PASS)
âœ… Spec regression: PASS
âœ… Cargo deny: 0 violations
âœ… npm audit: 0 high vulnerabilities
âœ… No blockers in TASK_BOARD
âœ… All commits properly tracked

SHOULD Criteria:
âœ… No escalations pending
âœ… No deferred work notes
âœ… Test coverage: 84% (>80% target met)
âœ… Security audit clean (Phase 1 touches storage layer)

â†’ Phase 1 READY TO CLOSE âœ…
```

#### How to Use This Gate

**Before closing phase:**
1. âœ… Check TASK_BOARD: All critical WPs show VALIDATED?
2. âœ… Run spec regression check
3. âœ… Run supply chain audits
4. âœ… Review escalations log (empty?)
5. âœ… Review WPs for deferred work notes
6. âœ… Confirm all dependencies resolved

**If ANY MUST criterion fails:**
â†’ Phase is NOT ready. Document blocker + ETA.

**If ALL MUST criteria pass:**
â†’ Phase ready to close (SHOULD criteria are recommendations, not blockers).

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
- âœ… VALIDATED â†’ Can assign WP-1-AppState-Refactoring
- ðŸŸ  IN PROGRESS â†’ Mark WP-1-AppState-Refactoring as BLOCKED
- ðŸ”´ READY FOR DEV â†’ Mark WP-1-AppState-Refactoring as BLOCKED
- âŒ FAILS Validator â†’ Don't assign, escalate

Rule: Never assign downstream work until blocker is VALIDATED.
```

**DO NOT close phase if blockers unresolved:**

```
Phase 1 closure requires:
- ALL 4 gate-critical WPs VALIDATED
- ALL dependencies satisfied
- NO unresolved blockers

If WP-1-Migration-Framework blocks WP-1-Dual-Backend-Tests:
â†’ Phase cannot close until BOTH are VALIDATED
```

**Document WHY WP is BLOCKED:**

```markdown
## Blocked

- WP-1-AppState-Refactoring: Waiting for WP-1-Storage-Abstraction-Layer to VALIDATE (ETA 3 days)
- WP-1-Dual-Backend-Tests: Blocked on 2 dependencies (WP-1-Storage-Abstraction-Layer, WP-1-Migration-Framework)
```

### 7.3 SLA for Work States [CX-635B]

**Orchestrator MUST enforce time-based SLAs to prevent work from stalling.**

| Status | Max Duration | Action if Exceeded | Escalation |
|--------|--------------|-------------------|------------|
| **BLOCKED** | 5 work days | Escalate blocker | Notify user: "WP-X has been blocked for 6 days. What's the plan?" |
| **READY FOR DEV** | 10 work days | Flag as risk | Check: Is Coder assigned? Is there a hidden blocker? |
| **IN PROGRESS** | 30 work days | Assess estimate | Was original estimate wrong? Do we need to split the work? |

#### BLOCKED Status (Max 5 work days)

**Scenario:** WP-1-AppState-Refactoring depends on WP-1-Storage-Abstraction-Layer

**Day 0-4:** Document blocker, leave in BLOCKED state

**Day 5:** If blocker still unresolved:
```
âš ï¸ ESCALATION: WP-X blocked beyond SLA [CX-635-B1]

WP-ID: WP-1-AppState-Refactoring
Status: BLOCKED (5 days, SLA exceeded)
Blocker: WP-1-Storage-Abstraction-Layer (status: {current status})

This WP cannot proceed until blocker resolves.

Action required:
1. What is the updated ETA for blocker resolution?
2. Should we split this work differently?
3. Is there alternative work to do while we wait?

Awaiting response by: {date/time}
```

#### READY FOR DEV Status (Max 10 work days)

**Scenario:** Packet created and verified, waiting for Coder to start

**Day 0-9:** WP sits in "Ready for Dev", waiting for Coder assignment

**Day 10:** If Coder hasn't started:
```
ðŸš¨ RISK FLAG: WP-X idle beyond SLA [CX-635-B2]

WP-ID: WP-1-Job-Cancel-Endpoint
Status: READY FOR DEV (10 days, no progress)
Created: {date}, assigned: {date}

Risk assessment:
- Is Coder aware of this task?
- Is there a blocker we missed?
- Should Coder prioritize this over other work?

Action: Confirm priority and Coder assignment
```

#### IN PROGRESS Status (Max 30 work days)

**Scenario:** Coder is actively working

**Day 0-29:** Coder makes progress, updates task packet with partial results

**Day 30:** If still IN PROGRESS with no completion in sight:
```
ðŸ“‹ ESTIMATE REVIEW: WP-X progress check [CX-635-B3]

WP-ID: WP-1-Storage-Abstraction-Layer
Status: IN PROGRESS (30 days, original estimate: 15-20 hours)

Actual progress: {what's done, what's remaining}
Original estimate: 15-20 hours (estimated 3-5 work days)
Actual effort: 30+ days

Analysis:
- Was original estimate too low?
- Did scope creep occur?
- Are there unexpected blockers?
- Should we split work into smaller packets?

Action: Reassess estimate or break work into phases
```

#### Escalation Template (Universal)

Use this template for ANY SLA-triggered escalation:

```
âš ï¸ SLA ESCALATION: {WP-ID} [CX-635]

**Work Packet:** {WP-ID} ({brief description})
**Status:** {BLOCKED|READY FOR DEV|IN PROGRESS}
**Duration:** {X days} (SLA limit: {Y days})
**Created:** {date}, Last update: {date}

**Current State:**
{Description of why we're escalating}

**Blocker/Issue:**
{Specific thing preventing progress}

**Action Needed:**
{What must happen to unblock}

**Response Required By:** {date/time}
**Escalation Channel:** {user|team lead|project manager}
```

---

## Part 8: Pre-Delegation Validation Checklist [CX-640]

**Before handing off to Coder, Orchestrator MUST verify all items below (and any applicable conditional items):**

- [ ] SPEC_ANCHOR references Main Body (not Roadmap)
- [ ] SPEC_ANCHOR in latest Master Spec version
- [ ] IN_SCOPE_PATHS are exact file paths (not "src/backend")
- [ ] OUT_OF_SCOPE clearly lists what Coder cannot touch
- [ ] DONE_MEANS are measurable (100% verifiable, not subjective)
- [ ] Every DONE_MEANS maps 1:1 to SPEC_ANCHOR requirement
- [ ] RISK_TIER assigned (LOW/MEDIUM/HIGH)
- [ ] TEST_PLAN includes all applicable commands
- [ ] TEST_PLAN lists `just cargo-clean` (external `../Cargo Target/handshake-cargo-target`) before post-work/self-eval
- [ ] BOOTSTRAP has 5-15 FILES_TO_OPEN
- [ ] BOOTSTRAP has 10-20 SEARCH_TERMS
- [ ] BOOTSTRAP has RISK_MAP (3-8 failure modes)
- [ ] USER_SIGNATURE locked with date/timestamp
- [ ] Dependencies documented (blockers + what this blocks)
- [ ] Effort estimate provided (hours)

**Conditional (BLOCKING when applicable):**
If the WP includes cross-boundary changes (e.g., UI/API/storage/events) OR any governing spec/DONE_MEANS includes MUST record/audit/provenance:

- [ ] End-to-end closure plan captured (producer output â†’ API schema â†’ server-side verification/source-of-truth â†’ audit event/log)
- [ ] Trust boundary decision recorded (client-provided audit/provenance is UNTRUSTED unless explicitly waived; server derives/verifies)
- [ ] No unused plumbing (any newly introduced request/response fields are used end-to-end or removed before delegation)
- [ ] Error taxonomy planned (stale input/hash mismatch vs invalid input vs scope violation vs provenance mismatch/spoof attempt)
- [ ] Deterministic `just post-work` plan is explicit for multi-commit WPs (record MERGE_BASE_SHA; include `--range MERGE_BASE_SHA..HEAD` in TEST_PLAN when needed)
- [ ] UI/action guardrails included when applicable (re-check prerequisites at click-time; block stale apply rather than generating misleading diagnostics)

**If ANY check fails:** Reject WP; request Orchestrator fix specific gaps.

---

## Part 9: Orchestrator Non-Negotiables [CX-640-650]

### âŒ DO NOT:

1. **Create WP without SPEC_ANCHOR** â€” Every WP must reference Master Spec Main Body
2. **Edit locked work packets** â€” Once USER_SIGNATURE added, packet is immutable
3. **Use vague scope** â€” IN_SCOPE_PATHS must be specific file paths
4. **Assign WP with unresolved blocker** â€” Wait for blocker to VALIDATE first
5. **Close phase without all WPs VALIDATED** â€” "Done" â‰  "VALIDATED"
6. **Skip pre-orchestration checklist** â€” All 14 items must pass
7. **Invent requirements** â€” Task packets point to SPEC_ANCHOR, period
8. **Let TASK_BOARD drift** â€” Ensure TASK_BOARD on `main` is status-synced when WP status changes (Validator: In Progress/Done; Orchestrator: planning states)
9. **Lump multiple features in one WP** â€” One WP per requirement
10. **Leave dependencies undocumented** â€” TASK_BOARD must show all blocking relationships

### âœ… DO:

1. **Create one WP per Master Spec requirement** â€” No lumping
2. **Lock every packet with USER_SIGNATURE** â€” Prevents instruction creep
3. **Map every DONE_MEANS to SPEC_ANCHOR** â€” Traceability required
4. **Document dependencies explicitly** â€” TASK_BOARD shows blockers
5. **Maintain Phase Gate visibility** â€” Keep status current
6. **Run pre-orchestration checklist** â€” Verify spec, board, supply chain
7. **Keep TASK_BOARD on `main` in sync** â€” Validator status-syncs In Progress/Done; Orchestrator maintains planning states
8. **Provide complete BOOTSTRAP** â€” Coder needs 5-15 files, 10-20 terms, risk map
9. **Create variant packets for changes** â€” Never edit locked packets
10. **Enforce blocking rules** â€” Don't assign downstream work prematurely

---

## Part 10: Real Examples (Templates)

See actual work packets in `.GOV/task_packets/` for patterns:

- **WP-1-Storage-Abstraction-Layer.md** â€” High risk, foundational (trait-based design)
- **WP-1-AI-Integration-Baseline.md** â€” Medium risk, feature (LLM integration)
- **WP-1-Terminal-Integration-Baseline.md** â€” High risk, security-sensitive

All follow the structure in this protocol; use them as templates for new WPs.

---

**ORCHESTRATOR SUMMARY:**

| Responsibility | Primary Document | Authority |
|---|---|---|
| Create work packets | `.GOV/task_packets/WP-*.md` | ORCHESTRATOR_PROTOCOL Part 4-5 |
| Maintain task board | `.GOV/roles_shared/TASK_BOARD.md` | ORCHESTRATOR_PROTOCOL Part 6 |
| Track dependencies | Packet + TASK_BOARD | ORCHESTRATOR_PROTOCOL Part 7 |
| Validate before delegation | Pre-work checklist | ORCHESTRATOR_PROTOCOL Part 8 |
| Lock packets | USER_SIGNATURE | ORCHESTRATOR_PROTOCOL Part 5.6 |
| Update status immediately | TASK_BOARD sync | ORCHESTRATOR_PROTOCOL Part 6.3 |
| Enforce phase gates | PHASE 1 CLOSURE GATES | ORCHESTRATOR_PROTOCOL Part 6.4 |
| Manage blockers | Dependency tracking | ORCHESTRATOR_PROTOCOL Part 7 |

**Orchestrator role = Precise work packets + Updated TASK_BOARD + Locked packets + Verified pre-work + Enforced dependencies + Phase gate management**
