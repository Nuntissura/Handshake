# Audit V2: WP_COMMUNICATIONS Governance — Full Expansion

## METADATA
- AUDIT_ID: AUDIT-20260311-WP-COMMUNICATIONS-GOV-V2
- DATE_UTC: 2026-03-11
- AUDITOR: EXTERNAL (Claude Opus 4.6, evaluation-only)
- SUPERSEDES: AUDIT-20260311-WP-COMMUNICATIONS-GOV (v1)
- SCOPE_SUMMARY: Full re-audit of expanded governance: schemas, JSONL receipts, heartbeat/stale detection, workflow lanes, signature refactor, orchestrator gates, non-agentic role boundaries, cycle limits, bootstrap hardening.
- BRANCH: role_orchestrator
- FILES_NEW: ~14
- FILES_MODIFIED: ~12

---

## 1. WHAT CHANGED SINCE V1 AUDIT

The orchestrator addressed **5 of 6 moderate issues** and **5 of 7 low issues** from the v1 audit, plus added significant new governance dimensions:

| V1 Issue | Severity | Resolution |
|----------|----------|------------|
| No receipt format specification | MODERATE | **FIXED** — RECEIPTS.jsonl + WP_RECEIPT.schema.json |
| No RUNTIME_STATUS schema | MODERATE | **FIXED** — WP_RUNTIME_STATUS.schema.json (41 required fields) |
| Validator responsibilities undefined | MODERATE | **FIXED** — Event-driven wake-up, trigger vocabulary, explicit non-agentic boundary |
| No template existence guards | MODERATE | **FIXED** — `ensureSchemaFilesExist()` in wp-communications-lib.mjs |
| Silent regex parsing failures | MODERATE | **PARTIALLY FIXED** — Warnings logged for missing optional fields; defaults flagged |
| All-or-nothing field constraint | MODERATE | **KEPT BY DESIGN** — Strict; all 4 fields or none |
| `<pending>` placeholder leaks | LOW | **FIXED** — Controlled vocabularies with enum validation |
| No case-insensitive paths | LOW | **ACKNOWLEDGED** — normalize() used; acceptable on target OS |
| No orphan directory detection | LOW | **FIXED** — wp-communications-check.mjs now validates both directions |
| Bootstrap attention_required | LOW | **FIXED** — Defaults now context-appropriate |
| No rollback on artifact failure | LOW | **FIXED** — Non-destructive failure handling; explicit repair path |
| No timing guidance for updates | LOW | **FIXED** — Heartbeat interval + stale_after thresholds in packet |
| Implicit ensure-wp-communications | LOW | **FIXED** — Explicit `just ensure-wp-communications` command |

**New governance dimensions not in v1:**
- Workflow lanes (MANUAL_RELAY vs ORCHESTRATOR_MANAGED)
- Signature bundle (USER_SIGNATURE + WORKFLOW_LANE + EXECUTION_OWNER)
- Orchestrator gate sequence (refine -> sign -> prepare -> packet -> delegate)
- Bounded cycle limits (max coder/validator/relay cycles)
- Non-agentic role boundaries (Codex CX-218C)
- Drive-agnostic enforcement (CX-109)
- Heartbeat/stale detection with event-driven validator wake-up

---

## 2. ARCHITECTURE OVERVIEW

### 2.1 Three-Layer Model
```
LAYER 1: TASK PACKET (AUTHORITATIVE)
  Scope, status, verdict, assignment, cycle limits, heartbeat config
  Cannot be overridden by anything below

LAYER 2: STRUCTURED COORDINATION (SCHEMA-VALIDATED)
  RUNTIME_STATUS.json — liveness, phase, next-actor, validator trigger, sessions
  RECEIPTS.jsonl — immutable append-only audit ledger

LAYER 3: FREEFORM COLLABORATION (UNVALIDATED)
  THREAD.md — append-only discussion, no schema enforcement
```

### 2.2 Lifecycle Sequence
```
REFINEMENT -> APPROVAL -> SIGNATURE -> PREPARE -> PACKET -> DELEGATION -> ACTIVE DEV -> VALIDATION -> COMPLETION
     |            |           |           |          |           |              |              |
  record-     operator     record-    record-   create-    orchestrator   wp-heartbeat   wp-receipt-
  refinement  approves     signature  prepare   task-pkt   hands off      (continuous)    append
     |            |           |           |          |           |              |              |
  GATES.json  refinement  GATES.json  GATES.json  packet +    coder gets   STATUS.json   RECEIPTS.jsonl
              file        + SIG_AUDIT             comm-arts   packet path  + receipt
```

### 2.3 Workflow Lanes
```
MANUAL_RELAY          — Operator relays commands, hard gates require human intervention
ORCHESTRATOR_MANAGED  — Orchestrator coordinates sessions and steering autonomously
```
Both lanes use the same rich packet, runtime, and receipt artifacts. The only difference is who steers.

---

## 3. COMPONENT AUDIT

### 3.1 Schemas — SOUND

**WP_RUNTIME_STATUS.schema.json** — 41 required fields with enum validation for all controlled vocabularies (workflow_lane, execution_owner, agentic_mode, next_expected_actor, validator_trigger, runtime_status). Includes cycle counters, heartbeat timing, session tracking, and backup state.

**WP_RECEIPT.schema.json** — 11 required fields. Receipt kinds: ASSIGNMENT, STATUS, HEARTBEAT, HANDOFF, VALIDATOR_QUERY, VALIDATION_START, VALIDATION_STATUS_SYNC, STEERING, REPAIR. Actor roles: SYSTEM, OPERATOR, ORCHESTRATOR, CODER, VALIDATOR. RFC3339 UTC timestamps enforced.

**Assessment:** Comprehensive. The schemas close the biggest gap from v1. Both use versioned schema markers (`wp_runtime_status@1`, `wp_receipt@1`) for future migration.

### 3.2 Shared Library (wp-communications-lib.mjs) — SOUND

Centralizes all validation, constants, and helpers. Exports controlled vocabulary arrays, validation functions (`validateRuntimeStatus()`, `validateReceipt()`), file I/O helpers, and schema guards. Both validators return error arrays for detailed diagnostics.

**Assessment:** Good engineering. Single source of truth for vocabularies prevents drift between scripts.

### 3.3 Heartbeat System (wp-heartbeat.mjs) — SOUND

Updates RUNTIME_STATUS.json + appends HEARTBEAT receipt in a single atomic operation. Manages `active_role_sessions` array (add/replace session records). Derives `ready_for_validation` from validator trigger. Maps runtime_status to session state (working/waiting/blocked/completed/idle).

**Assessment:** Well-designed. Forces intentional state declaration (no silent defaults). Validator trigger vocabulary enables event-driven wake-up without polling.

### 3.4 Receipt Ledger (wp-receipt-append.mjs) — SOUND

Append-only JSONL. Auto-populates refs array (packet, runtime status, thread, receipts file). Validates each entry against schema before writing.

**Assessment:** Clean. The shift from markdown to JSONL was the right call — machine-parseable, schema-validated, one entry per line for easy diffing.

### 3.5 Orchestrator Gates (orchestrator_gates.mjs) — SOUND

Deterministic gate sequence: refine -> sign -> prepare. Hard gates:
- Refinement must be complete before signature
- USER_APPROVAL_EVIDENCE must match exact format
- Signature is one-time-use (checked via git grep)
- ENRICHMENT_NEEDED=YES blocks signature
- Orchestrator-Agentic execution lane is forbidden
- Worktree paths must be repo-relative (CX-109)

**Assessment:** This is the strongest governance component. The signature uniqueness check via git grep is clever — prevents reuse across all tracked files. The drive-agnostic enforcement (reject absolute paths) is a good safeguard for multi-machine workflows.

### 3.6 Orchestrator Resume (orchestrator-next.mjs) — SOUND

Reads gate logs + filesystem to infer next step without needing conversation context. Scoring/ranking when multiple WPs compete. Outputs structured LIFECYCLE + OPERATOR_ACTION + NEXT_COMMANDS.

**Assessment:** Critical for relay workflows where the orchestrator model loses context between sessions. The scoring heuristic (stage score + freshness + task board status) is pragmatic.

### 3.7 Validation (wp-communications-check.mjs) — SOUND

Now validates both directions:
- Packet -> artifacts (declared paths exist and validate against schema)
- Artifacts -> packet (no orphan communication folders without matching packet)
- Each RECEIPTS.jsonl line validated individually against schema

**Assessment:** Addresses the orphan-detection gap from v1. Comprehensive cross-validation.

### 3.8 Role Protocols — SOUND

All three role protocols now explicitly state:
- **Orchestrator:** Single-session, non-agentic. Historical agentic protocol is legacy reference only.
- **Coder:** Sub-agents allowed ONLY with explicit operator approval + packet declaration. Primary Coder remains solely accountable.
- **Validator:** Single-session, non-agentic. Event-driven wake-up via validator_trigger vocabulary. Must validate entire WP lineage, not just latest version.

**Assessment:** The non-agentic boundary (CX-218C) is the most important safety property in the system. It prevents privilege escalation through delegation chains — a known risk in multi-agent systems (see OpenClaw governance patterns).

---

## 4. ALIGNMENT WITH INDUSTRY PATTERNS

| Industry Pattern | Source | Handshake Implementation | Alignment |
|-----------------|--------|--------------------------|-----------|
| Schema-validated outputs | Lobster, A2A | WP_RUNTIME_STATUS.schema.json, WP_RECEIPT.schema.json | STRONG |
| Standardized status vocabulary | A2A Protocol | runtime_status enum + validator_trigger enum | STRONG |
| Bounded iteration limits | Lobster loop limits | MAX_CODER/VALIDATOR/RELAY_CYCLES | STRONG |
| Write-Ahead Log | OpenClaw WAL | RECEIPTS.jsonl (append-only, immutable) | STRONG |
| Verify Before Reporting | OpenClaw VBR | Packet lifecycle (stub -> official -> validated) | STRONG |
| Per-team collaboration surface | TinyClaw team rooms | THREAD.md per WP | STRONG |
| Artifact system (persistent outputs) | Anthropic research | Task packets + communication artifacts | STRONG |
| Non-agentic orchestrator | Your own design | CX-218C (not common in industry) | UNIQUE |
| Drive-agnostic governance | Your own design | CX-109 (repo-relative paths only) | UNIQUE |
| Deterministic gate sequence | Lobster conditional gating | orchestrator_gates.mjs | STRONG |
| Event-driven validator wake | A2A status callbacks | validator_trigger vocabulary | STRONG |
| Effort scaling hints | Anthropic | Heartbeat interval + stale_after in packet | MODERATE |
| Dead-letter handling | TinyClaw | Not implemented | GAP |
| Centralized metrics | OpenClaw Mission Control | Not implemented | GAP |
| Emergency stop mechanism | OpenClaw Gateway | Not implemented | GAP |

---

## 5. REMAINING ISSUES

### 5.1 Cycle Counter Management (MODERATE — by design)

`MAX_*_CYCLES` are defined in packets but `current_*_cycle` counters are NOT auto-incremented by any script. External tracking is required — the coder or operator must manually increment via wp-heartbeat args or direct RUNTIME_STATUS edit.

**Risk:** Cycle limits can be silently exceeded if actors forget to increment.
**Recommendation:** Add a `just wp-cycle-increment WP-{ID} {coder|validator|relay}` command that atomically increments the counter and refuses to proceed if max is reached.

### 5.2 Receipt Ledger Growth (LOW — expected)

RECEIPTS.jsonl is append-only with no archival/rotation mechanism. Very active WPs could accumulate thousands of entries.

**Risk:** Read performance degradation on large files.
**Recommendation:** Implement optional ledger rotation if a WP exceeds ~10K receipt lines (archive to RECEIPTS.archived.jsonl).

### 5.3 Thread Append-Only Not Enforced (LOW — by design)

THREAD.md has no schema validation. The append-only discipline is governance culture, not technical enforcement.

**Risk:** A role could accidentally or intentionally rewrite thread history.
**Recommendation:** Accept as-is. Technical enforcement would require git hooks or checksums that add complexity without proportional benefit.

### 5.4 Validator Wake-Up Latency (LOW — by design)

Stale heartbeat trigger depends on manual checks or orchestrator polling. No automatic validator invocation mechanism exists.

**Risk:** Long-running work with infrequent heartbeats may not trigger validator until next manual check.
**Recommendation:** Document expected heartbeat cadence per workflow lane (MANUAL_RELAY: operator checks; ORCHESTRATOR_MANAGED: orchestrator polls on session start).

### 5.5 No Emergency Stop (LOW — future consideration)

No mechanism to halt all agent work across all WPs simultaneously.

**Risk:** If a coder sub-agent goes off-track, there's no single kill switch.
**Recommendation:** Future enhancement. For now, the non-agentic boundary on orchestrator/validator + explicit sub-agent approval on coder provides sufficient containment.

---

## 6. OVERALL VERDICT

### Rating

| Dimension | V1 Score | V2 Score | Notes |
|-----------|----------|----------|-------|
| Design | 8/10 | 9/10 | Three-layer model is clean and well-reasoned |
| Implementation | 6/10 | 9/10 | Schemas, shared lib, comprehensive validation |
| Integration | 8/10 | 9/10 | Gate sequence, heartbeat, receipts all wired together |
| Documentation | 7/10 | 9/10 | Codex invariants, protocol updates, README all aligned |
| Robustness | 5/10 | 8/10 | Schema validation, non-destructive failure, drive-agnostic |
| Backward Compat | 9/10 | 9/10 | Legacy tolerance + forward migration path |
| Safety | 7/10 | 9/10 | CX-218C non-agentic boundary, signature uniqueness, gate sequence |
| **Overall** | **7/10** | **9/10** | **Production-grade governance for a file-based multi-agent system** |

### What Improved Most
1. **Schema validation** — From zero to comprehensive (41-field runtime status, 11-field receipts)
2. **Role boundaries** — From implicit to hard-gated (CX-218C, Codex enforcement)
3. **Lifecycle determinism** — From ad-hoc to locked sequence (refine -> sign -> prepare -> packet -> delegate)
4. **Audit trail** — From markdown placeholders to schema-validated JSONL ledger
5. **Error handling** — From silent failures to explicit warnings with repair paths

### What Remains
- Cycle counter auto-management (moderate, addressable)
- Ledger rotation for long-lived WPs (low, future)
- Emergency stop mechanism (low, future)

### Conclusion
This is a well-designed, comprehensive governance system for file-based multi-agent coordination. It aligns strongly with industry patterns from OpenClaw, Anthropic, and the A2A protocol while adding unique safety properties (non-agentic orchestrator/validator, drive-agnostic governance, signature uniqueness). The three-layer authority model (packet > structured coordination > freeform collaboration) is sound and the implementation is thorough.

The system is ready for production use in both MANUAL_RELAY and ORCHESTRATOR_MANAGED workflow lanes.

---

*Audit V2 complete. This is an evaluation-only document — no changes were made to the repository.*
