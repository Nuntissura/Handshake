# Task Packet: WP-1-ACE-Validators-v3

## Metadata
- TASK_ID: WP-1-ACE-Validators-v3
- WP_ID: WP-1-ACE-Validators-v3
- STATUS: Done
- DATE: 2025-12-26
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator

## User Context
The security system was previously "hollow"—it checked if security data existed but didn't look at what the data actually said. This task fixes that by forcing the system to read every piece of text it processes, clean it up (normalization), and scan it for hidden instructions. If an attack is detected, the system will now "poison" the job, stopping it immediately to protect your data.

## Scope
- **What**: Remediate hollow security implementation in ACE validators by enforcing content-awareness, NFC-normalization, and atomic poisoning logic.
- **Why**: Prevent "Hollow Guard" vulnerabilities where security checks are bypassed by omitting raw content validation.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/ace/mod.rs
  * src/backend/handshake_core/src/ace/validators/mod.rs
  * src/backend/handshake_core/src/ace/validators/injection.rs
  * src/backend/handshake_core/src/ace/validators/leakage.rs
  * src/backend/handshake_core/src/workflows.rs
- **OUT_OF_SCOPE**:
  * Implementing the Workflow Engine trap for poisoning (moved to WP-1-AI-Job-Model-v3).
  * Full migration to cloud models (Phase 2).
  * UI development for security violations (Phase 2).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Security-critical runtime enforcement; failure allows prompt injection or data leakage.
- **HARDENED_INVARIANTS**:
  * **Content-Awareness**: Validators MUST resolve raw UTF-8 content via `resolve_content()`. Metadata-only checks are FORBIDDEN.
  * **NFC Normalization**: All scans MUST use NFC-normalized, case-folded text to prevent homoglyph bypasses.
  * **Atomic Poisoning**: Injection detection MUST trigger immediate `JobState::Poisoned` and termination of all workflow nodes.
- **TEST_PLAN**:
  ```bash
  # Compile and unit test
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml

  # Verify specific security guards
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace::validators::injection
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace::validators::leakage

  # Full hygiene and workflow check
  just validate
  just post-work WP-1-ACE-Validators-v3
  ```
- **DONE_MEANS**:
  * ✅ `AceRuntimeValidator` trait implemented with `resolve_content()` support per §2.6.6.7.11.0.
  * ✅ `PromptInjectionGuard` successfully detects injection in NFC-normalized substrings per §2.6.6.7.11.6.
  * ✅ `CloudLeakageGuard` performs recursive classification checks on composite `SourceRefs` per §2.6.6.7.11.5.
  * ✅ Every safety violation emits `FR-EVT-SEC-VIOLATION` to Flight Recorder.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  ```

## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md (Master Spec v02.91 §2.6.6.7.11)
  * docs/ARCHITECTURE.md
  * src/backend/handshake_core/src/ace/validators/mod.rs
  * src/backend/handshake_core/src/ace/validators/injection.rs
  * src/backend/handshake_core/src/workflows.rs
- **SEARCH_TERMS**:
  * "AceRuntimeValidator"
  * "PromptInjectionDetected"
  * "JobState::Poisoned"
  * "unicode_normalization"
  * "FR-EVT-SEC-VIOLATION"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Hollow implementation" -> Security Bypass
  * "Normalization failure" -> Injection vulnerability
  * "Poisoning race condition" -> Unsafe Workspace Mutation

## Authority
- **SPEC_ANCHOR**: §2.6.6.7.11 (ACE Security Guards)
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.93)
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Assumptions**: None.
- **Open Questions**: None.
- **Dependencies**: Foundational.

---

# BOOTSTRAP (Remediation)
- Verified requirements for [CX-VAL-HARD].
- Identified necessary changes in AceError, SecurityViolation, and PromptInjectionGuard.

# SKELETON (Remediation)
- AceError::PromptInjectionDetected { pattern, offset, context }
- SecurityViolation { ..., offset, context }
- InjectionMatch { pattern, offset, context }
- handle_security_violation(..., offset, context)

SKELETON APPROVED [ilja261220252345]

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220252202

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-{ID}-variant), do NOT edit this one.**

---

## VALIDATION REPORT (APPENDED per CX-WP-001)

**Validator:** Senior Red Hat Auditor
**Date:** 2025-12-26
**Verdict:** PASS (with Waivers)

### Evidence Mapping (Spec → Code)

| Requirement | Evidence |
|-------------|----------|
| [CX-VAL-HARD] AceError expansion | `ace/mod.rs:71-76` - PromptInjectionDetected { offset, context } |
| [CX-VAL-HARD] Evidence extraction | `injection.rs:93-135` - scan_for_injection_nfc returns InjectionMatch |
| [CX-VAL-HARD] SecurityViolation capture | `validators/mod.rs:142-154` - fields added; `mod.rs:295-323` - conversion |
| [CX-VAL-HARD] Workflow propagation | `workflows.rs:73-131` - context/offset passed to handle_security_violation |
| [CX-VAL-HARD] FR schema storage | `workflows.rs:105-115` - FrEvt005SecurityViolation includes offset/context |
| Content Awareness [HSK-ACE-VAL-100] | `validators/mod.rs:114-165` - validate_trace_with_resolver |
| Atomic Poisoning [HSK-ACE-VAL-101] | `workflows.rs:63-138` - handle_security_violation |
| NFC Normalization [HSK-ACE-VAL-102] | `injection.rs:86-111` - scan_for_injection_nfc |

### Tests Executed

| Command | Result |
|---------|--------|
| `cargo test ace::validators::injection` | 9 passed |
| `cargo test ace::validators::leakage` | 10 passed |
| `cargo test storage::retention` | FAIL (Waiver WAIVER-20251226-01) |
| `just validate` | FAIL (Waiver WAIVER-20251226-02) |

### Waivers Granted [CX-573F]

- **WAIVER-20251226-01**: `storage::retention` test failures (is_pinned column mismatch). Pre-existing debt.
- **WAIVER-20251226-02**: `pnpm lint` failure (node_modules missing). Environment issue.

### REASON FOR PASS

All DONE_MEANS criteria satisfied with high-fidelity evidence (offsets/context). Security guards hardened against "hollow" reporting. Process gates documented.

**STATUS:** VALIDATED

---

## RE-AUDIT VALIDATION REPORT (Forensic)
Verdict: PASS

### Evidence Verification (Code Reality)
- `ace/mod.rs:71-76`: SATISFIED. `AceError::PromptInjectionDetected` carries `offset: usize` and `context: String`.
- `injection.rs:93-135`: SATISFIED. `scan_for_injection_nfc` returns `InjectionMatch` with accurate character offsets and context extraction.
- `validators/mod.rs:142-154`: SATISFIED. `SecurityViolation` struct expanded with high-fidelity evidence fields.
- `workflows.rs:73-131`: SATISFIED. `handle_security_violation` propagates forensic evidence to Flight Recorder.

### REASON FOR PASS
Code implementation verified via manual inspection of character offset and context window logic. High-fidelity evidence mapping is correctly synchronized across the stack.

**STATUS:** HARD-VALIDATED (2025-12-27)

## VALIDATION REPORT — 2025-12-27 (Revalidation)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-ACE-Validators-v3.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.93 (§2.6.6.7.11)
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/ace/validators/mod.rs (content awareness + default pipeline of 12 guards)
- src/backend/handshake_core/src/ace/validators/injection.rs (PromptInjectionGuard NFC normalization + offset/context capture)
- src/backend/handshake_core/src/ace/validators/leakage.rs (CloudLeakageGuard recursive classification/exportable enforcement)
- src/backend/handshake_core/src/workflows.rs:73-155 (handle_security_violation poison trap + FR-EVT-SEC-VIOLATION)

Findings:
- Spec alignment: Content Awareness [HSK-ACE-VAL-100], Atomic Poisoning [HSK-ACE-VAL-101], and NFC Normalization [HSK-ACE-VAL-102] implemented via content resolver + normalized scans and poisoning trap.
- Pipeline includes all §2.6.6.7.11 guards; PromptInjectionGuard enforces substring patterns and triggers poisoning; CloudLeakageGuard blocks non-exportable/high-sensitivity refs recursively.
- Forbidden Pattern Audit [CX-573E]: PASS for in-scope production paths (only unwraps in tests).
- Zero Placeholder Policy [CX-573D]: PASS; no stubs or disabled paths in production code.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace::validators::injection` (PASS)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace::validators::leakage` (PASS)

REASON FOR PASS: Implementation conforms to Master Spec v02.93 §2.6.6.7.11 with content-aware scanning, NFC normalization, and atomic poisoning; targeted validator tests pass.

## VALIDATION REPORT — 2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-ACE-Validators-v3.md (STATUS: Done/Validated)
- Spec: Packet references Handshake_Master_Spec_v02.91 (A2.6.6.7.11); docs/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.91). Current SPEC_CURRENT is v02.93, so alignment with the Main Body cannot be confirmed. This requires re-enrichment and evidence remapping before claiming Done.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor the packet to Master Spec v02.93 (A2.6.6.7.11), update DONE_MEANS/EVIDENCE_MAPPING, rerun the TEST_PLAN and validator scans, then resubmit for validation. Until then, status must return to Ready for Dev.
