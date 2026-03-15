# Audit: WP_COMMUNICATIONS Governance Addition

## METADATA
- AUDIT_ID: AUDIT-20260311-WP-COMMUNICATIONS-GOV
- DATE_UTC: 2026-03-11
- AUDITOR: EXTERNAL (Claude Opus 4.6, evaluation-only)
- SCOPE_SUMMARY: Evaluate the WP_COMMUNICATIONS governance addition — new per-WP communication surface with THREAD, RUNTIME_STATUS, and RECEIPTS artifacts. Assess design, implementation, integration, and alignment with OpenClaw/TinyClaw patterns.
- BRANCH: role_orchestrator
- FILES_NEW: 6
- FILES_MODIFIED: 9

---

## 1. EXTERNAL REFERENCE CONTEXT

The orchestrator message states these changes were inspired by **OpenClaw** and **TinyClaw**. Here is what those systems provide as context for this audit.

### OpenClaw
- Open-source AI agent platform (150K+ GitHub stars) with multi-agent coordination
- Key governance patterns: per-agent tool allow/deny lists, minimal privilege, observability via OTEL traces, emergency gateway shutdown
- Community contributions include deterministic multi-agent pipelines (Lobster workflow engine) where LLMs do creative work and YAML workflows handle plumbing
- Skills ecosystem includes "agent-self-governance" (WAL, VBR, ADL) and "agent-team-orchestration" (defined roles, task lifecycles, handoff protocols, review workflows)

### TinyClaw
- Lightweight multi-agent collaboration framework by TinyAGI
- Key patterns: chained execution / fan-out, lightweight pub/sub event bus, persistent team chat rooms with `[#team_id: message]` tags, SQLite queue with atomic transactions and dead-letter management
- Agents collaborate via team mode with automatic message broadcasting

### Alignment Assessment
The WP_COMMUNICATIONS addition borrows sensible patterns from both:
- **From OpenClaw:** Deterministic governance hierarchy (packet > communication artifacts), role-based access patterns, validation-as-enforcement
- **From TinyClaw:** Per-WP collaboration surface (analogous to team chat rooms), append-only thread model, structured status tracking
- **Divergence (intentional):** This system is file-based and relay-friendly rather than pub/sub or API-driven, which fits the project's manual-relay + semi-autonomous hybrid model

---

## 2. DESIGN EVALUATION

### 2.1 Architecture

The addition introduces a three-artifact communication surface per work package:

| Artifact | Purpose | Format |
|----------|---------|--------|
| `THREAD.md` | Freeform append-only discussion | Markdown |
| `RUNTIME_STATUS.json` | Liveness, phase, watch state | JSON |
| `RECEIPTS.md` | Deterministic assignment/heartbeat/handoff records | Markdown |

**Verdict: SOUND.** The separation of concerns is clean — discussion, machine-readable state, and audit trail are distinct artifacts. The append-only constraint on THREAD.md prevents destructive edits. The JSON format for RUNTIME_STATUS enables programmatic reading.

### 2.2 Authority Hierarchy

The hard rule "packet wins" is stated in:
- `WP_COMMUNICATIONS/README.md`
- `TASK_PACKET_TEMPLATE.md`
- `CODER_PROTOCOL.md`
- `AGENTS.md`

**Verdict: SOUND.** Redundant reinforcement of the authority rule is appropriate — LLM agents benefit from repeated framing. No ambiguity about which source of truth prevails.

### 2.3 Backward Compatibility

The validator (`wp-communications-check.mjs`) only enforces the communication-folder contract for packets that declare WP_COMMUNICATION_DIR fields. Old packets without these fields pass validation.

**Verdict: SOUND.** Graceful opt-in avoids breaking existing workflows.

---

## 3. IMPLEMENTATION AUDIT

### 3.1 New Files

#### `.GOV/roles_shared/WP_COMMUNICATIONS/README.md`
- **Status:** Clean. Clear rules, explicit hierarchy, well-structured.
- **Issues:** None.

#### `.GOV/scripts/ensure-wp-communications.mjs`
- **Status:** Functional with gaps.
- **Issue 1 (MODERATE):** No existence check for template files before reading. If templates are deleted, the script throws a raw Node error instead of a helpful message.
- **Issue 2 (MODERATE):** Field parsing via regex (`parseSingleField()`, `parsePacketStatus()`) fails silently — returns empty strings on mismatch. If task packet metadata format evolves, extraction breaks without warning.
- **Issue 3 (LOW):** `AGENTIC_MODE` defaults to `<pending>` when packet is incomplete. This placeholder leaks into `RUNTIME_STATUS.json` and is never validated downstream.

#### `.GOV/scripts/validation/wp-communications-check.mjs`
- **Status:** Functional with edge cases.
- **Issue 4 (MODERATE):** All-or-nothing constraint — packet must declare all 4 WP_COMMUNICATION fields or none. If a packet accidentally declares 3 fields, validation fails rather than auto-completing or suggesting a fix.
- **Issue 5 (LOW):** Path normalization uses forward-slash conversion but no case-insensitive comparison. On Windows (which this repo runs on), path case mismatches could cause false failures.
- **Issue 6 (LOW):** Only validates packet → artifacts direction. Orphaned `WP_COMMUNICATIONS/{WP-ID}/` directories with no matching packet go undetected.

#### `.GOV/templates/WP_COMMUNICATION_THREAD_TEMPLATE.md`
- **Status:** Clean. Minimal and correct.
- **Issues:** None.

#### `.GOV/templates/WP_RECEIPTS_TEMPLATE.md`
- **Status:** Functional but underspecified.
- **Issue 7 (MODERATE):** Has `<append receipts here>` placeholders but no format specification. Agents won't know the expected structure for receipts (timestamp? actor? action? fields?). This will likely cause inconsistent receipt formats across roles.

#### `.GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json`
- **Status:** Functional but schema-loose.
- **Issue 8 (MODERATE):** Many fields default to `<pending>` (workflow_lane, execution_owner, heartbeat_due_at, stale_after, backup fields). No JSON schema or validation ensures these get populated. Consumers of this file cannot rely on field presence.
- **Issue 9 (LOW):** `attention_required` defaults to `false`. Bootstrap state might warrant `true` to ensure first-touch acknowledgment.

### 3.2 Modified Files

#### `TASK_PACKET_TEMPLATE.md`
- **Status:** Well-integrated.
- Adds 4 new metadata fields (WP_COMMUNICATION_DIR, WP_THREAD_FILE, WP_RUNTIME_STATUS_FILE, WP_RECEIPTS_FILE) and a new WP_COMMUNICATIONS section with rules and three-file breakdown.
- Format version updated to `2026-03-11`.
- **Issues:** None significant.

#### `TASK_PACKET_STUB_TEMPLATE.md`
- **Status:** Minimal change.
- Activation now expects communication folder to be created with the official packet.
- **Issues:** None.

#### `create-task-packet.mjs`
- **Status:** Well-integrated.
- Imports `ensureWpCommunications` and calls it after packet creation.
- **Issue 10 (LOW):** If `ensureWpCommunications()` throws, unclear whether packet creation is rolled back or left in an inconsistent state (packet exists but artifacts don't).

#### `gov-check.mjs`
- **Status:** Clean. Single import line added to wire in `wp-communications-check.mjs`.
- **Issues:** None.

#### `CODER_PROTOCOL.md`
- **Status:** Good guidance added.
- New section explains when/how to use communication artifacts.
- **Issue 11 (LOW):** No guidance on WHEN to update RUNTIME_STATUS.json (every commit? on status change? on session start/end?).

#### `ORCHESTRATOR_PROTOCOL.md`
- **Status:** Adequate.
- Confirms orchestrator responsibility to set up communication paths.
- **Issue 12 (LOW):** No explicit instruction to call `ensure-wp-communications` — relies on implicit integration via `create-task-packet.mjs`.

#### `VALIDATOR_PROTOCOL.md`
- **Status:** Thin.
- **Issue 13 (MODERATE):** Minimal guidance on validator's actual responsibilities regarding communication artifacts. Should validator update RUNTIME_STATUS? Append to RECEIPTS? Read-only? Unclear.

#### `AGENTS.md`
- **Status:** Clean. Clear operator-facing documentation.
- **Issues:** None.

#### `justfile`
- **Status:** Clean. Adds `ensure-wp-communications` command.
- **Issues:** None.

---

## 4. ISSUE SUMMARY

### By Severity

| # | Severity | Area | Description |
|---|----------|------|-------------|
| 7 | MODERATE | RECEIPTS_TEMPLATE | No receipt format specification — agents will produce inconsistent formats |
| 8 | MODERATE | RUNTIME_STATUS_TEMPLATE | Too many `<pending>` defaults with no schema validation |
| 13 | MODERATE | VALIDATOR_PROTOCOL | Validator responsibilities for communication artifacts undefined |
| 1 | MODERATE | ensure-wp-communications.mjs | No template file existence check — raw errors on missing templates |
| 2 | MODERATE | ensure-wp-communications.mjs | Silent regex parsing failures — no warning on field extraction miss |
| 4 | MODERATE | wp-communications-check.mjs | All-or-nothing field constraint too strict for partial declarations |
| 3 | LOW | ensure-wp-communications.mjs | `<pending>` placeholder leaks into RUNTIME_STATUS.json |
| 5 | LOW | wp-communications-check.mjs | No case-insensitive path comparison (Windows risk) |
| 6 | LOW | wp-communications-check.mjs | No orphan directory detection |
| 9 | LOW | RUNTIME_STATUS_TEMPLATE | Bootstrap `attention_required: false` may miss first-touch |
| 10 | LOW | create-task-packet.mjs | No rollback if artifact creation fails after packet is written |
| 11 | LOW | CODER_PROTOCOL | No timing guidance for RUNTIME_STATUS updates |
| 12 | LOW | ORCHESTRATOR_PROTOCOL | Implicit rather than explicit `ensure-wp-communications` reference |

### By Category

- **Schema / Format gaps:** Issues 7, 8 — the two most impactful; agents need structure to produce consistent artifacts
- **Error handling:** Issues 1, 2, 10 — silent failures or raw errors reduce debuggability
- **Role clarity:** Issues 11, 12, 13 — roles know communication artifacts exist but lack lifecycle guidance
- **Validation edge cases:** Issues 4, 5, 6 — validation is functional but has blind spots
- **Defaults:** Issues 3, 9 — placeholder values that may cause downstream confusion

---

## 5. OVERALL VERDICT

### What works well
- **Clean separation of concerns** — discussion, state, and receipts are distinct artifacts
- **Authority hierarchy is unambiguous** — packet wins, stated redundantly across all role protocols
- **Backward compatibility is preserved** — old packets pass validation; new system is opt-in
- **Automation is solid** — `create-task-packet.mjs` auto-bootstraps communication folders; `gov-check` enforces consistency
- **Design aligns with OpenClaw/TinyClaw patterns** — deterministic governance, per-team communication surfaces, structured liveness tracking

### What needs attention
- **Receipt and status format specification** — without a defined schema, multi-agent consistency will degrade over time. This is the single most important gap.
- **Validator role clarity** — the validator is the governance enforcement agent and needs explicit instructions for the new artifacts
- **Error handling hardening** — silent failures in field parsing and missing template checks will create hard-to-debug issues during autonomous operation

### Rating

| Dimension | Score | Notes |
|-----------|-------|-------|
| Design | 8/10 | Clean architecture, clear authority model |
| Implementation | 6/10 | Functional but error handling and schema gaps |
| Integration | 8/10 | Well-wired into existing create/validate pipeline |
| Documentation | 7/10 | Good high-level docs; lacks lifecycle and format guidance |
| Robustness | 5/10 | Silent failures, no schema validation, placeholder leaks |
| Backward Compat | 9/10 | Graceful opt-in, no breaking changes |
| **Overall** | **7/10** | **Solid foundation; needs format specs and error hardening before full autonomous operation** |

---

## 6. RECOMMENDATIONS (priority order)

1. **Define receipt format** — Add a structured example in RECEIPTS_TEMPLATE (timestamp, actor, action, reference fields)
2. **Add JSON schema for RUNTIME_STATUS** — Either inline or as a separate `.schema.json`, validate on write
3. **Clarify validator responsibilities** — Specify read-only vs append vs update permissions for each artifact
4. **Add template existence guards** in `ensure-wp-communications.mjs` with helpful error messages
5. **Add field extraction warnings** — Log when `parseSingleField()` returns empty for expected fields
6. **Add RUNTIME_STATUS update timing** to CODER_PROTOCOL (e.g., on session start, status change, session end)
7. **Consider orphan directory detection** in `wp-communications-check.mjs`

---

*Audit complete. This is an evaluation-only document — no changes were made to the repository.*
