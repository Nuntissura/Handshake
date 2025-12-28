# Task Packet: WP-1-Terminal-LAW-v2

## Metadata
- **TASK_ID:** WP-1-Terminal-LAW-v2
- **WP_ID:** WP-1-Terminal-LAW-v2
- **DATE:** 2025-12-28T17:00:00Z
- **REQUESTOR:** User (ilja)
- **AGENT_ID:** Orchestrator
- **ROLE:** Orchestrator
- **STATUS:** Done (VALIDATED)
- **SUPERSEDES:** WP-1-Terminal-LAW (stale SPEC_ANCHOR, incomplete structure)

---

## Authority
- **SPEC_CURRENT:** Handshake_Master_Spec_v02.96.md
- **SPEC_ANCHOR:** §10.1 Terminal Experience (Main Body)
- **Codex:** Handshake Codex v1.4.md
- **Task Board:** docs/TASK_BOARD.md
- **Strategic Pause Approval:** [ilja281220251700]

---

## User Context (Non-Technical Explainer)

This task ensures that when AI models run terminal commands, they cannot accidentally (or maliciously) interact with human-created terminal sessions. Think of it like having separate workspaces: the AI gets its own terminal area, and humans get theirs. The AI cannot peek into or type into the human's terminal without explicit permission.

Additionally, every AI terminal action must be fully traceable - we can always see which AI job ran which command, in which workspace, with which permissions. This creates an audit trail for security and debugging.

---

## Scope

### What
Implement Terminal LAW session type enforcement and AI isolation per Master Spec §10.1 (TERM-UX-001 through TERM-UX-003).

### Why
- **Security:** Prevent AI from attaching to/reading human terminal sessions (RCE vector mitigation)
- **Auditability:** Complete trace linkage for all AI terminal operations
- **Phase 1 Closure:** Terminal LAW is a blocking requirement for Phase 1 closure

### IN_SCOPE_PATHS
- `src/backend/handshake_core/src/terminal/mod.rs` (re-export session types)
- `src/backend/handshake_core/src/terminal/session.rs` (NEW - session type definitions)
- `src/backend/handshake_core/src/terminal/guards.rs` (add session type checks)
- `src/backend/handshake_core/src/terminal.rs` (integrate session types)
- `src/backend/handshake_core/src/flight_recorder/mod.rs` (ensure session_type in events)
- `src/backend/handshake_core/tests/terminal_session_tests.rs` (NEW - session isolation tests)

### OUT_OF_SCOPE
- Problem matchers (TERM-DIAG-*) - separate WP
- Secure/sandbox mode (TERM-SEC-003) - v1 optional, separate WP
- Platform-specific PTY/ConPTY abstractions - separate WP
- Consent/approval UI - frontend WP
- Terminal panel UI integration - frontend WP

---

## Quality Gate

### RISK_TIER: HIGH
- **Rationale:** Security-critical (AI terminal isolation); spec-governed; Phase 1 blocker
- **Requires:** cargo test + clippy + AI review + post-work validation

### HARDENED_INVARIANTS [CX-VAL-HARD]
1. **Content-Awareness:** Session type MUST be checked before any AI terminal operation
2. **NFC Normalization:** All session IDs MUST be NFC-normalized (already in TerminalRequest)
3. **Atomic Poisoning Prevention:** Session type field MUST be non-optional enum (no default bypass)

### TEST_PLAN
```bash
# Compile and unit tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Clippy (all targets)
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings

# Format check
cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check

# External Cargo target hygiene
just cargo-clean

# Post-work validation
just post-work WP-1-Terminal-LAW-v2
```

### DONE_MEANS
1. [ ] `TerminalSessionType` enum defined with variants: `HumanDev`, `AiJob`, `PluginTool` (per TERM-UX-001)
2. [ ] `TerminalSession` struct holds session_type, job_id, wsids, capability_set (per TERM-UX-003)
3. [ ] `TerminalGuard::check_session_isolation` method blocks AI access to `HumanDev` sessions (per TERM-UX-002)
4. [ ] AI attachment override requires explicit capability `terminal.attach_human` + logged consent
5. [ ] FlightRecorder events include `session_type` field for all terminal operations
6. [ ] All tests pass including new session isolation tests
7. [ ] No `unwrap()` in production paths (use `?` or explicit error handling)
8. [ ] `just post-work WP-1-Terminal-LAW-v2` returns PASS

### ROLLBACK_HINT
```bash
git revert <commit-hash>
# Single commit reverts:
# 1. TerminalSessionType enum (session.rs)
# 2. TerminalSession struct (session.rs)
# 3. Guard integration (guards.rs)
# 4. Test file (terminal_session_tests.rs)
```

---

## Bootstrap (Coder Work Plan)

### FILES_TO_OPEN
- `docs/START_HERE.md` (repository overview)
- `docs/SPEC_CURRENT.md` (current spec version)
- `Handshake_Master_Spec_v02.96.md` §10.1 (Terminal Experience LAW)
- `docs/ARCHITECTURE.md` (system architecture)
- `docs/CODER_PROTOCOL.md` (coder workflow)
- `src/backend/handshake_core/src/terminal.rs` (current implementation)
- `src/backend/handshake_core/src/terminal/guards.rs` (guard trait)
- `src/backend/handshake_core/src/terminal/config.rs` (config struct)
- `src/backend/handshake_core/src/terminal/redaction.rs` (redaction patterns)
- `src/backend/handshake_core/src/flight_recorder/mod.rs` (event recording)
- `src/backend/handshake_core/src/capabilities.rs` (capability registry)
- `src/backend/handshake_core/tests/terminal_guards_tests.rs` (existing tests)

### SEARCH_TERMS
- `"TerminalRequest"` (current request struct)
- `"TerminalGuard"` (guard trait)
- `"JobContext"` (job linkage)
- `"FlightRecorderEvent"` (event structure)
- `"session_id"` (existing session field)
- `"TERM-UX-001"` (spec reference)
- `"TERM-UX-002"` (AI attachment rule)
- `"TERM-UX-003"` (trace linkage)
- `"HumanDev"` (session type name)
- `"AiJob"` (session type name)
- `"check_capability"` (existing capability check)
- `"terminal.exec"` (capability ID)

### RUN_COMMANDS
```bash
# Verify current state
cargo check --manifest-path src/backend/handshake_core/Cargo.toml

# Run existing tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml -- terminal

# Check for existing session handling
grep -r "session" src/backend/handshake_core/src/terminal/
```

### RISK_MAP
- "Session type bypass via None/default" -> Terminal isolation failure (CRITICAL)
- "AI attaches to human session" -> Security violation, audit failure (CRITICAL)
- "Missing session_type in FlightRecorder" -> Audit trail incomplete (HIGH)
- "Test coverage gap for isolation" -> Validator blocks merge (MEDIUM)
- "Breaking change to TerminalRequest" -> Existing code fails to compile (MEDIUM)

---

## Validation

### Deterministic Manifest (COR-701)

| target_file | change_type | gates |
|-------------|-------------|-------|
| src/backend/handshake_core/src/terminal/session.rs | CREATE | G-SCHEMA, G-CAP |
| src/backend/handshake_core/src/terminal/mod.rs | MODIFY | G-INTEGRITY |
| src/backend/handshake_core/src/terminal/guards.rs | MODIFY | G-CAP, G-INTEGRITY |
| src/backend/handshake_core/src/terminal.rs | MODIFY | G-INTEGRITY |
| src/backend/handshake_core/tests/terminal_session_tests.rs | CREATE | G-DET |

### Evidence Mapping Template
```
[TERM-UX-001] TerminalSessionType enum -> src/backend/handshake_core/src/terminal/session.rs:L??
[TERM-UX-002] AI isolation check -> src/backend/handshake_core/src/terminal/guards.rs:L??
[TERM-UX-003] Trace linkage fields -> src/backend/handshake_core/src/terminal/session.rs:L??
```

---

## Notes

### Assumptions
- Existing `TerminalGuard` trait can be extended with session isolation check
- `JobContext` struct provides sufficient linkage (job_id, model_id, session_id, wsids)

### Open Questions
- None (spec §10.1 is comprehensive)

### Dependencies
- **Depends on:** WP-1-Security-Gates-v2 (VALIDATED - provides foundation)
- **Blocks:** WP-1-Terminal-Integration-Baseline (needs session types)

---

**Last Updated:** 2025-12-28
**User Signature Locked:** ilja281220251700

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-1-Terminal-LAW-v3), do NOT edit this one.**

---

## VALIDATION REPORT — WP-1-Terminal-LAW-v2
Verdict: **PASS**

### Scope Inputs:
- **Task Packet**: `docs/task_packets/WP-1-Terminal-LAW-v2.md`
- **Spec**: `Handshake_Master_Spec_v02.96` §10.1 (Terminal Experience LAW)

### Files Checked:
- `src/backend/handshake_core/src/terminal/mod.rs` (Migrated from `terminal.rs`)
- `src/backend/handshake_core/src/terminal/session.rs` (New)
- `src/backend/handshake_core/src/terminal/guards.rs` (Modified)
- `src/backend/handshake_core/src/capabilities.rs` (Modified - Scope Expansion)
- `src/backend/handshake_core/src/flight_recorder/mod.rs` (Modified)
- `src/backend/handshake_core/tests/terminal_session_tests.rs` (New)

### Findings:
- **Correctness & Functionality: PASS.** 
  - Implementation successfully enforces terminal isolation between AI and Human sessions.
  - The `terminal.attach_human` capability is correctly integrated into the SSoT (`CapabilityRegistry`).
  - Session type derivation logic (AiJob vs HumanDev) is sound and follows the approved skeleton.
- **Hygiene & Forbidden Patterns: PASS.**
  - **Audit [CX-573E]**: No `unwrap()`, `expect()`, or `panic!` found in production paths (validated).
  - **Zero Placeholder Policy [CX-573D]**: No hollow structs or mocks. The `terminal.attach_human` check is real and backed by the registry.
- **Spec Alignment: PASS.** 
  - **TERM-UX-001/003**: `TerminalSessionType` and `TerminalSession` correctly carry audit metadata.
  - **TERM-UX-002**: Isolation guard prevents AI attachment to human terminals without explicit capability AND logged consent.
  - **Auditability**: `TerminalCommandEvent` (FR-EVT-007) now captures `session_type`, `human_consent_obtained`, and `capability_set`.

### Tests:
- `blocks_ai_from_human_session_without_attach_capability`: **PASS**
- `allows_ai_with_attach_capability_and_logged_consent`: **PASS**
- `flight_recorder_captures_session_type_and_consent`: **PASS**
- All 118 backend tests passed.

### REASON FOR PASS:
The work satisfies 100% of the **Terminal Experience LAW (§10.1)** requirements. The implementation of the **Logged Consent** flag and the **Scope Expansion** into the `CapabilityRegistry` ensures that the security gate is not a "placeholder" but a deterministic architectural invariant.

---

### Task Packet Update (APPEND-ONLY):
- **STATUS Update**: **Done** (VALIDATED)
- **Closure Reason**: WP-1-Terminal-LAW-v2 closed successfully. All isolation rules and audit linkage requirements implemented and verified with integration tests.
- **User Signature Locked**: ilja281220251700

