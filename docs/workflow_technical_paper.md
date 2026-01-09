# Spec-Driven Multi-Agent Workflow (Complete Implementation Guide)

## Scope and Inputs

This document is **complete, standalone, and implementable by a fresh model** for any large software project using multi-agent AI workflows (Orchestrator, Coder, Validator, Tool Agent).

**What this covers**:
- Core concepts and 5 guiding principles
- Detailed roles and responsibilities
- 6-stage end-to-end flow
- Complete file structure and templates
- Real working examples (filled-in packets, validator output, etc.)
- Governance rules template (Codex)
- Protocol file templates
- Validator script implementations (samples + patterns)
- Phased rollout and implementation order
- Integration with existing projects
- Troubleshooting and common failures
- Testing strategy for the workflow itself

**Files you will create** (customize names as needed):
- `docs/SPEC_CURRENT.md` - Pointer to active spec
- `Master_Spec_v1.0.md` - The authoritative specification
- `{Project} Codex v1.0.md` - Governance rules
- `docs/ORCHESTRATOR_PROTOCOL.md` - Orchestrator workflow
- `docs/CODER_PROTOCOL.md` - Coder workflow
- `docs/VALIDATOR_PROTOCOL.md` - Validator workflow
- `docs/TASK_BOARD.md` - Master task tracking
- `docs/SIGNATURE_AUDIT.md` - Signature registry
- `docs/task_packets/TEMPLATE.md` - Packet template
- `docs/task_packets/WP-*.md` - Actual work packets
- `scripts/validation/*.mjs` - Validator scripts (10-15 files)
- `scripts/hooks/pre-commit` - Git hook
- `justfile` - Command wiring

## Core Principles

**1. Spec as Single Source of Truth**
- `docs/SPEC_CURRENT.md` is a pointer file naming the active Master Spec (e.g., `Master_Spec_v1.0.md`)
- Master Spec uses hierarchical SPEC_ANCHOR format (e.g., `A2.3.12.3`)
- Only Main Body sections are binding authority; Roadmap is optional reference
- Every work packet must cite a precise SPEC_ANCHOR that exists in the current spec; ambiguous anchors must be escalated, not guessed
- Spec change workflow: enrich a new version + update SPEC_CURRENT.md + run spec regression validator + obtain signature before use

**2. Signatures Create Forced Alignment**
- User-issued signature `{username}{DDMMYYYYHHMM}` (e.g., `alice_25121512345`) is required before spec enrichment OR packet creation
- Signature is a **collaborative pause**: Orchestrator proposes, user validates, signature locks the agreement (DONE_MEANS, TEST_PLAN, scope, risk tier)
- Signatures are **one-time-use only**, recorded in `docs/SIGNATURE_AUDIT.md`, verified via grep: `grep -r "{signature}" . | grep -c signature_audit.md` must equal 1
- Prevents autonomous drift; every significant decision has a human decision point

**3. Work is Packetized (One Requirement per Packet)**
- One logical requirement = one task packet (WP-{phase}-{name}.md)
- Packet contains 10 required fields: TASK_ID, STATUS, SPEC_ANCHOR, scope, DONE_MEANS, TEST_PLAN, BOOTSTRAP, ROLLBACK_HINT, VALIDATION, Signature Log
- Packets are **immutable after signature** (USER_SIGNATURE field freezes content); changes require `-v2` variant with fresh signature
- Prevents scope creep; enforces clear work boundaries

**4. State Consistency (Packet ↔ Task Board)**
- Task Board and packet STATUS must always match (Ready-for-Dev, In-Progress, Done, etc.)
- State changes are **atomic**: update both packet STATUS and task board entry together
- Prevents state drift; task board is always authoritative

**5. Evidence or It Does Not Exist**
- Every DONE_MEANS requirement must have:
  - **Code evidence**: specific file:line where the requirement is implemented
  - **Test proof**: command that fails if code is removed (removability check)
  - **Spec anchor**: which section governs this requirement
- Validator audits evidence deterministically; "looks good" is not evidence
- Traceability enables post-mortem analysis, regression detection, feature removal safety

## Roles & Responsibilities

### Orchestrator (Lead Architect / Engineering Manager)
- **Owns translation** of user intent and spec requirements into immutable task packets; does not write code
- **Pre-orchestration gates**: verify spec currency and regression, task board freshness, supply chain health (`cargo deny`, `npm audit`), governance file currency, dependency chains
- **Signature pause protocol**: before packet creation, propose DONE_MEANS (5-10 measurable checkpoints), TEST_PLAN commands, IN/OUT scope (exact file paths), RISK_TIER with justification, validator audit scope, rollback plan, packet variant triggers
- **Packet creation** (via `just create-task-packet WP-{id}`): fill all 10 required fields with zero placeholders; enforce SPEC_ANCHOR references; scope to exact files (not globs); one requirement per packet
- **Verification before delegation**: run `just pre-work WP-{id}` (blocks on failure); ensure `docs/TASK_BOARD.md` reflects Ready for Dev; lock packets with USER_SIGNATURE
- **Handoff**: provide packet path, WP_ID, RISK_TIER, authority docs (SPEC_CURRENT, protocols), and readiness confirmation; maintain SLAs (no downstream work until blockers are VALIDATED)

### Coder / Debugger
- **Refuses to start** without a complete task packet; performs scope adequacy check (can I identify all affected files? Are boundaries clear? Are there unexpected dependencies?)
- **Outputs BOOTSTRAP** (FILES_TO_OPEN 5-15, SEARCH_TERMS 10-20, RUN_COMMANDS 3-6, RISK_MAP 3-8) before first code change; sets task packet Status: In Progress and creates a docs-only bootstrap claim commit (Validator status-syncs `main`)
- **Implements strictly within IN_SCOPE_PATHS**, honoring DONE_MEANS and OUT_OF_SCOPE; enforces hard invariants (zero speculative requirements, no TODO placeholders without tracking IDs)
- **Updates packet** with evidence/status handoff and prepares commit message referencing WP_ID; Validator maintains the Operator-visible Task Board on `main`

### Validator (Senior Engineer / Lead Auditor)
- **Blocks merges** unless evidence proves alignment with spec, codex, and packet; preserves collaboration context in packets
- **Pre-flight checks**: confirm packet completeness (all 10 fields), spec version match, USER_SIGNATURE unchanged and valid, STATUS present, BOOTSTRAP present, TEST_PLAN concrete
- **Spec extraction**: enumerate every MUST/SHOULD from DONE_MEANS + spec anchors; map each to file:line evidence; missing evidence = FAIL
- **Hygiene gates**: reject hollow code, JSON blobs where types required, unwrap/expect/panic in production without waiver; enforce Zero Placeholder Policy; run `just validator-scan`
- **Targeted audits**: storage DAL (trait boundary, SQL portability, migration numbering, dual-backend readiness), LLM boundary, determinism/traceability (trace_id/job_id in mutations), security/RCE guardrails, git/build hygiene
- **Binary verdict**: PASS (evidence complete, audits clean, tests pass) appends validation report to packet and moves to Done; FAIL (gaps found) documents violations and returns to Orchestrator/Coder

### Tool Agent (Mechanical Execution via Typed Workflows)
- **Executes allowed commands** (tests, scans, formatters, linters, file searches) as **typed workflow invocations** (not raw shell)
- **Works within bounds**: respects IN_SCOPE_PATHS, honors OUT_OF_SCOPE, respects RISK_TIER constraints (no exploratory changes)
- **Returns artifact handles**: tools output references (handles), not embedded content; Coder/Validator fetch on-demand
- **Enables token efficiency**: prompt references handles instead of embedding 50KB of logs; 95%+ token savings on tool-heavy tasks
- **Reuses workflows**: tool invocations use subworkflow templates (test_suite, build_pack, validator_scan_pack) across all packets; zero duplication
- **Reduces context churn**: keeps Coder/Validator prompts tiny by using artifact references instead of raw output

## Cross-Domain Composition Macros (Reusable Workflow Patterns)

Instead of designing workflows for each domain, use these **4 universal composition macros** that apply to any large project:

### Macro 1: Ingest → Normalize → Index → Enrich → Present

**Applies to**: Data ingestion, format conversion, multi-source consolidation

**Packet pattern**:
- **DONE_MEANS**:
  1. Raw data ingested into standard entity format
  2. Normalized to canonical schema (no domain-specific quirks)
  3. Indexed for search/retrieval
  4. Enriched with computed properties (summaries, classifications)
  5. Presented in multiple views (API, UI, export)

**Tool Agent workflow template**: `ingest_pack`
- Inputs: source type, entity format, normalization rules
- Outputs: entity handle, index handle, enrichment handle
- Error workflow: emit diagnostic (format mismatch, schema violation, index failure)

**Example packets**: WP-1-CSV-Ingestion, WP-2-Email-Thread-Ingestion, WP-3-Log-Aggregation

---

### Macro 2: Extract → Compute → Visualize → Export

**Applies to**: Data analytics, reporting, metric generation

**Packet pattern**:
- **DONE_MEANS**:
  1. Data extracted from source (subset query, filter)
  2. Computed metrics/aggregations applied
  3. Visualized in at least one format (table, chart, graph)
  4. Exported in canonical format (JSON, CSV, PDF)

**Tool Agent workflow template**: `analytics_pack`
- Inputs: extraction filter, computation rules, visualization type
- Outputs: extraction handle, computation handle, visualization handle, export handle
- Error workflow: emit diagnostic (query failed, computation timeout, export format error)

**Example packets**: WP-4-Revenue-Dashboard, WP-5-Performance-Report, WP-6-Audit-Export

---

### Macro 3: Detect → Suggest → Gate → Apply

**Applies to**: Decision workflows, approval chains, safe mutations

**Packet pattern**:
- **DONE_MEANS**:
  1. Condition/anomaly detected via rule, threshold, or ML
  2. Suggestion generated (action, parameter, reason)
  3. Gated by validator (human or automated check)
  4. Applied only after gate passes

**Tool Agent workflow template**: `decision_pack`
- Inputs: detection rules, suggestion algorithm, gate criteria
- Outputs: detection handle, suggestion handle, gate verdict, execution handle
- Error workflow: emit diagnostic (detection failed, suggestion generation failed, gate rejected)

**Example packets**: WP-7-Calendar-Conflict-Detection, WP-8-Cost-Anomaly-Alert, WP-9-Approval-Chain

---

### Macro 4: Observe → Diagnose → Bundle → Repair

**Applies to**: Debugging, incident response, post-mortem analysis

**Packet pattern**:
- **DONE_MEANS**:
  1. Symptoms observed (logs, metrics, errors)
  2. Root cause diagnosed (dependency traces, state inspection)
  3. Debug bundle created (artifacts, snapshots, reproduction steps)
  4. Repair steps proposed (rollback, patch, workaround)

**Tool Agent workflow template**: `diagnostic_pack`
- Inputs: symptom filters, diagnostic rules, repair knowledge base
- Outputs: observation handle, diagnosis handle, bundle handle, repair plan handle
- Error workflow: emit diagnostic (trace collection failed, diagnosis inconclusive, bundle export failed)

**Example packets**: WP-10-Failed-Job-Debugging, WP-11-Memory-Leak-Investigation, WP-12-Schema-Corruption-Recovery

---

### Why These Macros Scale

1. **Reusability**: Every new feature maps to one of 4 patterns; no new workflow design needed
2. **Consistency**: All packets using same macro have same structure, same validation gates, same evidence model
3. **Tool reuse**: `ingest_pack` template used for CSV, email, logs, URLs, etc. (parameterized by input type)
4. **Error handling**: Each macro has standard error workflows (no custom failure handling per domain)
5. **Token efficiency**: Tool Agent learns 4 workflow templates; invocation is parameterized

---

## End-to-End Flow (6 Stages)

**Stage 1: Intake & Coverage Check**
- User prompt arrives; Orchestrator extracts explicit requirements and implied constraints (treat prompt as proto-spec)
- Run "clearly covers" 5-point test against spec Main Body (does it cover scope, risks, success criteria, dependencies, rollback?)
- If any test fails, request clarification and enrich spec with new version + signature before proceeding

**Stage 2: Spec Anchoring & Signature Pause**
- Orchestrator maps each requirement to a SPEC_ANCHOR; if no anchor exists, enrich spec (new version, update SPEC_CURRENT.md, run `just validator-spec-regression`)
- **Signature pause (user-orchestrator alignment)**: Orchestrator proposes:
  - 5-10 DONE_MEANS (measurable yes/no checkpoints, not aspirational)
  - TEST_PLAN commands (executable proof for each DONE_MEANS)
  - IN_SCOPE_PATHS (exact files, 5-20 specific paths)
  - OUT_OF_SCOPE (explicit deferrals with reasons)
  - RISK_TIER (LOW/MEDIUM/HIGH with justification)
  - Validator audit scope (which audits required?)
  - Rollback plan (if implementation fails, how to recover?)
  - Packet variant triggers (scope creep handling)
- User validates: interpretation accuracy? TEST_PLAN feasible? Scope realistic? Risk acceptable? Cost/benefit aligned?
- User provides signature `{username}{DDMMYYYYHHMM}`; Orchestrator records in packet's Signature & Enrichment Log

**Stage 3: Packetization & Pre-Flight**
- Orchestrator creates task packet (via `just create-task-packet WP-{id}`) with all 10 fields (no placeholders)
- Runs `just pre-work WP-{id}` (gates on completeness)
- Updates `docs/TASK_BOARD.md` to Ready for Dev (atomic update: both packet and board)
- Issues handoff to Coder: packet path, WP_ID, RISK_TIER, authority docs, readiness confirmation

**Stage 4: Coder Implementation Phase**
- Coder reads task packet, performs scope adequacy check (identifies all affected files, validates boundaries, checks for hidden dependencies)
- Outputs BOOTSTRAP block before first change (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)
- Implements strictly within IN_SCOPE_PATHS, honoring DONE_MEANS and OUT_OF_SCOPE (no speculative requirements)
- Executes TEST_PLAN commands (copy-paste-ready bash commands from packet)
- Records validation evidence in packet's VALIDATION block (Command, Result, Notes)
- Runs `just post-work WP-{id}` (gates on completion)
- Updates packet STATUS to Ready-for-Validation + updates Task Board atomically
- Prepares commit message referencing WP_ID

**Stage 5: Validator Audit Phase**
- Validator reads packet, performs pre-flight: completeness? Spec match? USER_SIGNATURE valid? BOOTSTRAP present?
- Extracts every MUST/SHOULD from DONE_MEANS + spec anchor sections
- Maps each requirement to file:line code evidence
- Runs test commands to verify evidence (removability check: does test fail if code deleted?)
- Runs `just validator-scan` (forbidden patterns), custom audits (DAL, error codes, traceability, security, etc.)
- Records findings in new VALIDATION REPORT block (Verdict: PASS/FAIL, Scope, Findings, Tests, Risks/Gaps, Packet Updates)
- PASS: appends report to packet, updates packet STATUS to Done, updates Task Board to Done
- FAIL: documents gaps and returns to Orchestrator/Coder for rework

**Stage 6: Orchestrator Finalization**
- Publishes mechanical output to user: work summary + validation status + what changed
- Files validation report inside the original packet (immutable record)
- Updates Task Board entry to Done (with VALIDATED marker if applicable)
- Closes work item; issues SLA tracking for next phase

---

## Task Packet Lifecycle & State Flow

**State Transitions**:
```
Backlog → Ready-for-Dev → In-Progress → Ready-for-Validation → Done (VALIDATED)
   ↓          ↓               ↓                  ↓                  ↓
(waiting)  (queued)      (active)         (awaiting audit)      (closed)
```

**Status Rules**:
- **Backlog**: packet created but not spec-ready; waiting for enrichment or dependency resolution
- **Ready-for-Dev**: fully spec'd, signature obtained, `just pre-work` passed; awaiting coder assignment
- **In-Progress**: coder actively implementing; packet shows BOOTSTRAP + validation evidence
- **Ready-for-Validation**: implementation complete; `just post-work` passed; awaiting validator review
- **Done (VALIDATED)**: validator issued PASS; validation report appended to packet; task board entry shows VALIDATED

**Immutability & Variants**:
- Once USER_SIGNATURE is recorded, packet content is frozen
- Any change after signature requires creating a `-v2` variant with fresh signature and new collaboration notes
- Old packet versions remain in git history for audit trail

**SLAs & Escalation**:
- **Backlog >5 days**: escalate; dependency or prioritization issue
- **Ready-for-Dev >10 days**: escalate; insufficient coder capacity
- **In-Progress >30 days**: escalate; scope creep or complexity underestimated
- **Ready-for-Validation >3 days**: escalate; validator queue blocked

**Dependency Enforcement**:
- Task Board explicitly lists blockers: "WP-2-Feature-B blocked on WP-1-Foundation"
- Downstream packets remain BLOCKED until upstream STATUS is VALIDATED
- `just validator-phase-gate {PHASE}` prevents phase closure if blockers unresolved

---

## Dependency Graph Resolution Algorithm

**When should a new packet be created vs. added to existing packet?** Use this algorithm:

### Decision Tree

```
User prompt arrives with new requirement.

1. DOES IT MODIFY CODE IN EXISTING PACKET?
   ├─ YES → Is the packet still In-Progress (not yet VALIDATED)?
   │        ├─ YES → Add to existing packet as variant (WP-{id}-v2, new signature)
   │        └─ NO → Create new packet (WP-{id+1})
   └─ NO → Proceed to step 2

2. IS IT A DEPENDENCY OF OTHER WORK?
   ├─ YES → Create new packet; other packets reference it as blocker
   │        Packet status starts as "Ready-for-Dev"
   └─ NO → Proceed to step 3

3. DOES IT REQUIRE SIGNIFICANT NEW FILES/MODULES?
   ├─ YES → Create new packet (separates concerns)
   └─ NO → Could be part of existing packet (consolidate if same phase)

4. IS IT PART OF PHASE N CLOSURE GATES?
   ├─ YES → Create new packet; list in TASK_BOARD Phase N section
   └─ NO → Create new packet or consolidate (estimate effort)
```

### Dependency Graph Representation

In `docs/TASK_BOARD.md`, explicitly list dependency edges:

```markdown
## Phase 1 Closure Gates (BLOCKING)

### Storage Backend Portability (Sequential)
1. WP-1-Storage-Abstraction-Layer
   - Blocker: None (foundational)
   - Blocked: WP-1-AppState-Refactoring, WP-1-Migration-Framework

2. WP-1-AppState-Refactoring
   - Blocker: WP-1-Storage-Abstraction-Layer (MUST complete first)
   - Blocked: WP-1-Dual-Backend-Tests

3. WP-1-Migration-Framework
   - Blocker: None (can start independently)
   - Blocked: WP-1-Dual-Backend-Tests

4. WP-1-Dual-Backend-Tests
   - Blocker: WP-1-Storage-Abstraction-Layer + WP-1-Migration-Framework
   - Blocked: None (closure gate)
```

### Circular Dependency Detection

**Script** `scripts/validation/validator-dependency-graph.mjs`:
```javascript
// Detect cycles in packet dependencies
const packets = readTaskBoard();
const graph = buildDependencyGraph(packets);
const cycles = detectCycles(graph);

if (cycles.length > 0) {
  console.error("❌ Circular dependency detected:");
  cycles.forEach(cycle => {
    console.error(`  ${cycle.join(" → ")} → ${cycle[0]}`);
  });
  process.exit(1);
}
```

### Dependency Conflict Resolution

| Conflict | Resolution |
|----------|-----------|
| Two independent packets modify same file | Split changes into separate packets; one depends on other for file initialization |
| Packet A needs output from Packet B, but B isn't started | Mark A as BLOCKED until B is VALIDATED |
| Three packets all depend on foundational WP-0 | WP-0 is Phase 0 closure gate; nothing proceeds until WP-0 is VALIDATED |
| User adds new requirement during WP-5-A implementation | Check if it modifies WP-5-A scope: if yes → WP-5-A-v2 (new signature); if no → new WP-5-B |

---

## Concurrent Packet Execution Rules

**Multiple Coders working in parallel on different packets.** How to prevent conflicts?

### Non-Conflicting (Parallel OK)
- ✅ WP-1-Storage-Abstraction-Layer (modifies src/storage/) + WP-2-UI-Frontend (modifies app/src/) → No conflict
- ✅ WP-3-Auth (modifies src/api/auth.rs) + WP-4-Logging (modifies src/logging.rs) → No conflict
- ✅ Same file but different functions: WP-5-Feature-A adds `fn feature_a()` + WP-6-Feature-B adds `fn feature_b()` in same file → Git merge conflict-free if functions don't interact

### Conflicting (Serialize or Risk)
- ❌ WP-7-Database-Schema-v1 (adds columns) + WP-8-Database-Schema-v2 (modifies same columns) → Serialize: WP-8 blocked until WP-7 VALIDATED
- ❌ WP-9-API-Contract (changes endpoint signature) + WP-10-Client (calls that endpoint) → Serialize: WP-10 blocked until WP-9 VALIDATED
- ❌ Both WP-11 and WP-12 modify configuration structure → Coordinate via Task Board; one becomes dependency

### Conflict Detection Rule

**Orchestrator runs before allowing parallel packets**:
```bash
# Extract IN_SCOPE_PATHS from each packet
paths_wp1=$(grep -A 10 "^IN_SCOPE_PATHS:" docs/task_packets/WP-{id1}.md | grep "^  -" | awk '{print $2}')
paths_wp2=$(grep -A 10 "^IN_SCOPE_PATHS:" docs/task_packets/WP-{id2}.md | grep "^  -" | awk '{print $2}')

# Check for overlaps
overlap=$(comm -12 <(echo "$paths_wp1" | sort) <(echo "$paths_wp2" | sort))

if [ -n "$overlap" ]; then
  echo "⚠️  Conflict: Both packets modify:"
  echo "$overlap"
  echo "Create dependency or split packets"
  exit 1
fi
```

### Merge Strategy

When parallel packets complete:

1. **Test each independently**: WP-1 tests PASS, WP-2 tests PASS
2. **Test combined**: Run all tests together (integration check)
3. **Merge sequentially**: WP-1 → main, then WP-2 → main (or both → feature branch → main if high confidence)
4. **Re-validate if any conflicts**: Validator re-runs critical audits if merge required fixups

### SLA for Parallel Work

- Limit: Max 3 concurrent packets touching overlapping modules
- Retry: If merge fails, revert one packet; coordinate rebase
- Gate: Both packets must be VALIDATED before merging concurrently

---

## Signature Pause (Alignment & Enrichment)
- **Orchestrator proposes**: SPEC_ANCHORs or enrichment needed, 5-10 DONE_MEANS (measurable), TEST_PLAN commands, IN/OUT scope, RISK_TIER, validator audit scope, rollback plan, variant triggers.
- **User validates**: interpretation accuracy, feasibility of commands, scope realism, risk acceptance, cost/benefit.
- **Signature block (recorded in packet)**:
  ```
  ## Signature & Enrichment Log
  Signature: alice_25121512345 (immutable)
  Timestamp: 2025-12-15 12:34:56 UTC
  Notes:
  - Clarified: OAuth flow required; DONE_MEANS updated to include callback test.
  - Spec Enrichment: Added A4.2.7 OAuth; SPEC_CURRENT updated.
  - Validator Scope: DAL audit skipped; LLM boundary + security required.
  Locked: DONE_MEANS, TEST_PLAN, IN_SCOPE_PATHS, OUT_OF_SCOPE, validator checks.
  ```
- Any change after signature => create `WP-{id}-v2` with a new signature.

---

## Spec Enrichment Rules (Drift Prevention)

**When is spec enrichment valid? When is it scope creep?** Use this decision matrix:

### Decision Matrix: Valid Enrichment vs. Scope Creep

| Scenario | Valid? | Reason | Action |
|----------|--------|--------|--------|
| **Clarifying existing anchor** (e.g., "A3.2.1 OAuth: what counts as 'authenticated'?") | ✅ Yes | Removes ambiguity; doesn't change requirement | Enrich → create v1.1 → run validator-spec-regression → obtain signature |
| **Adding missing anchor** (user says "we also need rate limiting" but spec doesn't cover it) | ✅ Yes | New requirement, not change to existing | Enrich → add A5.1.2 Rate Limiting → run validator-spec-regression → obtain signature |
| **Rewriting existing anchor** (e.g., "A3.2.1 said 'email auth only', now add social login") | ❌ No | Changes requirement after user signed off on DONE_MEANS | Escalate to user; new signature required; check if existing packets affected |
| **Changing DONE_MEANS after signature** (Orchestrator adds new checkpoint mid-work) | ❌ No | Violates Signature Lock principle; scope creep | Create WP-{id}-v2 with new signature; don't modify original |
| **Adding out-of-scope requirement as "nice-to-have"** (packet says "no caching", user asks for "optional Redis support") | ❌ No | Introduces uncertainty; optional features are scope creep | Document in packet OUT_OF_SCOPE; create separate WP if needed |
| **Removing anchor** (spec v1.0 required feature X, now X is obsolete) | ✅ Yes | Spec evolution; old anchor becomes "deprecated" | Create v2.0 with deprecation notice; run validator-spec-regression; check dependent packets |

### Spec Enrichment Process

**When enrichment is needed** (before packet creation):

1. **Intake phase**: Orchestrator identifies gap (requirement not covered by current spec)
2. **Enrich spec**: Create new spec version (e.g., v1.0 → v1.1 or v1.0 → v2.0 if breaking)
3. **Run validator-spec-regression**: Ensure all previous anchors still exist (no accidental deletions)
4. **Obtain signature**: User validates enrichment doesn't change existing intent; provides signature
5. **Update SPEC_CURRENT.md**: Point to new spec version
6. **Create packet**: Now reference new anchors with confidence
7. **Log in SIGNATURE_AUDIT.md**: Timestamp, user, version, enrichment scope

### Spec Versioning Strategy

**Minor enrichment** (v1.0 → v1.1): Clarifications, new anchors, no anchor changes
- Old packets remain valid (old anchors still exist)
- No validator-spec-regression needed (backward compatible)
- Example: v1.0 has "User Authentication", v1.1 adds "Multi-Factor Auth" without changing auth baseline

**Major enrichment** (v1.0 → v2.0): Rewrites, anchor changes, removals
- Old packets may be invalid (anchors removed or changed)
- **Validator-spec-regression REQUIRED** before publishing
- Example: v1.0 specified SQLite only, v2.0 introduces pluggable storage (trait changes)

### Red Flags (Probable Scope Creep)

- Enrichment is proposed **during implementation** (sign says "this feature is harder, let's also add...")
- Enrichment is "optional" or "nice-to-have"
- User never saw enrichment before signature (Orchestrator enriched unilaterally)
- Enrichment changes DONE_MEANS of existing packet (should create -v2)
- Enrichment removes existing anchor without deprecation notice

---

## Spec Version Migration (Breaking Changes)

When spec changes significantly (v1.0 → v2.0), existing packets may reference old anchors. Use this process:

### Pre-Migration: Audit Impact
```bash
# Find all packets referencing v1.0 anchors
grep -r "v1.0" docs/task_packets/ | wc -l

# Identify anchors that will be removed/renamed
grep "^A2.3" docs/Master_Spec_v1.0.md > old_anchors.txt
grep "^A2.3" docs/Master_Spec_v2.0.md > new_anchors.txt
diff old_anchors.txt new_anchors.txt  # Shows gaps
```

### During Migration: Anchor Mapping
Create `docs/SPEC_MIGRATION_v1_to_v2.md`:
```markdown
## Anchor Mapping (v1.0 → v2.0)

- A2.3.1 (old: "SQLite Storage") → A2.4.1 (new: "Pluggable Storage Trait")
- A2.3.2 (old: "Direct SQL") → DEPRECATED (no replacement; SQL access now via trait only)
- A3.2.1 (old: "Auth") → A3.2.1 (unchanged)
```

### Post-Migration: Packet Updates
- Old packets **remain valid** if referenced anchor exists (even if moved)
- Old packets **become invalid** if anchor removed (escalate to Orchestrator)
- Create new anchor deprecation period (3 phases): DEPRECATED → REMOVED → gone

---

## Templates (Copy, then fill)

### Task Packet (10 Required Fields)
```
# Task Packet: WP-{phase}-{name}
- TASK_ID: WP-{phase}-{name}
- STATUS: Ready-for-Dev | In-Progress | Ready-for-Validation | Done | Backlog
- SPEC_ANCHOR: {e.g., A2.3.12.3}
- What: {1-2 sentences}
- Why: {rationale + spec reference}
- IN_SCOPE_PATHS:
  - {exact file path 1}
  - {exact file path 2}
- OUT_OF_SCOPE:
  - {explicit deferral + reason}
- DONE_MEANS: 5-10 measurable checkpoints (yes/no), each mapped to SPEC_ANCHOR
- TEST_PLAN:
  - {command 1}
  - {command 2}
  - {manual review if medium/high}
- BOOTSTRAP (Coder work plan):
  - FILES_TO_OPEN: 5-15 files
  - SEARCH_TERMS: 10-20 grep strings
  - RUN_COMMANDS: 3-6 setup/test commands
  - RISK_MAP: 3-8 failure modes -> subsystems
- ROLLBACK_HINT: {git revert <sha> or explicit steps}
- VALIDATION (filled after work):
  - Command:
  - Result:
  - Notes:
- Signature & Enrichment Log: {signature block}
```

### Task Board Entry
```
## Phase {N} Closure Gates (Blocking)
- [WP-1-Storage-Layer] Ready-for-Dev (no blockers)
- [WP-1-AppState-Refactor] Blocked on WP-1-Storage-Layer

## In Progress
- [WP-1-Storage-Layer]

## Ready for Dev
- [WP-1-Migration-Framework]
```

### Orchestrator Handoff (to Coder)
```
Task Packet: docs/task_packets/WP-{id}.md
WP_ID: WP-{id}
RISK_TIER: LOW|MEDIUM|HIGH

Read: ORCHESTRATOR_PROTOCOL.md, CODER_PROTOCOL.md, SPEC_CURRENT.md
Run: just pre-work {WP_ID}
Deliver: BOOTSTRAP block before first change.
Scope is locked to IN_SCOPE_PATHS; OUT_OF_SCOPE is forbidden.
DONE_MEANS and TEST_PLAN are frozen by signature.
```

### Coder BOOTSTRAP Block (before coding)
```
BOOTSTRAP
WP_ID: WP-{id}
RISK_TIER: MEDIUM
TASK_TYPE: FEATURE|DEBUG|REFACTOR|HYGIENE
FILES_TO_OPEN:
- docs/START_HERE.md
- docs/SPEC_CURRENT.md
- docs/task_packets/WP-{id}.md
- src/path/file.rs
- ...
SEARCH_TERMS:
- "TraitName"
- "SqlPool"
- ...
RUN_COMMANDS:
- just dev
- cargo test --manifest-path src/backend/Cargo.toml
- pnpm -C app test
RISK_MAP:
- "Trait mismatch" -> "Storage layer"
- "API contract break" -> "Frontend/IPC"
```

### Validator Report
```
VALIDATION REPORT - WP-{id}
Verdict: PASS | FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-{id}.md (status: {status})
- Spec: {SPEC_CURRENT ref + anchors}

Findings:
- Requirement: {DONE_MEANS item} -> {path:line}; test: {command} (PASS/FAIL)
- Hygiene: {clean/issues}
- Forbidden Patterns: {results}
- Audits: {DAL/LLM/security/etc}

Tests:
- {command}: PASS/FAIL (summary)

Risks/Gaps:
- {residual risks or waivers}

Packet Update:
- STATUS set to Done|In-Progress|Blocked with reason
- Task Board updated
```

### Tool Agent Allowlist (per packet)
```
## Tooling (Allowed for Tool Agent)
- Workdir: {repo root or subdir}
- Allowed commands:
  - just pre-work {WP_ID}
  - cargo test --manifest-path ...
  - pnpm -C app test
  - rg "pattern" src/
- Output limits: {max lines/MB}
- Time limits: {per command}
- Logs: Attach output to VALIDATION section
```

### Evidence Table (inside packet)
```
## Evidence Mapping
| DONE_MEANS | SPEC_ANCHOR | Code Evidence (path:line) | Test Command |
|------------|-------------|---------------------------|--------------|
| "DB is behind trait" | A2.3.12.3 | src/storage/mod.rs:45-120 | cargo test -p core storage_tests |
| "No pool leaks" | A2.3.12.3 | src/api/*.rs imports checked | rg "SqlPool" src/api |
```

### Evidence Validation Patterns (Beyond Code)

**Not all DONE_MEANS have code evidence.** Use these patterns for non-code requirements:

#### Pattern 1: Performance Metrics
**DONE_MEANS**: "API response time < 200ms for user queries"

**Evidence**:
```markdown
Evidence Type: Performance Benchmark
- Metric: response_time_p99
- Baseline (before): 450ms
- Target (after): <200ms
- Actual (after): 185ms ✅ PASS
- Benchmark command: `jq '.response_time_p99' perf_results.json`
- Proof of removability: Revert performance optimization → benchmark shows >200ms ✅
- File evidence: src/api/handlers/user_queries.rs:45-60 (caching implementation)
```

#### Pattern 2: UI/UX Accessibility
**DONE_MEANS**: "All interactive elements keyboard-accessible (WCAG 2.1 AA)"

**Evidence**:
```markdown
Evidence Type: Accessibility Audit
- Tool: axe DevTools / Pa11y
- Violations before: 5 (2 critical)
- Violations after: 0 ✅ PASS
- Test command: `pa11y --standard WCAG2AA https://app/feature`
- Proof of removability: Remove keyboard event handlers → audit fails ✅
- File evidence: app/src/components/FeatureForm.tsx:12-45 (onKeyDown handlers)
- Screenshot: app_accessibility_audit_2025-12-25.png
```

#### Pattern 3: Documentation Completeness
**DONE_MEANS**: "API endpoint documented with request/response examples"

**Evidence**:
```markdown
Evidence Type: Documentation Audit
- Files: docs/api/v1/users.md + src/main.rs doc comments
- Completeness checklist:
  ✓ Endpoint URL and method
  ✓ Required headers + authentication
  ✓ Request body schema (with example)
  ✓ Response body schema (with example)
  ✓ Error codes (documented in src/errors.rs)
  ✓ Rate limits (documented)
- Proof of removability: Remove doc comments → CI check fails (missing docs) ✅
- File evidence: docs/api/v1/users.md:1-50, src/api/handlers.rs:100-120
```

#### Pattern 4: User Feedback / Acceptance
**DONE_MEANS**: "Feature meets user acceptance criteria (approved by stakeholder)"

**Evidence**:
```markdown
Evidence Type: User Sign-Off
- Stakeholder: product-manager@company.com
- Sign-off timestamp: 2025-12-25 14:30:00 UTC
- Acceptance criteria met: ✅ All 5 acceptance criteria verified
  ✓ Feature works as demoed
  ✓ No blocking bugs
  ✓ Performance acceptable
  ✓ UX meets expectations
  ✓ Ready for production
- Sign-off reference: Jira HSK-1234 comment #42
- Proof: Signed change log in docs/FEATURE_ACCEPTANCE.md
```

#### Pattern 5: Security Scan Results
**DONE_MEANS**: "No critical vulnerabilities; all dependencies CVSS <7.0"

**Evidence**:
```markdown
Evidence Type: Security Audit
- Scanner: cargo audit + npm audit
- Vulnerabilities before: 3 critical, 2 high
- Vulnerabilities after: 0 critical, 0 high ✅ PASS
- Command: `cargo audit --deny warnings && npm audit --audit-level high`
- Dependencies updated: Cargo.toml:15, package.json:23-27
- Proof of removability: Revert dependency updates → scan fails ✅
```

#### Pattern 6: Schema/Contract Compliance
**DONE_MEANS**: "API response matches OpenAPI 3.0 schema"

**Evidence**:
```markdown
Evidence Type: Schema Validation
- Tool: swagger-cli validator
- Schema file: docs/openapi-3.0.yaml
- Valid schemas: 12/12 ✅ PASS
- Command: `swagger-cli validate docs/openapi-3.0.yaml`
- Implementation evidence: src/api/handlers/*.rs lines match response types
- Proof of removability: Change response struct → schema validation fails ✅
```

---

### Signature Audit Entry (project root)
```
| Signature | Used By | Timestamp | Purpose | Spec Version | Evidence |
|-----------|---------|-----------|---------|--------------|----------|
| alice_25121512345 | Orchestrator | 2025-12-15 12:34:56 UTC | Packet creation WP-1 | v02.85 | docs/task_packets/WP-1.md |
```

### Justfile Skeleton
```
create-task-packet WP_ID:
  node scripts/create-task-packet.mjs {{WP_ID}}

pre-work WP_ID:
  node scripts/validation/pre-work-check.mjs {{WP_ID}}

post-work WP_ID:
  node scripts/validation/post-work-check.mjs {{WP_ID}}

validator-scan:
  node scripts/validation/validator-scan.mjs

validator-spec-regression:
  node scripts/validation/validator-spec-regression.mjs

validate-workflow WP_ID:
  just pre-work {{WP_ID}}
  just validator-scan
  just validator-spec-regression
  just post-work {{WP_ID}}
```

## Validation Scripts (Modular, One Concern Each)

**Pre-Work Gate (Gate 0)**: `pre-work-check.mjs`
- Verifies task packet file exists: `docs/task_packets/WP-{ID}.md`
- Confirms all 10 required fields present: TASK_ID, STATUS, SPEC_ANCHOR, scope, DONE_MEANS, TEST_PLAN, BOOTSTRAP, ROLLBACK_HINT, VALIDATION, Signature Log
- Scans for placeholder text (`{field_name}`, `TODO(TBD)`, `FIXME`, mock values)
- Exit 0: proceed to implementation; Exit 1: return to Orchestrator for completion
- **Purpose**: prevents handoff until packet is actionable

**Post-Work Gate (Gate 1)**: `post-work-check.mjs`
- Verifies VALIDATION section has outcomes recorded (Command, Result, Notes)
- If TEST_PLAN lists test commands, validates they're documented with results
- For MEDIUM/HIGH risk: verifies `manual review` completed and not BLOCKED
- Checks git diff shows actual changes (work was done)
- Exit 0: work validated, safe to commit; Exit 1: incomplete, return to Coder
- **Purpose**: prevents merge until validation evidence present

**Forbidden Pattern Scanner**: `validator-scan.mjs`
- Scans production code for patterns that indicate incomplete work or unsafe patterns
- **Forbidden** (exceptions allowed in tests only): `unwrap`, `expect(`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`
- **Placeholder patterns** (anywhere): `Mock`, `Stub`, `placeholder`, `hollow`, `{field}`
- **Configurable** per project: scan paths, allowed exceptions
- Exit 0: clean; Exit 1: patterns found, lists all occurrences

**Spec Regression Validator**: `validator-spec-regression.mjs`
- Gates spec integrity and phase progression
- Checks: `docs/SPEC_CURRENT.md` exists and is readable
- Verifies points to valid spec file (e.g., `Master_Spec_v1.0.md`) in repo root
- Validates spec file contains all REQUIRED_ANCHORS (project-defined list):
  ```
  const REQUIRED_ANCHORS = [
    "A2.3.12",    // Example: Storage portability
    "A2.3.11",    // Example: Retention/GC
    "A2.6.7"      // Example: Semantic catalog
  ];
  ```
- Exit 0: spec valid; Exit 1: file missing or anchor gap
- **Purpose**: ensures spec coherence; blocks phase progression on regression

**Custom Auditors** (architecture-specific):
- `validator-dal-audit`: storage layer boundary (no direct DB access outside modules, SQL portability, trait isolation, migration hygiene)
- `validator-error-codes`: typed errors + traceability (no stringly errors, HSK-#### codes, trace IDs in mutations)
- `validator-traceability`: determinism markers (trace_id, job_id, request_id) in mutation points
- `validator-git-hygiene`: .gitignore coverage, no artifacts committed (target/, *.pdb, node_modules)
- `validator-coverage-gaps`: test coverage thresholds (<80% flagged)
- `validator-security`: RCE guardrails, input validation, secret detection

**Phase Gate Validator**: `validator-phase-gate.mjs`
- Reads `docs/TASK_BOARD.md` "Phase {N} Closure Gates (BLOCKING)"
- Verifies every listed blocking WP: status = Done AND VALIDATED
- Runs `just validator-spec-regression` (spec must be current for phase closure)
- Checks no unresolved dependencies (all upstream = VALIDATED)
- Exit 0: clear to proceed; Exit 1: blockers remain
- **Purpose**: gates phase progression; prevents moving forward with incomplete foundations

## Context Window & Prompt Hygiene (Mitigating Context Rot)

**Spec Anchoring Prevents Drift**:
- All instructions live in durable files (spec version, packet, task board), not ephemeral chat history
- Refer to SPEC_ANCHOR IDs instead of pasting entire specs (e.g., "per A2.3.12, implement trait-based storage" vs. entire storage section)
- Spec Signature freezes intent; changes require new signature, creating audit trail

**Retrieval-Style Prompts** (for each agent call):
- Supply only relevant spec slices: read SPEC_ANCHOR sections directly, not full spec
- Include the full task packet (includes Signature Log + DONE_MEANS + TEST_PLAN + BOOTSTRAP)
- Attach latest VALIDATION block (what has been verified so far)
- Keep total context <20KB for Coder/Validator calls; larger context for Orchestrator (who coordinates)

**Short & Structural Prompts**:
- Refer to packet fields by name + ID ("per DONE_MEANS item 3, verify file:line evidence")
- Cite SPEC_ANCHOR IDs instead of explaining requirements ("implement per A2.3.12.3" vs. "implement a trait-based storage layer that...")
- Use packet templates + signature log to avoid re-explaining context every turn

**Long-Running Packets (>5 days)**:
- Have Orchestrator issue brief state digests: current scope, open questions, last validation results, next expected step
- Digest replaces verbose transcript; agents read digest + packet, not entire conversation
- Reduces need to context-search through 50+ turns of negotiation

**Large-Context Models**:
- Even long-context models benefit from hierarchical prompting (anchor IDs + slices)
- Structured logs (validation blocks, BOOTSTRAP, command outputs) are safer for reuse than raw transcripts
- Logs are deterministic and auditable; transcripts drift

**Example Prompt (for Coder)**:
```
WP_ID: WP-1-Storage-Layer
SPEC_ANCHOR: A2.3.12.3
Read: docs/SPEC_CURRENT.md sections A2.3.12.3 + A2.3.12.1
Read: docs/task_packets/WP-1-Storage-Layer.md (full packet)
DONE_MEANS:
  1. "AppState exposes Database as Arc<dyn Trait>"
  2. "No SqlitePool leaks to API layer"
TEST_PLAN: cargo test -p core storage_tests; grep "SqlPool" src/api | wc -l
Task: Implement DONE_MEANS per packet; record evidence; run TEST_PLAN.
```

## Git Hook Integration (Pre-Commit Enforcement)

**File**: `scripts/hooks/pre-commit`

**Purpose**: Prevent broken commits; enforce codex/governance invariants before code enters tree

**Sample Implementation**:
```bash
#!/bin/bash
set -e

# 1. Verify hard invariants (governance rules)
node scripts/validation/codex-check.mjs

# 2. Warn about TODOs without tracking IDs
grep -r "TODO()" src/ app/ && \
  echo "❌ TODOs must have tracking ID: TODO(HSK-####)" && exit 1

# 3. Forbid placeholder values in code
grep -r "{" src/ app/ --include="*.rs" --include="*.ts" --include="*.tsx" && \
  echo "⚠️  Found placeholder text in code" && exit 1

# 4. Lint and format
cargo fmt --check || exit 1
pnpm run lint --quiet || exit 1

echo "✅ Pre-commit checks passed"
```

**Wire via**:
```bash
git config core.hooksPath scripts/hooks
```

**Effect**: Commit blocked until checks pass; catches errors before pushing to remote

---

## Documentation Structure (Project-Agnostic)

Create `docs/` directory with:

```
docs/
├── START_HERE.md              # Entry point; repo map; AI agent workflow
├── SPEC_CURRENT.md            # Pointer: "Current spec is Master_Spec_vX.Y.Z.md"
├── ARCHITECTURE.md            # Module layout, responsibilities, entry points, RDD
├── RUNBOOK_DEBUG.md           # Error codes, bug triage, debug patterns
├── QUALITY_GATE.md            # Risk tier definitions, Gate 0/1 requirements
├── TASK_BOARD.md              # Master task status + Phase gates + dependencies
├── SIGNATURE_AUDIT.md         # Immutable registry of consumed signatures
├── OWNERSHIP.md               # Path/area ownership (optional)
├── task_packets/
│   ├── README.md              # Packet naming convention, validation commands
│   ├── TEMPLATE.md            # Copy-paste packet template
│   └── WP-1-*.md, WP-2-*.md   # (20-30 actual packets during active work)
├── adr/                       # Architecture Decision Records
│   ├── ADR-0001-workflow.md
│   └── ADR-0002-spec-governance.md
└── agents/
    └── AGENT_REGISTRY.md      # Map of contributing agents/models
```

**Master Spec location** (repo root):
```
Master_Spec_v1.0.md            # Current authoritative spec (~1MB+)
Master_Spec_v0.9.md            # (Previous versions for audit trail)
{Project} Codex v1.0.md        # Governance rules (versioned)
{Project} Codex v0.9.md
```

---

## Phase Gate Pattern (Blocking Phase Progression)

**Command**: `just validator-phase-gate {PHASE_ID}`

**What It Does**:
1. Reads `docs/TASK_BOARD.md` section "Phase {PHASE_ID} Closure Gates (BLOCKING)"
2. Verifies every listed blocking WP: STATUS = Done AND VALIDATED
3. Runs `just validator-spec-regression` (spec must be current)
4. Checks no unresolved dependencies (all upstream = VALIDATED)
5. Returns exit 0 (clear to proceed) or exit 1 (blockers remain)

**Usage Example**:
```bash
# Before closing Phase 1, run:
just validator-phase-gate Phase-1

# Output:
# ✅ Phase 1 Closure Gates check...
# ❌ BLOCKED: WP-1-Storage-Layer (status: Done, not VALIDATED)
# ❌ BLOCKED: WP-1-AppState-Refactor (status: In-Progress)
# Exit 1

# Once all gates pass:
# ✅ All blocking WPs VALIDATED
# ✅ Spec regression check passed
# ✅ No unresolved dependencies
# Exit 0
```

**Gates Phase Progression** (in CI/CD or release script):
```bash
if just validator-phase-gate Phase-1; then
  echo "✅ Phase 1 ready to close"
  git tag phase-1-complete
  git push origin phase-1-complete
else
  echo "❌ Blockers remain; cannot close phase"
  echo "Run: just TASK_BOARD.md to review blocking WPs"
  exit 1
fi
```

**Why This Matters**:
- Prevents moving forward with incomplete foundations
- Enforces spec coherence (regression check)
- Makes dependency chains explicit
- Blocks merges to main/release until gates pass

---

## Implementation Order (Phased Rollout)

**DO NOT build everything at once.** Use this phased approach so each phase unblocks the next.

### Phase 0: Foundation (Days 1-2)
**Goal**: Create the spec infrastructure and first task packet template.

1. Create `Master_Spec_v1.0.md` (can start minimal: ~20KB with 5-10 sections and anchors)
2. Create `docs/SPEC_CURRENT.md` (pointer file, 3 lines)
3. Create `docs/SIGNATURE_AUDIT.md` (empty table, 5 lines)
4. Create `docs/TASK_BOARD.md` (empty structure, 10 lines)
5. Create `docs/task_packets/TEMPLATE.md` (copy from Templates section below, 50 lines)
6. Create `scripts/create-task-packet.mjs` (simple Node script: generate packet from template)
7. Test: `node scripts/create-task-packet.mjs WP-0-Test` → produces `docs/task_packets/WP-0-Test.md`

**Deliverable**: You can now create task packets.

### Phase 1: Validation Gates (Days 3-4)
**Goal**: Enforce packet completeness before handoff.

1. Create `scripts/validation/pre-work-check.mjs` (verify 10 required fields, no placeholders)
2. Create `scripts/validation/post-work-check.mjs` (verify VALIDATION results recorded)
3. Create `scripts/validation/validator-spec-regression.mjs` (verify spec file exists + anchors)
4. Create `scripts/validation/validator-scan.mjs` (grep for forbidden patterns)
5. Create `justfile` with commands:
   - `just create-task-packet WP_ID`
   - `just pre-work WP_ID`
   - `just post-work WP_ID`
   - `just validator-scan`
   - `just validator-spec-regression`
6. Test:
   - `just create-task-packet WP-1-Feature` → creates packet
   - `just pre-work WP-1-Feature` → FAIL (has placeholders)
   - Edit packet, remove placeholders
   - `just pre-work WP-1-Feature` → PASS

**Deliverable**: Packet quality gates work; you can't hand off incomplete packets.

### Phase 2: Governance Rules (Days 5-6)
**Goal**: Define hard invariants for your project.

1. Create `{Project} Codex v1.0.md` (copy template from section below; ~40KB)
   - Define 10-20 hard invariants (CX-101, CX-102, etc.)
   - Examples: "LLM calls only via module X", "No direct DB access outside storage", "All errors typed, not strings"
2. Create `scripts/validation/codex-check.mjs` (grep for codex violations)
3. Create `scripts/hooks/pre-commit` (wire codex-check + lint + tests)
4. Wire hook: `git config core.hooksPath scripts/hooks`
5. Test: Try to commit code that violates a codex rule → commit blocked

**Deliverable**: Hard invariants are enforced at commit time; code quality guaranteed.

### Phase 3: Protocol Files (Days 7-8)
**Goal**: Document workflow for each agent.

1. Create `docs/ORCHESTRATOR_PROTOCOL.md` (copy template + customize, ~50KB)
   - Pre-orchestration gates checklist
   - Signature pause protocol
   - Packet creation workflow
2. Create `docs/CODER_PROTOCOL.md` (copy template + customize, ~30KB)
   - Pre-coding checklist (scope adequacy)
   - Validation order (TEST_PLAN → manual review → post-work)
   - Evidence recording format
3. Create `docs/VALIDATOR_PROTOCOL.md` (copy template + customize, ~20KB)
   - Pre-flight checks
   - Evidence verification steps
   - Audit scope (DAL, security, hygiene, etc.)
4. Test: Train an agent on each protocol; run through a mock task packet

**Deliverable**: Agents know their workflow; can operate semi-autonomously.

### Phase 4: Custom Validators (Days 9-12)
**Goal**: Add domain-specific validation (storage DAL, API contracts, security, etc.).

1. Create `scripts/validation/validator-dal-audit.mjs` (if storage-based)
   - Check trait boundaries, SQL portability, migration hygiene
2. Create `scripts/validation/validator-error-codes.mjs`
   - Enforce typed errors, trace ID logging, no stringly errors
3. Create `scripts/validation/validator-security.mjs`
   - RCE guardrails, input validation, secret detection
4. Create `scripts/validation/validator-git-hygiene.mjs`
   - .gitignore coverage, no artifacts committed
5. Create `scripts/validation/validator-phase-gate.mjs`
   - Block phase progression until all gates VALIDATED
6. Wire into `justfile`: `just validator-{concern}`, `just validator-phase-gate PHASE`
7. Test: Create a test task packet; run all validators; fix violations until PASS

**Deliverable**: Rich validation catches domain-specific issues; phase gates work.

### Phase 5: Integration & Docs (Days 13-14)
**Goal**: Make the workflow real and documented.

1. Create `docs/START_HERE.md` (entry point: repo map, AI agent workflow, links)
2. Create `docs/ARCHITECTURE.md` (module layout, responsibilities, entry points)
3. Create `docs/RUNBOOK_DEBUG.md` (error codes, debug patterns)
4. Create `docs/QUALITY_GATE.md` (risk tier definitions, Gate 0/1 requirements)
5. Create `docs/agents/AGENT_REGISTRY.md` (map of contributing agents/models)
6. Create first **real** task packet for your project's first feature
   - Use all the tools: pre-work gate, signature pause, BOOTSTRAP, etc.
   - Run full workflow: create → code → validate → merge
7. Test: Dry run the entire workflow on a low-risk feature

**Deliverable**: Workflow is live; documented; agents can operate independently.

---

## Real Working Examples

### Example 1: Complete Task Packet (Filled In)

**File**: `docs/task_packets/WP-1-User-Authentication.md`

```markdown
# Task Packet: WP-1-User-Authentication

## Metadata
- TASK_ID: WP-1-User-Authentication
- STATUS: Done (VALIDATED)
- DATE: 2025-12-15 10:00 UTC
- REQUESTOR: alice
- USER_SIGNATURE: alice_25121510000
- SPEC_VERSION: Master_Spec_v1.0.md

## What
Implement email + password authentication endpoint (POST /auth/login) returning JWT token with 24-hour TTL.

## Why
Foundation for user identity system. Spec A3.2.1 requires "Users authenticate with email+password, receive bearer token, subsequent requests validated via Bearer header."
Blocks: WP-2-Authorization-Roles, WP-3-Session-Management

## Scope
- IN_SCOPE_PATHS:
  - src/api/handlers/auth.rs
  - src/api/models/user.rs
  - src/services/jwt.rs
  - tests/integration/auth_test.rs
  - Cargo.toml (add jwt crate)
- OUT_OF_SCOPE:
  - OAuth/third-party auth (Phase 2)
  - Password reset/recovery (Phase 1.5)
  - Rate limiting (separate WP-0-Rate-Limiting)
  - Database schema changes (data layer frozen per A2.3.12)

## DONE_MEANS
1. POST /auth/login accepts {email, password} JSON
2. Valid credentials return {jwt_token, expires_in: 86400} JSON
3. Invalid credentials return 401 + error code AUTH-001
4. JWT decodes to {user_id, email, iat, exp}
5. Subsequent requests with "Authorization: Bearer {token}" header validated (token not expired, signature valid)
6. Expired token returns 401 + error code AUTH-002
7. Malformed token returns 401 + error code AUTH-003
8. Integration test: login → validate token → request with Bearer → success

## TEST_PLAN
- `cargo test -p api auth_integration` (PASS; 8 test cases)
- `curl -X POST http://localhost:3000/auth/login -H "Content-Type: application/json" -d '{"email":"test@example.com","password":"password123"}'` (returns jwt_token)
- `curl http://localhost:3000/user -H "Authorization: Bearer {token}"` (PASS, returns user data)
- `curl http://localhost:3000/user -H "Authorization: Bearer invalid"` (returns 401)
- `grep "JWT\|jwk\|token" src/api/models/user.rs | wc -l` (returns > 0, proves JWT in code)
- Code review: validate no plaintext passwords logged, no token leakage to stdout

## BOOTSTRAP (Coder's Work Plan)
- FILES_TO_OPEN:
  - docs/SPEC_CURRENT.md (section A3.2.1)
  - docs/task_packets/WP-1-User-Authentication.md (this packet)
  - src/api/handlers/mod.rs (entry point)
  - src/services/mod.rs (service structure)
  - tests/integration/auth_test.rs (test patterns)
  - Cargo.toml (dependencies)
  - docs/RUNBOOK_DEBUG.md (error codes section)
- SEARCH_TERMS:
  - "LoginRequest", "TokenResponse", "validate_jwt"
  - "user_id", "email_address"
  - "jsonwebtoken", "chrono"
  - "401", "AUTH-"
  - "Bearer", "Authorization"
- RUN_COMMANDS:
  - `cargo build --manifest-path src/Cargo.toml`
  - `cargo test -p api --test integration` (baseline)
  - `sqlx database create` (if new DB needed)
- RISK_MAP:
  - "JWT library vulnerability" → Security audit required (MEDIUM risk)
  - "Password stored plaintext" → Must use bcrypt (HIGH risk)
  - "Token expiry not enforced" → MEDIUM risk
  - "API contract mismatch" → Integration test catches (LOW risk)

## RISK_TIER
MEDIUM
- Why: Security-sensitive (passwords, tokens), but single module (no IPC yet)
- Triggers: manual review required, security audit in validator
- Rollback: `git revert <sha>`; feature flag gated, can be disabled in config

## ROLLBACK_HINT
If authentication breaks:
1. `git revert <commit-hash>`
2. Restart server: `cargo run --bin api`
3. Test: `curl /auth/login` should return 404 (endpoint gone)

## VALIDATION (filled by Coder)

### BOOTSTRAP Output
```
WP_ID: WP-1-User-Authentication
RISK_TIER: MEDIUM
TASK_TYPE: FEATURE
FILES_TO_OPEN:
- docs/SPEC_CURRENT.md (A3.2.1)
- src/api/handlers/auth.rs (will create)
- src/services/jwt.rs (will create)
- tests/integration/auth_test.rs (will create)
- Cargo.toml (will update)
SEARCH_TERMS:
- "LoginRequest", "TokenResponse"
- "validate_jwt", "issue_token"
- "jsonwebtoken", "bcrypt"
- "401", "AUTH-"
RUN_COMMANDS:
- cargo build --manifest-path src/Cargo.toml
- cargo test -p api auth_integration
- curl -X POST http://localhost:3000/auth/login ...
RISK_MAP:
- "JWT library vulnerability" -> Security audit
- "Password stored plaintext" -> Bcrypt required
```

### Test Results
```
Command: cargo test -p api auth_integration
Result: PASSED (8/8 cases)
  ✓ login_valid_credentials
  ✓ login_invalid_password
  ✓ login_invalid_email
  ✓ token_decode_valid
  ✓ token_expired
  ✓ token_malformed
  ✓ bearer_header_validation
  ✓ bearer_token_not_found

Command: grep "bcrypt" src/services/jwt.rs | wc -l
Result: 2 (passwords are hashed)

Command: grep "println\|dbg\|password" src/services/jwt.rs | grep -v test
Result: 0 (no password logging in production)

manual review: PASSED (security review)
```

## Signature & Enrichment Log

**Signature**: alice_25121510000 (immutable)
**Timestamp**: 2025-12-15 10:00 UTC

### User-Orchestrator Collaboration Notes:
- **Clarified**: User confirmed email+password only (OAuth deferred). DONE_MEANS expanded to include Bearer token validation in subsequent requests.
- **Spec Enrichment**: Spec section A3.2.1 already covers this fully; no enrichment needed.
- **Rubric Adjustments**: Added requirement 5 (Bearer token validation in API). TEST_PLAN now includes curl test of authenticated request. manual review added for MEDIUM risk (security).
- **Risks Acknowledged**: User approved MEDIUM risk; rollback plan is git revert + restart. Security audit required before merge.

### Locked Intent:
- **DONE_MEANS**: [frozen above]
- **TEST_PLAN**: [frozen above]
- **IN_SCOPE_PATHS**: [frozen above]
- **OUT_OF_SCOPE**: [frozen above]
- **Validator Audit Scope**: hygiene (bcrypt, no password logging), security (JWT lib), error codes (AUTH-001, AUTH-002, AUTH-003)
```

---

### Example 2: Validator Report Output

```markdown
# VALIDATION REPORT - WP-1-User-Authentication

**Verdict**: PASS

## Scope Inputs
- Task Packet: docs/task_packets/WP-1-User-Authentication.md
- Spec: Master_Spec_v1.0.md (section A3.2.1)
- USER_SIGNATURE: alice_25121510000 (valid, not reused)

## Findings

### Requirement Mapping (Evidence Verification)
| DONE_MEANS | Spec Anchor | Code Evidence | Test Command | Status |
|------------|-------------|---------------|--------------|--------|
| "POST /auth/login accepts email+password" | A3.2.1 | src/api/handlers/auth.rs:12-35 (LoginRequest struct) | cargo test login_valid_credentials | PASS |
| "Returns jwt_token + expires_in" | A3.2.1 | src/api/handlers/auth.rs:40-52 (TokenResponse struct) | curl /auth/login | PASS |
| "Invalid credentials return 401" | A3.2.1 | src/api/handlers/auth.rs:60-75 (error handling) | cargo test login_invalid_password | PASS |
| "JWT decodes to user_id+email+iat+exp" | A3.2.1 | src/services/jwt.rs:30-60 (token claims) | cargo test token_decode_valid | PASS |
| "Bearer header validation" | A3.2.1 | src/api/middleware/auth.rs:15-40 (middleware) | cargo test bearer_header_validation | PASS |
| "Expired token rejected" | A3.2.1 | src/api/middleware/auth.rs:45-65 (exp check) | cargo test token_expired | PASS |
| "Malformed token rejected" | A3.2.1 | src/api/middleware/auth.rs:70-85 (parse error) | cargo test token_malformed | PASS |
| "Integration test: login → Bearer → success" | A3.2.1 | tests/integration/auth_test.rs:50-100 | cargo test auth_integration | PASS |

### Hygiene & Forbidden Patterns
- Forbidden patterns scan: PASS (no unwrap, expect, todo, println in src/services/jwt.rs production code)
- Placeholder check: PASS (no {field} or TODO(TBD) in code)
- Error codes: PASS (AUTH-001, AUTH-002, AUTH-003 documented in RUNBOOK_DEBUG.md)

### Security Audit
- Password hashing: PASS (bcrypt with cost 12, verified in test)
- No password logging: PASS (grep password src/services/jwt.rs returns 0 in production code)
- JWT library: PASS (jsonwebtoken v9.0, security audit OK)
- Token expiry: PASS (TTL 86400 seconds enforced)

### Test Verification
- TEST_PLAN execution: PASS (all 8 integration tests passed)
- Coverage: PASS (auth module 92% coverage, above 80% threshold)
- Removal check: PASS (removing token validation code breaks 3+ tests)

### Residual Risks / Waivers
- None. All gates passed. No waivers needed.

## Packet Update
- **STATUS**: Done (VALIDATED)
- **Task Board**: Move WP-1-User-Authentication to Done + mark VALIDATED
- **Next**: WP-2-Authorization-Roles can now proceed (was blocked on WP-1)
```

---

### Example 3: Task Board (Phase 1)

```markdown
# TASK_BOARD.md

## Phase 1 Closure Gates (BLOCKING)

Must be Done + VALIDATED before Phase 1 can close:

- [WP-1-User-Authentication] **Done (VALIDATED)** ✅
  - Blocker: None
  - Unblocks: WP-2-Authorization-Roles, WP-3-Session-Management

- [WP-1-Database-Migrations] **Done (VALIDATED)** ✅
  - Blocker: None
  - Unblocks: WP-1-Storage-Abstraction

- [WP-1-Error-Codes-Registry] **Done (VALIDATED)** ✅
  - Blocker: None
  - Unblocks: All (error codes are foundation)

- [WP-1-Logging-Framework] **Ready-for-Dev** (not yet started)
  - Blocker: None
  - Unblocks: WP-1-Observability

## In Progress

- [WP-1-Logging-Framework] Started 2025-12-16, by coder_model_v2

## Ready for Validation

- [WP-1-API-Contracts] Implementation complete, awaiting validator review (SLA: 2 days)

## Ready for Dev

- [WP-1-Observability] Spec'd, no blockers, awaiting coder assignment (SLA: 10 days)
- [WP-1-Metrics-Collection] Spec'd, awaiting blockers WP-1-Logging-Framework to clear

## Backlog

- [WP-2-Authorization-Roles] Blocked on WP-1-User-Authentication (ready when Phase 1 closes)
- [WP-2-Session-Management] Blocked on WP-1-User-Authentication
```

---

## Governance Rules (Codex) Template

**File**: `{Project} Codex v1.0.md` (~40-60KB, customize for your project)

```markdown
# {Project} Codex v1.0.md

## Hard Invariants (Governance Rules)

### CX-101: LLM Integration Boundary
**Rule**: All LLM API calls (chat, completion, embeddings) must go through `/src/backend/llm/client.rs`
**Scope**: Production code only (tests exempt)
**Rationale**: Centralized control, cost tracking, rate limiting, prompt injection prevention
**Violation**: Direct `openai_client.call(...)` or HTTP requests to LLM API outside `/src/backend/llm/`
**Waiver**: None (hard blocker)

### CX-102: Database Access Layer
**Rule**: Database queries only via `src/backend/storage/` module. No direct sqlx::query calls outside storage/.
**Scope**: Production code (tests exempt)
**Rationale**: Dual-backend readiness (SQLite → PostgreSQL), migration consistency, audit trail
**Violation**: `sqlx::query` or `.execute()` outside src/backend/storage/
**Waiver**: None (hard blocker)

### CX-103: Error Handling
**Rule**: All errors must be typed (custom enum), never string errors. Error codes prefixed {PROJECT_CODE}-#### (e.g., AUTH-001).
**Scope**: Production code (tests can use anyhow)
**Rationale**: Deterministic error handling, user-facing messages, post-mortem analysis
**Violation**: `Err(String::from(...))` or `anyhow!(...)` in production code
**Waiver**: None (hard blocker)

### CX-104: Logging & Observability
**Rule**: All mutations (create, update, delete, state changes) logged with trace_id. No plaintext passwords/secrets.
**Scope**: Production code
**Rationale**: Auditability, compliance, debugging, incident response
**Violation**: Missing trace_id, passwords/API keys logged, debug!() or println!() in production
**Waiver**: Acceptable for read-only operations; mutation logging non-negotiable

### CX-105: Test Coverage
**Rule**: New code must maintain >=80% coverage. Removal-style tests required (code removed = test fails).
**Scope**: Production code
**Rationale**: Regression detection, confidence in refactors
**Violation**: Coverage <80%, tests don't check behavior (just mock)
**Waiver**: Below 80% requires explicit team approval + documented reason

### CX-106: TODO Tracking
**Rule**: All TODOs must have tracking ID format: TODO(PROJECT-####) with GitHub issue reference.
**Scope**: Production code
**Rationale**: Prevent accumulation of dead TODOs; force closure
**Violation**: TODO() without ID, TODO(TBD), TODO(fixme)
**Waiver**: None (catch at pre-commit)

### CX-107: No Speculative Code
**Rule**: Code must implement exactly DONE_MEANS, nothing more. No "future-proofing" or "optional" features.
**Scope**: All code
**Rationale**: Scope creep prevention, evidence clarity, maintainability
**Violation**: Code that doesn't map to DONE_MEANS, unused abstractions, "phase 2" code in phase 1
**Waiver**: Refactoring code is exempt if no behavior change

### CX-108: Placeholder Removal
**Rule**: Zero placeholder values in production code. {field}, FIXME, Mock, Stub, TBD, placeholder must not appear.
**Scope**: All code
**Rationale**: Prevents incomplete code reaching production
**Violation**: Grep finds placeholder patterns
**Waiver**: None (catch at pre-commit)

### CX-109: Schema Immutability (Phase 1)
**Rule**: Database schema is frozen during Phase 1. Storage DAL trait contract (CX-102) cannot change.
**Scope**: src/backend/storage/trait.rs
**Rationale**: Storage is foundational; changes cascade to all code
**Violation**: Adding/removing trait methods, changing signatures
**Waiver**: Only if ALL Phase 1 downstream code updated in same packet (use WP-id-v2)

### CX-110: Git Hygiene
**Rule**: Commits must be atomic, reference WP_ID, pass pre-commit hooks.
**Scope**: All commits to main
**Rationale**: Traceability, revertability, compliance
**Violation**: Merge commits without WP reference, commits that fail tests, uncommitted changes blocking merge
**Waiver**: None (enforced at git hook + CI)
```

---

## Protocol File Templates

### ORCHESTRATOR_PROTOCOL.md Template

```markdown
# ORCHESTRATOR_PROTOCOL.md

## Pre-Orchestration Checklist (BLOCKING GATES)

Before accepting ANY user prompt, Orchestrator MUST verify:

### Gate 1: Spec Currency
- [ ] `docs/SPEC_CURRENT.md` exists and is readable
- [ ] Points to valid spec file (e.g., `Master_Spec_v1.0.md`)
- [ ] Run `just validator-spec-regression` → PASS
- [ ] If FAIL: escalate to user; spec must be fixed before proceeding

### Gate 2: Task Board Fresh
- [ ] `docs/TASK_BOARD.md` exists
- [ ] All In-Progress items have SLA tracked (started date recorded)
- [ ] No stale items (>30 days In-Progress) without escalation
- [ ] Phase N Closure Gates listed explicitly

### Gate 3: Governance Current
- [ ] `{Project} Codex v1.0.md` exists
- [ ] All protocol files current (ORCHESTRATOR, CODER, VALIDATOR)
- [ ] Pre-commit hooks wired and working

### Gate 4: Supply Chain
- [ ] Run `cargo deny` → no blockers
- [ ] Run `npm audit` (if applicable) → no critical
- [ ] Dependency check: no major version changes without review

### Gate 5: Signature Audit
- [ ] `docs/SIGNATURE_AUDIT.md` exists
- [ ] All previous signatures logged (audit trail complete)

---

## Signature Pause Protocol

When user prompt arrives, Orchestrator MUST:

1. **Intake**: Extract explicit requirements + implied constraints
2. **Coverage Check**: Does prompt "clearly cover" scope, risks, success criteria, dependencies, rollback?
   - If NO: request clarification; escalate; do NOT proceed
3. **Spec Anchoring**: Map requirements to SPEC_ANCHOR
   - If gap: enrich spec (new version) + obtain signature before proceeding
4. **Signature Pause**: PAUSE and collaborate with user
   - Propose DONE_MEANS (5-10 measurable checkpoints)
   - Propose TEST_PLAN (executable commands)
   - Propose IN/OUT scope (exact files)
   - Propose RISK_TIER (LOW/MEDIUM/HIGH)
   - Propose validator audit scope
5. **User Validation**: User MUST vet and approve
   - Interpretation accuracy? Feasibility? Scope realistic? Risk acceptable?
6. **Signature**: User provides signature `{username}{DDMMYYYYHHMM}`
7. **Packet Creation**: Create packet, run `just pre-work`, update Task Board

---

## Packet Creation Checklist

When creating packet, Orchestrator MUST fill:

- [ ] TASK_ID: `WP-{phase}-{name}`
- [ ] STATUS: `Ready-for-Dev`
- [ ] SPEC_ANCHOR: cite exact anchor from spec (e.g., A2.3.12.3)
- [ ] What: 1-2 sentence description
- [ ] Why: rationale + business value
- [ ] IN_SCOPE_PATHS: 5-20 exact files (not globs)
- [ ] OUT_OF_SCOPE: explicit deferrals
- [ ] DONE_MEANS: 5-10 measurable checkpoints, 1:1 with SPEC_ANCHOR
- [ ] TEST_PLAN: copy-paste bash commands
- [ ] BOOTSTRAP: FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP
- [ ] ROLLBACK_HINT: git revert or manual steps
- [ ] RISK_TIER: LOW/MEDIUM/HIGH
- [ ] Signature & Enrichment Log: frozen intent

---

## Handoff to Coder

Issue packet + provide:
1. Packet path: `docs/task_packets/WP-{id}.md`
2. WP_ID: `WP-{id}`
3. RISK_TIER: `LOW | MEDIUM | HIGH`
4. Authority docs: `SPEC_CURRENT.md`, `CODER_PROTOCOL.md`
5. Command: `just pre-work WP-{id}` (must PASS)
6. Confirmation: "Packet is ready for coding. No changes permitted after signature."
```

---

### CODER_PROTOCOL.md Template

```markdown
# CODER_PROTOCOL.md

## Pre-Coding Checklist (BLOCKING GATES)

### Gate 0: Packet Exists & Complete
- [ ] Task packet file exists: `docs/task_packets/WP-{id}.md`
- [ ] All 10 required fields present
- [ ] No placeholder text (`{field_name}`, `TODO(TBD)`)
- [ ] Run `just pre-work WP-{id}` → PASS (exit 0)
- [ ] If FAIL: return packet to Orchestrator

### Gate 1: Scope Adequacy
- [ ] Can I identify all affected files? (yes/no)
- [ ] Are scope boundaries clear? (yes/no)
- [ ] Are there unexpected dependencies? (list them)
- [ ] Is scope realistic for RISK_TIER? (yes/no)
- [ ] If NO to any: escalate to Orchestrator before starting

### Gate 2: Understand Spec
- [ ] Read `docs/SPEC_CURRENT.md` section on SPEC_ANCHOR
- [ ] Understand DONE_MEANS: can I explain each one?
- [ ] Understand TEST_PLAN: will each command actually prove DONE_MEANS?

---

## Implementation Workflow

### Step 1: Output BOOTSTRAP (BEFORE FIRST CODE CHANGE)
Create BOOTSTRAP block in packet:
```
BOOTSTRAP
WP_ID: WP-{id}
RISK_TIER: {LOW|MEDIUM|HIGH}
TASK_TYPE: {FEATURE|DEBUG|REFACTOR|HYGIENE}
FILES_TO_OPEN: [read packet FILES_TO_OPEN]
SEARCH_TERMS: [read packet SEARCH_TERMS]
RUN_COMMANDS: [read packet RUN_COMMANDS]
RISK_MAP: [read packet RISK_MAP]
```
Set task packet Status: In Progress + claim fields and create a docs-only bootstrap claim commit (Validator status-syncs `main`)

### Step 2: Implement Strictly Within Scope
- Only modify files in IN_SCOPE_PATHS
- Do NOT touch OUT_OF_SCOPE files
- Do NOT implement anything not in DONE_MEANS
- Enforce hard invariants (CX-101, CX-102, CX-103, etc.)
- No TODO() without tracking ID: `TODO(PROJECT-1234)`

### Step 3: Run TEST_PLAN
Execute each command from TEST_PLAN section:
```
Command: cargo test -p {module} {tests}
Result: PASSED (N/N cases)
  ✓ case_1
  ✓ case_2
  ...

Command: {grep/curl/etc}
Result: [output]
```
Record in packet VALIDATION block.

### Step 4: For MEDIUM/HIGH Risk: manual review
Record pass/fail in VALIDATION block.

### Step 5: Post-Work Gate
Run `just post-work WP-{id}` (verifies VALIDATION results recorded)
- Exit 0: work validated, safe to commit
- Exit 1: incomplete, return to implementation

### Step 6: Status Sync (Task Board)
- Mark task packet STATUS for handoff (e.g., `Implementation complete; awaiting validation`)
- Notify Validator; Validator status-syncs the Operator-visible Task Board on `main` and moves the WP to Done on PASS/FAIL

### Step 7: Prepare Commit
Commit message format:
```
feat: WP-{id} - {short description}

{Detailed explanation of what changed and why}

Evidence:
- {DONE_MEANS 1}: src/file.rs:45-60
- {DONE_MEANS 2}: test result PASS
- {DONE_MEANS 3}: grep "pattern" returns N matches

WP_ID: WP-{id}
Refs: #{issue_number}
```
```

---

### VALIDATOR_PROTOCOL.md Template

```markdown
# VALIDATOR_PROTOCOL.md

## Pre-Flight Checks (BLOCKING GATES)

Before auditing implementation:

- [ ] Task packet exists and complete (all 10 fields)
- [ ] Spec version in packet matches SPEC_CURRENT.md
- [ ] USER_SIGNATURE present and valid (not reused)
- [ ] STATUS in packet is consistent with Task Board
- [ ] BOOTSTRAP block present (shows work was started)
- [ ] TEST_PLAN section exists with commands
- [ ] VALIDATION section has outcomes recorded

**If ANY fail**: return packet to Orchestrator for fixes.

---

## Core Validation Steps

### Step 1: Spec Extraction
Extract every MUST/SHOULD from:
- DONE_MEANS in task packet
- SPEC_ANCHOR sections in spec
Create evidence mapping table:
| DONE_MEANS | SPEC_ANCHOR | Code Evidence | Test Proof |
| ... | ... | ... | ... |

### Step 2: Evidence Verification
For each requirement:
1. Locate file:line code evidence
2. Read the code; verify it implements requirement
3. Run test command; verify it PASSES
4. Verify test is removable (fails if code deleted)
5. **Missing evidence = FAIL**

### Step 3: Hygiene Gates
Run validators:
```
just validator-scan              → 0 forbidden patterns
just validator-git-hygiene       → 0 artifacts committed
just validator-error-codes       → typed errors, codes documented
just validator-{custom}          → domain-specific checks
```
**Any FAIL = overall FAIL**

### Step 4: Test Verification
- Verify TEST_PLAN was executed (results in VALIDATION block)
- Check coverage >= 80% (or documented waiver)
- Verify at least one removal-check test (code removal breaks test)

### Step 5: Security & Architecture Audits
- LLM boundary: all calls via /src/backend/llm/ (CX-101)
- Storage boundary: no direct DB outside /storage/ (CX-102)
- Error handling: typed errors with codes (CX-103)
- Logging: trace IDs in mutations, no secrets (CX-104)
- Traceability: job_id, request_id correlation

### Validator Evidence Aggregation Format

When **multiple validators** run on a single packet, aggregate findings into a single VALIDATION REPORT:

**Aggregation Strategy**:

```markdown
## VALIDATION REPORT - WP-{id}

### Executive Summary
- Verdict: PASS | FAIL
- Validators run: 6 (pre-flight, spec-extraction, hygiene, custom-audits: 3)
- Failures: 0 | {N} critical, {M} warnings
- Time: 5m 32s

### Per-Validator Results

#### Validator 1: Pre-Flight Checks
Status: ✅ PASS
- Packet completeness: ✅ (all 10 fields)
- Spec match: ✅ (v1.0 anchors valid)
- USER_SIGNATURE: ✅ (alice_25121512345, not reused)
- Bootstrap present: ✅

#### Validator 2: Evidence Extraction & Verification
Status: ✅ PASS
- DONE_MEANS extracted: 5
- Evidence mapping: 5/5 complete (100%)
- Test proof collected: 5/5
- Code removability verified: 5/5 ✅

#### Validator 3: Hygiene Gates
Status: ⚠️  WARNING (non-blocking)
- Forbidden patterns: ✅ (0 found)
- Git cleanliness: ✅
- Coverage: 78% (below 80%, but documented waiver CX-EXC-001)
- Codex violations: ✅ (0)

#### Validator 4: Storage DAL Audit (custom)
Status: ✅ PASS
- Trait isolation: ✅
- SQL portability check: ✅ (no SQLite-specific syntax outside storage/)
- Migration numbering: ✅ (migration_001.sql, migration_002.sql)
- Schema immutability: ✅

#### Validator 5: Security Audit (custom)
Status: ✅ PASS
- RCE guardrails: ✅
- Input validation: ✅
- Secret detection: ✅ (0 secrets found)
- Dependency audit: ✅ (0 critical vulnerabilities)

#### Validator 6: Git Hygiene (custom)
Status: ✅ PASS
- .gitignore coverage: ✅
- Artifacts committed: ✅ (0)
- Commit message format: ✅

### Aggregation Logic

**Verdict is PASS if and only if**:
- Pre-flight: PASS
- Evidence extraction: PASS
- All custom auditors: PASS
- Hygiene gates: PASS or WARNING (FAIL allowed with documented waiver)
- Any FAIL from any validator → overall FAIL

**Verdict is FAIL if**:
- Any required validator returns FAIL
- Evidence gaps found (pre-flight or extraction)
- Codex violation (non-waiverable)
- Critical security issue found

### Conflict Resolution

When validators disagree:

| Scenario | Resolution |
|----------|-----------|
| Storage auditor says ✅ PASS; Security auditor says ❌ FAIL | Overall = FAIL; rework required |
| Hygiene auditor says WARNING (low coverage); Pre-flight says PASS | Overall = PASS IF waiver approved |
| Evidence extraction says FAIL (missing proof); Coder says "it's there" | Overall = FAIL; require evidence resubmission |

### Validator Report Template

```markdown
VALIDATION REPORT - WP-{id}

Verdict: PASS | FAIL

Scope:
- Packet: docs/task_packets/WP-{id}.md
- Spec: {SPEC_CURRENT version}
- Coders: {names}

Validators Executed (in order):
1. pre-flight-check (built-in)
2. spec-extraction (built-in)
3. hygiene-gates (built-in)
4. validator-dal-audit (custom)
5. validator-security (custom)

Findings Summary:
- Total validators: 5
- PASS: 5
- FAIL: 0
- WARNING: 0
- Blockers: None

Test Results:
- TEST_PLAN executed: {N} commands
- All PASS: ✅
- Coverage: {X}% (threshold: 80%)
- Removability checks: {Y}/Y PASS

Risks/Gaps:
- None

Approval:
- Validator: {name}
- Timestamp: {ISO 8601}
- Signature: {validator_signature} (optional)
```

---

## Verdict (Binary: PASS or FAIL)

### PASS Criteria (ALL must be true):
- Evidence mapping complete (every DONE_MEANS has file:line + test)
- All tests PASS
- Hygiene clean (no forbidden patterns)
- Coverage >= 80%
- Codex compliance verified
- Custom audits satisfied

**Action**: Append validation report to packet; update STATUS → Done; update Task Board to Done (VALIDATED)

### FAIL Criteria (ANY is true):
- Missing evidence for requirement
- Test fails or missing
- Forbidden pattern found
- Codex violation detected
- Coverage < 80% without waiver

**Action**: Document gaps; list violations; return packet to Orchestrator/Coder for rework

---

## Waiver Policy

Waivers are ALLOWED (with approval) for:
- Coverage <80% (if business-justified + documented)
- Test gap in Phase 1 (Phase 2 backlog)
- Deferred design debt (explicit TODO(PROJECT-####) + issue link)

Waivers are NOT ALLOWED for:
- Spec regression (requirement not met)
- Evidence gaps (must have file:line proof)
- Codex violations (hard invariants)
- Security issues (RCE, plaintext secrets)
```

---

## Validator Script Implementations

### Sample: pre-work-check.mjs

```javascript
// scripts/validation/pre-work-check.mjs
// Gate 0: Ensure packet is complete before handoff

import fs from 'fs';
import path from 'path';

const REQUIRED_FIELDS = [
  'TASK_ID',
  'STATUS',
  'SPEC_ANCHOR',
  'What',
  'Why',
  'IN_SCOPE_PATHS',
  'OUT_OF_SCOPE',
  'DONE_MEANS',
  'TEST_PLAN',
  'BOOTSTRAP',
  'ROLLBACK_HINT',
  'VALIDATION',
  'Signature & Enrichment Log'
];

const PLACEHOLDERS = [
  /\{field\w+\}/g,
  /TODO\(TBD\)/g,
  /FIXME/g,
  /placeholder/gi,
  /\{.*?\}/g  // Any {something}
];

export async function validate(wpId, projectRoot) {
  const packetPath = path.join(projectRoot, 'docs', 'task_packets', `${wpId}.md`);

  if (!fs.existsSync(packetPath)) {
    console.error(`❌ Packet not found: ${packetPath}`);
    process.exit(1);
  }

  const content = fs.readFileSync(packetPath, 'utf-8');

  // Check all required fields present
  for (const field of REQUIRED_FIELDS) {
    if (!content.includes(`## ${field}`) && !content.includes(`- ${field}`)) {
      console.error(`❌ Missing required field: ${field}`);
      process.exit(1);
    }
  }

  // Check for placeholders
  for (const placeholder of PLACEHOLDERS) {
    const matches = content.match(placeholder);
    if (matches) {
      console.error(`❌ Found placeholder text: ${matches[0]}`);
      process.exit(1);
    }
  }

  console.log(`✅ Pre-work check passed: ${wpId}`);
  process.exit(0);
}

const wpId = process.argv[2];
const projectRoot = process.cwd();
validate(wpId, projectRoot);
```

### Sample: validator-scan.mjs

```javascript
// scripts/validation/validator-scan.mjs
// Scan for forbidden patterns in production code

import { execSync } from 'child_process';

const SCAN_PATHS = [
  'src/',
  'app/src/'
];

const FORBIDDEN_PATTERNS = [
  { pattern: 'unwrap()', reason: 'Panic in production; must handle error' },
  { pattern: 'expect(', reason: 'Panic in production; must handle error' },
  { pattern: 'todo!()', reason: 'Incomplete code; must implement or defer' },
  { pattern: 'unimplemented!()', reason: 'Incomplete code' },
  { pattern: 'dbg!(', reason: 'Debug macro; must remove' },
  { pattern: 'println!(', reason: 'Use logging framework; no stdout' },
  { pattern: 'eprintln!(', reason: 'Use logging framework; no stderr' }
];

const PLACEHOLDER_PATTERNS = [
  'Mock', 'Stub', 'placeholder', 'hollow', /\{.*?\}/
];

let violations = 0;

for (const scanPath of SCAN_PATHS) {
  // Exclude tests
  const excludeTests = `--exclude-dir=tests --exclude-dir=test`;

  for (const { pattern, reason } of FORBIDDEN_PATTERNS) {
    try {
      const cmd = `rg "${pattern}" ${scanPath} ${excludeTests}`;
      const result = execSync(cmd, { encoding: 'utf-8' });

      if (result.trim()) {
        console.error(`❌ FORBIDDEN: ${pattern}`);
        console.error(`   Reason: ${reason}`);
        console.error(result);
        violations++;
      }
    } catch (e) {
      // No matches (expected)
    }
  }

  for (const placeholder of PLACEHOLDER_PATTERNS) {
    try {
      const cmd = `rg "${placeholder}" ${scanPath} ${excludeTests}`;
      const result = execSync(cmd, { encoding: 'utf-8' });

      if (result.trim()) {
        console.error(`⚠️  PLACEHOLDER: ${placeholder}`);
        console.error(result);
        violations++;
      }
    } catch (e) {
      // No matches
    }
  }
}

if (violations > 0) {
  console.error(`\n❌ Found ${violations} violations`);
  process.exit(1);
} else {
  console.log('✅ validator-scan passed');
  process.exit(0);
}
```

### Template: Custom Validator Pattern

```javascript
// scripts/validation/validator-{your-concern}.mjs
// Template for project-specific validators

export async function validate(wpId, projectRoot) {
  // 1. Read task packet
  const packetPath = `${projectRoot}/docs/task_packets/${wpId}.md`;
  const packetContent = readFile(packetPath);

  // 2. Parse IN_SCOPE_PATHS from packet
  const inScopePaths = extractField(packetContent, 'IN_SCOPE_PATHS');

  // 3. Apply domain-specific checks
  // Example: For storage audits
  for (const filePath of inScopePaths) {
    const content = readFile(`${projectRoot}/${filePath}`);

    // Check 1: No direct DB access
    if (content.includes('sqlx::query') && !filePath.includes('storage/')) {
      console.error(`❌ CX-DBP-VAL-010: Direct DB access in ${filePath}`);
      process.exit(1);
    }

    // Check 2: SQL portability
    if (content.includes('strftime(') || content.includes('?1')) {
      console.error(`❌ CX-DBP-VAL-011: SQLite-only SQL in ${filePath}`);
      process.exit(1);
    }
  }

  // 4. Return verdict
  console.log(`✅ validator-{concern} passed`);
  process.exit(0);
}

const wpId = process.argv[2];
validate(wpId, process.cwd());
```

---

## Troubleshooting & Common Failures

### "Packet fails pre-work check"

**Symptoms**: `just pre-work WP-1` returns exit 1

**Causes**:
1. Missing required field (check all 10 fields present)
2. Placeholder text in packet (`{field}`, `TODO(TBD)`)
3. Field value is empty or just comment marker

**Fix**:
1. Read pre-work-check output: identifies which field is missing
2. Open packet; add field with concrete value
3. Remove all `{placeholders}` and replace with real values
4. Re-run `just pre-work WP-1` → exit 0

### "Validator says evidence missing"

**Symptoms**: Validator report says "Missing evidence for DONE_MEANS item 3"

**Causes**:
1. Code not implemented (DONE_MEANS not actually done)
2. Test command not executed or failed
3. file:line reference doesn't actually contain the code

**Fix**:
1. Verify code exists at file:line location (open file, check line numbers)
2. Verify test command runs without error: `cargo test ...`
3. If code missing: implement it
4. If test fails: debug test; fix code
5. Record new results in VALIDATION block
6. Re-run validator

### "Codex check fails at commit"

**Symptoms**: `git commit` blocked with "CX-101 violation: LLM call outside /src/backend/llm/"

**Causes**:
1. Code calls LLM API directly (not via module)
2. Code uses `println!` or `dbg!` (logging instead of logging framework)
3. Code has `unwrap()` or `expect(` in production

**Fix**:
1. Read codex rule (e.g., CX-101) for what's forbidden and why
2. Refactor code to comply:
   - LLM calls: move to `/src/backend/llm/client.rs`
   - Logging: use logging framework instead of println
   - Error handling: use typed errors instead of unwrap
3. Stage changes: `git add .`
4. Try commit again: `git commit ...`

### "Scope creep: features not in DONE_MEANS"

**Symptoms**: Coder implements extra features not in task packet

**Causes**:
1. Coder assumed "while we're here, let's also..."
2. Spec ambiguity; coder inferred additional requirements
3. Scope adequacy check not done properly

**Fix**:
1. **Before implementation**: Coder must do Scope Adequacy Check (identify all affected files, validate boundaries)
2. **During implementation**: Stick to DONE_MEANS only; nothing more
3. **If new feature discovered**: Create WP-{id}-v2 packet with fresh signature
4. **Validator catches**: Will FAIL if code doesn't match DONE_MEANS (removable test fails)

### "Signature audit shows reuse"

**Symptoms**: `grep -r "alice_25121510000" . | wc -l` returns 2+ (signature used twice)

**Causes**:
1. Signature was issued for Phase 0 enrichment
2. Same signature reused for Phase 1 packet (violation)
3. Typo in signature log (same signature recorded twice)

**Fix**:
1. Each signature is ONE-TIME USE ONLY
2. If reuse detected: STOP work; escalate to user
3. Request fresh signature (different timestamp): `alice_25121510100`
4. Log new signature in SIGNATURE_AUDIT.md
5. Re-run spec regression check

### "Task Board out of sync with packets"

**Symptoms**: Packet STATUS = "Done" but Task Board shows "Ready-for-Dev"

**Causes**:
1. Packet STATUS changed but Operator-visible Task Board on `main` was not status-synced
2. Task Board was edited directly without matching packet reality
3. Multi-branch/worktree drift (local board differs from `main`)

**Fix**:
1. Source-of-truth rule: packet STATUS is authoritative.
2. Validator performs a docs-only status-sync commit on `main`:
   - Read current packet STATUS
   - Update Task Board entry on `main` to match
   - Verify with: `git show main:docs/TASK_BOARD.md | rg "WP-1-Feature"` matches packet reality

### "Phase gate blocked; can't close Phase 1"

**Symptoms**: `just validator-phase-gate Phase-1` returns exit 1

**Causes**:
1. At least one "Phase 1 Closure Gates (BLOCKING)" WP is not Done+VALIDATED
2. Spec regression check failed (anchor missing)
3. Dependency unresolved (downstream WP blocker still In-Progress)

**Fix**:
1. Run command with verbose output: see which WP is blocking
2. Go to blocking WP: move to Done and validate
3. Check spec regression: `just validator-spec-regression` should PASS
4. Check dependencies: all upstream WPs must be VALIDATED
5. Once all clear: `just validator-phase-gate Phase-1` returns exit 0
6. Can now close phase: `git tag phase-1-complete && git push origin phase-1-complete`

---

## Integration with Existing Projects

If you have an **existing codebase** (not starting from scratch), use this retrofit approach:

### Week 1: Minimal Setup (Don't Block Current Work)
1. Create `Master_Spec_v0.1.md` describing current state (big picture, 10-15 sections)
2. Create `docs/SPEC_CURRENT.md` pointer
3. Create `docs/SIGNATURE_AUDIT.md` (empty)
4. Create `docs/TASK_BOARD.md` (list current in-flight work)
5. Create `scripts/create-task-packet.mjs` (simple generator)

**Do NOT force everyone to use this workflow yet.** Just set up infrastructure.

### Week 2: Optional Adoption (Volunteer Features)
1. Volunteer a low-risk feature (e.g., documentation, small bug fix)
2. Create task packet for it (using template)
3. Run full workflow: pre-work → code → validate → merge
4. Measure: did workflow catch issues? Did it help?

### Week 3: Phased Rollout (Team Agreement)
- If Week 2 successful: make workflow mandatory for NEW features only
- Existing work continues as-is
- Gradual adoption: new feature per week uses workflow

### Month 2: Enforcement
- All NEW feature work uses workflow
- All bug fixes use workflow
- Refactors optional (can use or skip)

### Month 3: Full Adoption
- All work uses workflow
- Agents can operate semi-autonomously
- Governance rules enforced at pre-commit

---

## Testing the Workflow Infrastructure

Before using on real work, validate each piece works:

### Test 1: Packet Creation
```bash
just create-task-packet WP-0-Test
# Verify: docs/task_packets/WP-0-Test.md created with template
```

### Test 2: Pre-Work Gate
```bash
just pre-work WP-0-Test
# Should FAIL (has placeholders)
# Edit packet: remove {field} values
just pre-work WP-0-Test
# Should PASS (all fields concrete, no placeholders)
```

### Test 3: Spec Regression
```bash
just validator-spec-regression
# Should PASS (spec file exists, required anchors present)
```

### Test 4: Validator Scan
```bash
just validator-scan
# Should PASS (no forbidden patterns in src/)
# To test: add unwrap() to src/main.rs, re-run → FAIL
```

### Test 5: Pre-Commit Hook
```bash
# Add println!() to src/lib.rs
git add src/lib.rs
git commit -m "test"
# Should FAIL (pre-commit hook rejects println)
# Remove println(), re-commit → PASS
```

### Test 6: Full Workflow (Dry Run)
1. Create test packet for small feature
2. Implement it
3. Run `just post-work WP-{id}`
4. Validator audits
5. Merge

If all tests pass, infrastructure is ready.

---

---

## Tool Agent: Mechanical Workflow Orchestration & Token Efficiency

The Tool Agent executes mechanical operations (tests, scans, format checks, grep, curl, etc.) **as typed workflow invocations**, not raw shell commands. This enables **token efficiency** and **reusable tool pipelines** across all task packets.

### Problem Without Typed Workflows

**Token Bloat from Embedded Outputs:**
```
Prompt 1: "Run these 5 test commands" (1KB)
Tool outputs: 50KB of test logs → embedded in next prompt
Prompt 2: "Here's test output... run next commands" (51KB)
Tool outputs: 40KB of build logs → embedded in next prompt
Prompt 3: "Here's build output... run grep" (41KB)
...
Total tokens: ~150KB of raw I/O → ~750K tokens for prompting (4:1 expansion)
```

**Result**: For a 10-command task, Coder/Validator prompts bloat to 50KB+. Context rot, token waste, repeated parsing.

### Solution: Artifact Handle Discipline

**With Typed Workflows + Handles:**
```
Prompt 1: "Invoke workflow 'test_and_build_WP' with inputs {module: api, tests: auth_integration}" (1KB)

Tool Agent executes:
  node_1: cargo test → outputs handle "test_run_001" (log stored separately)
  node_2: cargo build → outputs handle "build_001"
  node_3: grep search → outputs handle "grep_001"

Response: {handles: [test_run_001, build_001, grep_001], status: SUCCESS} (0.5KB)

Prompt 2: "Validate using artifact handles; test_run_001 = {status: PASSED, count: 8}" (1KB)
(Validator fetches handles on-demand, not embedded in prompt)

Total tokens: ~2.5KB actual prompts → ~12K tokens
Reduction: 97.3% token savings vs embedded logs
```

### Core Pattern: Workflow Spec Instead of Shell Commands

**Old BOOTSTRAP (text commands):**
```
RUN_COMMANDS:
- cargo test -p api auth_integration
- cargo build --manifest-path src/Cargo.toml
- grep "bcrypt" src/services/jwt.rs
```

**New BOOTSTRAP (typed workflow):**
```json
{
  "tooling": {
    "workflow": "test_and_build_WP",
    "template": "api_test_pack",
    "inputs": {
      "package": "api",
      "test_target": "auth_integration",
      "build_manifest": "src/Cargo.toml",
      "search_terms": ["bcrypt"]
    },
    "outputs": {
      "test_results": {"handle_pattern": "test_run_*", "required": true},
      "build_log": {"handle_pattern": "build_*", "required": true},
      "grep_results": {"handle_pattern": "grep_*", "required": false}
    },
    "constraints": {
      "timeout_sec": 600,
      "output_size_limit_mb": 100,
      "allowed_tools": ["cargo", "grep", "rg"]
    }
  }
}
```

**Benefits:**
1. **Reusable**: same "api_test_pack" template used in 100 packets, zero duplication
2. **Deterministic**: typed inputs/outputs, no ambiguity
3. **Token-efficient**: outputs are handles, not embedded content
4. **Debuggable**: Tool Agent can replay, replay-from-pin, etc.
5. **Auditable**: full workflow spec in packet, reproducible

### Tool Agent Capability Catalog Pattern

**Define once. Reuse 100 times.** A formal registry of all available workflows ensures:
- No tool duplication across packets
- Consistent versioning and capability metadata
- Fast capability lookup (which workflows can solve this problem?)
- Lineage tracking (which packets depend on which workflows?)

#### Capability Catalog Format

Create `docs/TOOL_AGENT_CATALOG.md`:

```markdown
# Tool Agent Capability Catalog v1.0

## Workflow Families

### Testing & Validation Family
**Purpose**: Execute tests, coverage checks, linting, format validation

| Workflow ID | Name | Inputs | Outputs | Error Workflow | Version | Status |
|---|---|---|---|---|---|---|
| api_test_pack | API Test & Build | package, test_target, build_manifest, search_terms | test_results, build_log, grep_results | emit_diagnostic_bundle | 1.0 | Stable |
| backend_validation_pack | Backend Validator | repo_path, lint_rules, test_threshold | lint_report, test_report, coverage_report | notify_failures | 1.0 | Stable |
| validator_scan_pack | Forbidden Pattern Scanner | scan_paths, forbidden_patterns, severity_level | scan_results, violation_details | escalate_critical | 1.2 | Stable |

### Data Processing Family
**Purpose**: Ingest, normalize, transform, export data

| Workflow ID | Name | Inputs | Outputs | Error Workflow | Version | Status |
|---|---|---|---|---|---|---|
| ingest_pack | Ingest & Normalize | source_type, entity_format, rules | entity_handle, index_handle, enrichment_handle | fallback_ingestion | 1.0 | Stable |
| analytics_pack | Extract-Compute-Visualize | extraction_filter, computation_rules, viz_type | extraction_handle, computation_handle, viz_handle, export_handle | diagnostic_analytics | 1.0 | Stable |

### Decision & Approval Family
**Purpose**: Detect conditions, suggest actions, gate approvals

| Workflow ID | Name | Inputs | Outputs | Error Workflow | Version | Status |
|---|---|---|---|---|---|---|
| decision_pack | Detect-Suggest-Gate | detection_rules, suggestion_algo, gate_criteria | detection_handle, suggestion_handle, gate_verdict, execution_handle | escalate_gate_failure | 1.0 | Stable |

### Debugging & Recovery Family
**Purpose**: Diagnose failures, bundle artifacts, propose repairs

| Workflow ID | Name | Inputs | Outputs | Error Workflow | Version | Status |
|---|---|---|---|---|---|---|
| diagnostic_pack | Observe-Diagnose-Bundle | symptom_filters, diagnostic_rules, repair_kb | observation_handle, diagnosis_handle, bundle_handle, repair_handle | escalate_inconclusive | 1.0 | Stable |

## Capability Inheritance Model

When a workflow calls another workflow (subworkflow), capabilities cascade:

```
Parent Packet: WP-1-Storage-Layer (RISK_TIER: HIGH, needs security audit)
  ├─ Invokes: backend_validation_pack
  │   Inherited capabilities:
  │   - Test execution (restricted to IN_SCOPE_PATHS)
  │   - Forbidden pattern scanning (inherits severity level)
  │   - Lint rules (inherits codex rules)
  │
  └─ Validator applies MOST_RESTRICTIVE rule:
      - If parent = HIGH risk, child = MEDIUM risk → child inherits HIGH
      - If parent forbids "unwrap()", child cannot use it either
      - Capability constraint propagates down the call tree
```

## Workflow Versioning

- **v1.0**: Initial stable release (feature complete)
- **v1.1+**: Backwards-compatible enhancements (new optional input, new optional output)
- **v2.0+**: Breaking changes (input/output schema change, required new capability)

**Migration**: When workflow version changes:
```bash
# Packets referencing workflow_v1.0 continue working
# Orchestrator can optionally upgrade to v2.0 (requires re-validation)
grep -r "api_test_pack" docs/task_packets/ | wc -l  # Find all users

# Create deprecation notice
echo "api_test_pack@1.0 deprecated as of Phase 3; v2.0 available"

# Migrate packets at phase boundary (not mid-implementation)
```

## Workflow Dependency Graph

Workflows can depend on other workflows:

```
validator_scan_pack (depends on)
  └─ backend_validation_pack
     └─ api_test_pack
```

When validator_scan_pack runs, it transitively gains capabilities from its dependencies.

---

### Subworkflow Templates (Reusable Tool Pipelines)

Instead of repeating commands in every packet, define templates once and reuse:

**Template 1: api_test_pack**
```
test_and_build_WP:
  - Input: package, test_target, build_manifest
  - Nodes:
    - run_cargo_tests(package, test_target) → handle "test_run_*"
    - run_cargo_build(build_manifest) → handle "build_*"
    - run_clippy(package) → handle "clippy_*"
    - check_coverage(package, threshold: 80) → handle "coverage_*"
  - Error workflow: if any node fails, emit diagnostic bundle + suggest fix
  - Output: all handles + summary (PASSED/FAILED/NEEDS_REVIEW)
```

**Usage in Packet 1:**
```json
{"workflow": "test_and_build_WP", "inputs": {package: "api", test_target: "auth"}}
```

**Usage in Packet 2:**
```json
{"workflow": "test_and_build_WP", "inputs": {package: "api", test_target: "storage"}}
```

**Packet 3-100:** same template, different inputs. Zero command duplication. Every execution tracked with handles.

**Template 2: backend_validation_pack**
```
backend_full_validation_WP:
  - Nodes:
    - lint_check (cargo clippy) → handle "clippy_*"
    - format_check (cargo fmt --check) → handle "fmt_*"
    - deny_check (cargo deny) → handle "deny_*"
    - test_suite (all tests) → handle "test_*"
  - Output: all handles + summary
  - Reusable across: Rust backend, system code, utilities
```

**Template 3: validator_scan_pack**
```
validator_comprehensive_scan_WP:
  - Nodes:
    - forbidden_patterns_scan → handle "scan_forbidden_*"
    - error_code_audit → handle "audit_errors_*"
    - coverage_check → handle "coverage_report_*"
    - security_scan → handle "security_*"
  - Output: all handles + PASS/FAIL verdict
  - Used in: every validator audit
```

### Error Workflows (Token-Efficient Failure Handling)

**Without error workflows** (token wasteful):
- Tool fails: raw error message sent to Coder
- Coder parses 50KB error log
- Tries to understand failure context
- Prompts inflate with raw debugging info

**With error workflows** (token efficient):
```
on_failure handler:
  - Capture error + context
  - Generate Diagnostic struct: {error_code, location, possible_causes}
  - Generate Debug Bundle: {logs_handle, context_handle, env_handle}
  - Generate Repair Suggestion: {suggestion_WP, estimated_fix_time}

Output to Coder: {diagnostic: {...}, debug_bundle_handle: "debug_001", suggestion: {...}} (1KB)
```

**Result**: Coder gets structured error + handle to debug bundle, not raw logs. Token savings: 98%.

### Evidence Validation via Handles

**Old VALIDATION (embedded logs):**
```
Command: cargo test -p api auth_integration
Result: PASSED (8/8 cases)
  ✓ login_valid_credentials
  ✓ login_invalid_password
  ...
[output repeated, taking 5KB in packet]
```

**New VALIDATION (handles):**
```
VALIDATION:
  test_suite_execution:
    - workflow_id: "exec_12345"
    - workflow_spec: "api_test_pack"
    - test_results_handle: "test_run_001"
    - test_results_summary: {status: PASSED, passed: 8, failed: 0, skipped: 0}
    - build_log_handle: "build_001"
    - build_status: SUCCESS

  execution_proof:
    - command_executed: "cargo test -p api auth_integration" (sha256: abc123)
    - output_hash: "def456" (contents verified against test_run_001 handle)
    - timestamp: 2025-12-25T10:00:00Z
    - reproducible: true (can replay from handle)

  validator_checks:
    - evidence_complete: PASS (all handles present)
    - tests_executable: PASS (can replay test_run_001)
    - output_size: PASS (under limit)
```

**Benefit**: Validator audits via handles, fetches artifacts on-demand, prompt stays tiny (0.5KB).

### Token Efficiency: The Math

**For a Medium Task (8 test commands + 2 builds):**

| Approach | Command I/O | Prompt Size | Total Tokens | Notes |
|----------|------------|------------|--------------|-------|
| Raw shell (embedded) | 300KB | 150KB | 750K | Every output embedded in next prompt |
| Artifact handles | 300KB | 1.5KB | 7.5K | Outputs are handles, fetched on-demand |
| **Savings** | — | **99%** | **99%** | 100x token reduction |

**For Large Task (30 validator audits + 10 tests):**

| Approach | Total Tokens |
|----------|--------------|
| Raw shell | 2M+ |
| Artifact handles | 20K |
| **Savings** | **99%** |

### Implementation: From Raw Shell to Typed Workflows

**Phase 0: Define Workflow Specs (Days 1-2)**
1. Create `WorkflowSpec` schema (JSON typed format)
2. Define 5 base templates: test_pack, build_pack, validator_pack, formatter_pack, security_pack
3. Update BOOTSTRAP section to reference workflows instead of listing commands

**Phase 1: Implement Artifact Handles (Days 3-4)**
1. Extend Tool Agent to output handles instead of embedding content
2. Create handle storage: `artifacts/{exec_id}/{handle_id}/content`
3. Update VALIDATION section to store handles + summary (not full logs)

**Phase 2: Error Workflows (Days 5-6)**
1. Define error_workflow node: captures failure → emits diagnostic + bundle handle
2. Integrate into all templates: on_failure triggers error workflow
3. Coder receives structured error (diagnostic struct + handle)

**Phase 3: Pinned Fixtures (Days 7-8)**
1. Allow "pin output" on any workflow node
2. Store pinned handles in packet
3. Validator can replay from pinned (deterministic, no re-execution)

**Phase 4: Cross-Domain Composition (Days 9-12)**
1. Define domain packs (IngestPack, ExportPack, API-TestPack)
2. Subworkflows can call subworkflows (call_workflow node)
3. Build complex pipelines by composing templates

### Template Definition Format

```yaml
# templates/api_test_pack.yaml
name: api_test_pack
description: "Test, build, lint for API modules"
version: "1.0"

inputs:
  package: string        # e.g., "api"
  test_target: string    # e.g., "auth_integration"
  build_manifest: string # e.g., "src/Cargo.toml"

outputs:
  test_results:
    type: artifact_handle
    pattern: "test_run_*"
    required: true
    validator: must contain {status, passed, failed, skipped}

  build_log:
    type: artifact_handle
    pattern: "build_*"
    required: true

  clippy_report:
    type: artifact_handle
    pattern: "clippy_*"
    required: false

workflow:
  - node: run_tests
    tool: cargo
    args: ["test", "-p", "{package}", "--test", "{test_target}"]
    outputs: {handle: "test_run_{timestamp}"}
    on_failure: trigger error_workflow

  - node: run_build
    tool: cargo
    args: ["build", "--manifest-path", "{build_manifest}"]
    outputs: {handle: "build_{timestamp}"}
    on_failure: trigger error_workflow

  - node: run_clippy
    tool: cargo
    args: ["clippy", "-p", "{package}"]
    outputs: {handle: "clippy_{timestamp}"}
    on_failure: log_warning (non-blocking)

constraints:
  timeout_sec: 600
  output_size_limit_mb: 500
  allowed_tools: [cargo, grep, rg, curl]

error_workflow: emit_diagnostic_bundle
```

### How BOOTSTRAP + Workflow Spec Reduces Prompt Size

**BOOTSTRAP with workflow template:**
```json
{
  "BOOTSTRAP": {
    "FILES_TO_OPEN": ["src/api/handlers/auth.rs", "tests/integration/auth_test.rs"],
    "SEARCH_TERMS": ["LoginRequest", "validate_jwt", "Bearer"],
    "WORKFLOW": {
      "template": "api_test_pack",
      "inputs": {package: "api", test_target: "auth_integration"}
    },
    "WORKFLOW_VERSION": "1.0",
    "EXPECTED_OUTPUTS": ["test_results", "build_log"],
    "RISK_MAP": [
      {failure: "Test fails", impact: "logic error", mitigation: "review test code"},
      {failure: "Build fails", impact: "compilation error", mitigation: "fix syntax"}
    ]
  }
}
```

**Coder receives:**
- Files to read (concrete paths, no bloat)
- Search terms (keywords, not full code)
- Workflow spec (1 line, fully reusable)
- Risk map (structured, easy to parse)

**Coder invokes workflow:**
```
Tool Agent: invoke api_test_pack(package: "api", test_target: "auth_integration")
Returns: {test_results: handle_001, build_log: handle_002, status: PASSED}
```

**Coder updates VALIDATION:**
```
VALIDATION:
  workflow_execution: api_test_pack
  outputs: {test_results: handle_001, build_log: handle_002}
  status: PASSED
```

**Validator audits via handles:**
- Fetch handle_001 → verify test counts, pass/fail
- Fetch handle_002 → verify build succeeded
- **No embedding of logs, no context bloat**

### Token Budget Tracking

Add to your project's CI/CD:

```bash
# Measure prompt sizes before/after token optimization
echo "Prompt size (raw shell mode): $(wc -c < prompt_before.txt)"     # Expected: 50-100KB
echo "Prompt size (typed workflows): $(wc -c < prompt_after.txt)"     # Expected: 1-5KB
echo "Token reduction: $(( (1 - $(wc -c < prompt_after.txt) / $(wc -c < prompt_before.txt)) * 100 ))%"

# Expected: 95-99% reduction for tool-heavy tasks
```

#### Token Budget Per-Packet Tracking

Define **token budgets** per packet phase. Enforce in CI/CD:

```bash
# scripts/ci/token-budget-check.sh
# Measure actual token usage per packet; warn if exceed budget

WP_ID=$1
PHASE=$(echo $WP_ID | cut -d'-' -f1)  # WP-1-... → WP-1

# Define phase budgets (adjust based on typical prompts)
declare -A BUDGETS=(
  ["WP-0"]=5000      # Foundation: small scope
  ["WP-1"]=15000     # Phase 1: ~5 packets, each up to 3K tokens
  ["WP-2"]=20000     # Phase 2: larger features
  ["WP-3"]=25000     # Phase 3: complex features
)

BUDGET=${BUDGETS[$PHASE]:-20000}

# Estimate tokens: prompt_bytes / 4 (rough approximation)
# More precise: count tokens via API or tokenizer library
ACTUAL=$(cat docs/task_packets/$WP_ID.md | wc -c)
TOKENS=$((ACTUAL / 4))

echo "Packet $WP_ID:"
echo "  Budget: $BUDGET tokens"
echo "  Actual: $TOKENS tokens"
echo "  Status: $([ $TOKENS -le $BUDGET ] && echo '✅ PASS' || echo '❌ EXCEED')"

[ $TOKENS -le $BUDGET ] && exit 0 || exit 1
```

#### Phase-Level Token Budget Tracking

Track cumulative token usage across all packets in a phase:

```markdown
## Phase 1 Token Budget (Master Spec v1.0)

| WP-ID | Prompt Size | Tokens (est.) | Budget | Status | Savings |
|---|---|---|---|---|---|
| WP-1-Storage-Layer | 2.1KB | 525 | 3K | ✅ 83% under | 96% (vs 15KB embedded) |
| WP-1-AppState-Refactor | 1.8KB | 450 | 3K | ✅ 85% under | 95% |
| WP-1-Migration-Framework | 2.3KB | 575 | 3K | ✅ 81% under | 97% |
| WP-1-Dual-Backend-Tests | 2.5KB | 625 | 3K | ✅ 79% under | 96% |
| **Phase 1 Total** | **9.7KB** | **2,175 tokens** | **12K budget** | ✅ PASS | **96% avg savings** |

**Projection**:
- Phase 1 (4 packets): 2.2K tokens
- Phase 2 (6 packets est.): 3.3K tokens
- Phase 3 (8 packets est.): 4.4K tokens
- **Total project**: 10K tokens (vs 500K with embedded logs)
```

#### Evidence of Token Savings

Compare before/after for same packet:

```markdown
## Token Efficiency Evidence (WP-1-Storage-Layer)

### BEFORE: Embedded logs in prompt

Prompt content:
```
WP-1-Storage-Layer
Implement storage trait...
[Full 50KB test output embedded]
[Full 40KB build logs embedded]
[Full 15KB grep results embedded]
...
```
**Total prompt**: 145KB → ~725K tokens

Metadata: cargo test output = 50 test cases × 1000 bytes = 50KB
          cargo build output = 40KB
          ripgrep results = 15KB

### AFTER: Artifact handles only

Prompt content:
```json
{
  "WP_ID": "WP-1-Storage-Layer",
  "DONE_MEANS": 5,
  "TEST_PLAN": 3,
  "BOOTSTRAP": {
    "workflow": "backend_validation_pack",
    "outputs": {
      "test_results": "handle_test_001",
      "build_log": "handle_build_001",
      "grep_results": "handle_grep_001"
    }
  },
  "VALIDATION": {
    "test_results_summary": {status: PASSED, count: 8},
    "build_status": "SUCCESS"
  }
}
```
**Total prompt**: 2.1KB → ~10K tokens

**Savings**: 145KB → 2.1KB = 98.6% reduction
**Token savings**: 725K → 10K = 98.6% reduction (72.5x smaller)
```

#### Token Budget Alerts

In your CI/CD, fail the build if:
- Single packet exceeds phase budget by >10%
- Phase total exceeds cumulative budget by >10%
- Average savings < 90% vs embedded logs baseline

```yaml
# .github/workflows/token-budget-check.yml
name: Token Budget Check

on: [pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Check token budgets
        run: |
          for wp in docs/task_packets/WP-*.md; do
            bash scripts/ci/token-budget-check.sh "$(basename $wp .md)" || exit 1
          done
      - name: Report phase budgets
        if: always()
        run: bash scripts/ci/phase-token-report.sh
```

---

## Incident Recovery & Rollback Workflows

**When production breaks or a packet implementation causes issues**, use these systematic recovery patterns.

### Rapid Assessment (First 5 Minutes)

1. **Identify the breaking packet**:
   - Check recent commits: `git log --oneline | head -10 | grep WP-`
   - Find the WP_ID from commit message
   - Locate packet: `docs/task_packets/WP-{id}.md`

2. **Check ROLLBACK_HINT**:
   ```markdown
   ROLLBACK_HINT: git revert {commit-sha}
   # Or:
   ROLLBACK_HINT:
   - Drop migration: `sqlx migrate revert`
   - Restart service: `systemctl restart api`
   - Verify: `curl http://localhost:3000/health`
   ```

3. **Execute immediate rollback**:
   ```bash
   git revert {commit-sha} --no-edit
   git push origin main
   # Or manually execute ROLLBACK_HINT steps
   ```

### Post-Incident Packet (WP-INCIDENT-*)

Create a special task packet to investigate and document the failure:

```markdown
# Task Packet: WP-INCIDENT-2025-12-25-API-Crash

## What
Investigation of production API crash triggered by WP-1-Auth-v2 at 2025-12-25 14:30 UTC.
Rollback executed; root cause analysis required to prevent recurrence.

## Why
- Breaking change: OAuth token validation broke existing Bearer token format
- Impact: All API calls returned 401; services downstream degraded
- Discovery: Alerts fired at 14:31 UTC; rollback completed at 14:35 UTC
- Recurrence risk: HIGH (similar code pattern in WP-2-Session-Management)

## IN_SCOPE_PATHS
- src/api/middleware/auth.rs (token validation logic)
- tests/integration/auth_test.rs (missing test case)
- docs/SPEC_CURRENT.md (clarify token format guarantee)

## OUT_OF_SCOPE
- Other middleware (separate audit)
- UI login flows (QA responsibility)

## DONE_MEANS
1. Root cause identified and documented (design flaw or test gap?)
2. Proof: code review shows missing backward compatibility check
3. Proof: gap in TEST_PLAN (new format but no old format test)
4. Preventive: Add test case for old token format (removability: without test, no backward compat guarantee)
5. Update spec (A3.2.1) to clarify token versioning strategy
6. Post-mortem: 1-page summary with timeline and improvements

## TEST_PLAN
- `git diff {revert-commit}...HEAD src/api/middleware/auth.rs` (show what changed)
- `grep -r "Bearer\|OAuth" tests/ | grep -v ".git"` (identify related tests)
- Create new test case: `test_legacy_token_format_still_valid()`
- Run full test suite after fix: `cargo test -p api auth_integration`

## BOOTSTRAP
FILES_TO_OPEN:
- docs/task_packets/WP-1-Auth-v2.md (original packet)
- src/api/middleware/auth.rs (breaking code)
- tests/integration/auth_test.rs (test gaps)
- Incident alert log (timestamps, error messages)

SEARCH_TERMS:
- "Bearer", "token_format", "legacy", "backward"

RUN_COMMANDS:
- git log --oneline --grep="WP-1-Auth-v2"
- git show {commit-sha} -- src/api/middleware/auth.rs
- cargo test -p api auth -- --nocapture

## RISK_TIER
HIGH (post-incident analysis; changes to critical auth code)

## ROLLBACK_HINT
Investigation packet; no rollback. But if findings suggest another rollback needed:
- Revert both WP-1-Auth-v2 + WP-2-Session-Mgmt (dependent)
- Restart services
- Verify API health
```

### Incident Packet Validation (Special Protocol)

**Incident packets skip normal pre-work gate** (they're reactive), but **require enhanced auditing**:

1. **Root Cause Analysis Checklist**:
   - [ ] Timeline documented (when issue started, when detected, when rolled back)
   - [ ] Affected packets identified (which WPs depend on this code?)
   - [ ] Gap analysis (what was missing: test, design review, spec clarity?)
   - [ ] Proof of gap (show the test that should have caught it)

2. **Preventive Action Checklist**:
   - [ ] Code fix implemented
   - [ ] New test added (removability: failing without fix)
   - [ ] Spec updated or clarified
   - [ ] Similar code patterns audited (grep for risky patterns)
   - [ ] All dependent packets re-validated

3. **Validation Report for Incident**:
   ```markdown
   VALIDATION REPORT - WP-INCIDENT-2025-12-25-API-Crash

   Verdict: PASS (investigation complete; preventive measures confirmed)

   Root Cause: Token validation middleware didn't support legacy Bearer format

   Evidence:
   - Gap: no test case for backward compatibility (src/api/middleware/auth_test.rs)
   - Proof: test_legacy_token_format_still_valid() fails without fix
   - Fix applied: src/api/middleware/auth.rs:45-60 (format detection logic)
   - New test: tests/integration/auth_test.rs:102-120 (backward compat guarantee)

   Prevention:
   - Spec updated (A3.2.1): "API must maintain token format compatibility for 2 major versions"
   - Custom auditor added: validator-backward-compat.mjs (checks removability for all auth tests)
   - Similar patterns audited: grep "parse.*token" src/ (found 3 more locations, all fixed)

   Dependent packets affected: WP-2-Session-Management (required re-validation, passed)

   SLA impact:
   - Incident duration: 5 minutes (14:30-14:35 UTC)
   - Recovery time: 8 minutes (rollback + verification)
   - Post-mortem: 2 hours (root cause + fix + testing)
   ```

### Systematic Incident Prevention

Add these validators to catch issues **before** production:

1. **Backward Compatibility Auditor**:
   ```javascript
   // scripts/validation/validator-backward-compat.mjs
   // Ensure code changes maintain compatibility with previous version

   - Check: auth tests include legacy format tests
   - Check: API responses include version markers
   - Check: migrations are reversible
   - Check: config schema additions are optional (not breaking)
   ```

2. **Risk Assessment Auditor**:
   ```javascript
   // scripts/validation/validator-risk-assessment.mjs
   // Identify HIGH/CRITICAL risk changes

   - Middleware changes: HIGH risk (auth, logging, rate limiting)
   - Database schema: CRITICAL risk (migrations)
   - API contract: CRITICAL risk (breaking clients)
   - Third-party libs: HIGH risk (version bumps)
   ```

3. **Test Coverage Auditor (Stricter for HIGH Risk)**:
   ```javascript
   // scripts/validation/validator-test-coverage.mjs
   // Require >90% coverage for HIGH risk, >85% for MEDIUM

   - If RISK_TIER = HIGH: coverage_threshold = 90%
   - If RISK_TIER = MEDIUM: coverage_threshold = 80%
   - If RISK_TIER = LOW: coverage_threshold = 70%
   ```

### Incident Postmortem Template

After every incident, create a postmortem document:

```markdown
# Postmortem: WP-1-Auth-v2 Production Incident

## Timeline
- 2025-12-25 14:30:00 UTC: API crash detected; 401 errors on all endpoints
- 14:31:00 UTC: Alert fired; on-call engineer notified
- 14:32:00 UTC: Root cause identified (auth middleware)
- 14:35:00 UTC: Rollback deployed; API recovered
- 14:40:00 UTC: All health checks passing

## Root Cause
WP-1-Auth-v2 changed token format from JWT (old) to PASETO (new), breaking legacy clients.
Original TEST_PLAN only tested new format; no test for old format backward compatibility.

## Impact
- Duration: 5 minutes
- Services affected: API + 3 dependent microservices
- User-visible: Yes (web app showed 401 errors)
- Data loss: No

## Why Incident Occurred

| Aspect | Issue | Prevention |
|--------|-------|-----------|
| Code review | New format not flagged as breaking | Require 2+ reviewers for auth changes |
| Testing | Missing backward compatibility test | Custom validator: test_legacy_formats.mjs |
| Spec clarity | Token versioning strategy unclear | Update spec A3.2.1 with version guarantee |
| Pre-prod validation | No staging env test | Add RISK_TIER=CRITICAL → stage first |

## Immediate Actions (Completed)
- [x] Rollback WP-1-Auth-v2
- [x] Added backward compatibility test
- [x] Updated spec (A3.2.1)
- [x] Re-validated dependent packets

## Follow-Up Actions
- [ ] Increase test coverage threshold for auth (85% → 95%)
- [ ] Create custom validator for backward compatibility checks
- [ ] Require RISK_TIER=CRITICAL → staging + extended QA
- [ ] Review all token-related code patterns (WP-2-Session, WP-3-MFA)

## Lessons Learned
1. **Always test backward compatibility** for breaking changes
2. **Mark spec invariants** (token versioning is a guarantee, not implementation detail)
3. **Staging environment essential** for CRITICAL risk packets
4. **Test removability** should include "removing new feature" tests, not just "adding feature" tests

---

Prepared by: {Validator name}
Date: 2025-12-25 16:00:00 UTC
Signed: {signature}
```

---

## Making It Project-Agnostic
- Swap `SPEC_ANCHOR` format, file paths, and command runners to match your stack (e.g., `go test ./...`, `npm test`, `pytest`).
- Rename risk tiers or add tiers; adjust validator scripts to your invariants (e.g., API contract checks, schema checks).
- Replace LLM/LLM-boundary rules with your own hard invariants (e.g., no direct DB access, no network calls in handlers).
- Update forbidden patterns to reflect your language/tooling.

## Quick Start Checklist (new repo)
1) Create `docs/SPEC_CURRENT.md` pointing to your spec file.
2) Add packet template under `docs/task_packets/`.
3) Create `docs/TASK_BOARD.md` with columns and phase gates.
4) Add `docs/SIGNATURE_AUDIT.md` (empty table).
5) Add validator scripts under `scripts/validation/`; wire into `justfile`.
6) Add `scripts/create-task-packet.mjs` (or equivalent generator).
7) Add git hook (`scripts/hooks/pre-commit`) for invariants and linting.
8) Train agents on Orchestrator/Coder/Validator protocols and the signature pause.
9) Run `just pre-work WP-{id}` before handoff; block on any failure.
10) Require signature pause before enrichment or packet creation; log every signature.
