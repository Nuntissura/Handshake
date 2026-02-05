# Task Packet: WP-1-LLM-Core

## Metadata
- TASK_ID: WP-1-LLM-Core
- DATE: 2025-12-26
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator


## SKELETON APPROVED
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja261220250045

## Scope
- **What**: Implement LLM Client Foundation and Ollama Adapter per ??4.2.3.
- **Why**: Provide a portable, auditable core for LLM interactions that enforces token budgets and emits Flight Recorder events.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/llm/mod.rs
  * src/backend/handshake_core/src/llm/ollama.rs
  * src/backend/handshake_core/src/models.rs
- **OUT_OF_SCOPE**:
  * Implementing higher-level job logic (Workflow Engine).
  * Supporting non-Ollama providers (Phase 2).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Foundation for all AI actions; failure risks silent token leakage and vendor lock-in.
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-spec-regression
  just validator-scan WP-1-LLM-Core
  just validator-hygiene-full
  ```
- **DONE_MEANS**:
  * ??? `LlmClient` trait implemented per ??4.2.3.1 in v02.93.
  * ??? `CompletionRequest` and `CompletionResponse` structs match ??4.2.3.1 exactly.
  * ??? Ollama adapter correctly executes requests and parses usage metadata.
  * ??? Budget enforcement verified: returns `HSK-402-BUDGET-EXCEEDED` on overflow.
  * ??? Flight Recorder integration: every call emits a span/event with usage metrics.
  * ??? No forbidden patterns (unwrap/expect/panic/dbg/Value in domain).

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/START_HERE.md
  * .GOV/roles_shared/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.93.md
  * src/backend/handshake_core/src/llm.rs
- **SEARCH_TERMS**:
  * "LlmClient"
  * "Ollama"
  * "TokenUsage"
  * "max_tokens"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Ollama API mismatch" -> LLM layer
  * "Budget leakage" -> Safety gate failure
  * "Missing observability" -> Compliance failure

## Authority
- **SPEC_ANCHOR**: ??4.2.3 (LLM Client Adapter)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.93.md
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md

## Notes
- **Assumptions**: Local Ollama server is running on localhost:11434.
- **Open Questions**: None.
- **Dependencies**: Foundational.

---

## HISTORY

### AUDIT REPORT ??? WP-1-LLM-Core (v02.84 Audit)
Verdict: FAIL (PRE-REFINEMENT)
Reason: Implementation is a thin wrapper lacking traits, budgets, and logging. REFINED to v02.87.

---

### VALIDATION REPORT ??? WP-1-LLM-Core (2025-12-26)
Verdict: PASS (With Documented Waivers)

**Scope Inputs:**
- Task Packet: `.GOV/task_packets/WP-1-LLM-Core.md`
- Spec: `Handshake_Master_Spec_v02.87 ??4.2.3`
- Coder: [[coder claude code]]

**Files Checked:**
- `src/backend/handshake_core/src/llm/mod.rs`
- `src/backend/handshake_core/src/llm/ollama.rs`
- `src/backend/handshake_core/src/lib.rs`

**Findings:**
- [??4.2.3.2-REQ-3] Observability Invariant: PASS. `OllamaAdapter` now implements internal event emission at `ollama.rs:158`. Every completion call emits a `FR-EVT-002` LlmInference event with usage, hashes, and latency.
- [??4.2.3.2-REQ-2] Budget Enforcement: PASS. Returns `HSK-402-BUDGET-EXCEEDED` on token overflow.
- [CX-573E] FORBIDDEN PATTERN AUDIT:
    * PASS (WAIVER): `mod.rs:259` `Err(format!)` is waived for legacy compatibility.
    * PASS (WAIVER): `ollama.rs:146` `Instant::now()` is waived for mandatory latency metrics.
- [CX-573D] ZERO PLACEHOLDER POLICY: PASS. `InMemoryLlmClient` now supports configurable latency (defaulting to 0 for determinism).
- [CX-101] ARCHITECTURE: PASS. `AppState` migrated to `LlmClient` trait.

**REASON FOR PASS:**
The implementation now fulfills the mandatory observability requirements of ??4.2.3.2 by making the LLM adapter self-contained regarding Flight Recorder integration. Legacy hygiene violations are appropriately waived and documented to prevent regressions while maintaining backward compatibility for `workflows.rs`.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250045

## VALIDATION REPORT â€” 2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-LLM-Core.md (STATUS: Validated)
- Spec: Packet references Handshake_Master_Spec_v02.87; .GOV/roles_shared/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.87). Current SPEC_CURRENT is v02.93, so LLM Core requirements and evidence must be rechecked against the updated Main Body before claiming Done.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor LLM Core DONE_MEANS to Master Spec v02.93, refresh EVIDENCE_MAPPING, rerun TEST_PLAN/validator scans, and resubmit. Status must return to Ready for Dev until revalidated.

## VALIDATION REPORT â€” 2025-12-27 (Revalidation, Spec v02.93)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-LLM-Core.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.93 (Â§4.2.3 LLM Client Adapter)
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/llm/mod.rs:15-126 (LlmClient trait; CompletionRequest/Response; TokenUsage; ModelProfile)
- src/backend/handshake_core/src/llm/mod.rs:159-180 (HSK-402 BudgetExceeded error)
- src/backend/handshake_core/src/llm/ollama.rs:24-236 (OllamaAdapter budget enforcement, FR-EVT-002 emission)

Findings:
- Spec alignment: Completion types and LlmClient trait match Â§4.2.3; budget enforcement returns HSK-402-BUDGET-EXCEEDED on overflow; rate-limit surfaces HSK-429.
- Observability: Ollama adapter emits FR-EVT-002 with hashes, usage, and latency; uses shared Flight Recorder.
- Forbidden Pattern Audit [CX-573E]: PASS (validator-scan).
- Zero Placeholder Policy [CX-573D]: PASS; no stubs.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` (PASS; warnings: unused imports, deprecated helper)
- `just validator-scan` (PASS)

REASON FOR PASS: LLM core complies with Spec v02.93 Â§4.2.3 with strict enums/traits, budget enforcement, and Flight Recorder observability; targeted tests and validator scan passed.

---

## REVALIDATION REPORT - WP-1-LLM-Core (2025-12-30)

VALIDATION REPORT - WP-1-LLM-Core
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-LLM-Core.md
- Spec Pointer: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (4.2.3 LLM Client Adapter; 11.5 Flight Recorder)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Commands (evidence):
- just validator-spec-regression: PASS
- node .GOV/scripts/validation/gate-check.mjs WP-1-LLM-Core: FAIL ("SKELETON appears before BOOTSTRAP")
- node .GOV/scripts/validation/post-work-check.mjs WP-1-LLM-Core: FAIL (non-ASCII packet + missing COR-701 manifest fields/gates)
- just validator-packet-complete WP-1-LLM-Core: FAIL (STATUS missing/invalid)

Blocking Findings:
1) Phase gate FAIL: `node .GOV/scripts/validation/gate-check.mjs WP-1-LLM-Core` fails.
   - '## SKELETON APPROVED' appears before '## BOOTSTRAP' (.GOV/task_packets/WP-1-LLM-Core.md:11 and .GOV/task_packets/WP-1-LLM-Core.md:49).
2) Deterministic manifest gate FAIL: `node .GOV/scripts/validation/post-work-check.mjs WP-1-LLM-Core` fails.
   - Packet contains non-ASCII characters (count=12).
   - No COR-701 manifest fields parsed (target_file/start/end/pre_sha1/post_sha1/line_delta), and required gates are missing/un-checked.
3) Packet completeness gate FAIL: `just validator-packet-complete WP-1-LLM-Core` fails because the packet does not contain a canonical `**Status:**` marker.
4) Spec mismatch: packet is anchored to Handshake_Master_Spec_v02.93.md, but .GOV/roles_shared/SPEC_CURRENT.md now requires Handshake_Master_Spec_v02.98.md.

Spec-to-code spot-check (non-exhaustive; blocked by gates above):
- Spec 4.2.3.1 defines CompletionRequest without trace_id (Handshake_Master_Spec_v02.98.md:10495); implementation adds trace_id: Uuid (src/backend/handshake_core/src/llm/mod.rs:44).
- Spec 11.5 defines FR-EVT-002 as EditorEditEvent (Handshake_Master_Spec_v02.98.md:30812); code defines FrEvt002LlmInference (src/backend/handshake_core/src/flight_recorder/mod.rs:214) and emits FlightRecorderEventType::LlmInference (src/backend/handshake_core/src/flight_recorder/mod.rs:35), which has no matching event schema in v02.98 (rg \"llm_inference\" in spec -> no matches).

Forbidden Pattern Audit (scoped to LLM files):
- unwrap() found only in test code (src/backend/handshake_core/src/llm/ollama.rs:351).

REASON FOR FAIL:
- Required workflow gates (gate-check + COR-701 post-work-check) do not pass. Additionally, the implementation appears misaligned with current spec v02.98 (CompletionRequest fields; Flight Recorder event taxonomy).

Required Remediation:
- Create a NEW packet (recommended: WP-1-LLM-Core-v2) anchored to Handshake_Master_Spec_v02.98.md.
- Ensure BOOTSTRAP and SKELETON headings exist and appear before any headings containing \"SKELETON\" (gate-check uses /#+ SKELETON/i).
- Make the packet ASCII-only and include a COR-701 VALIDATION manifest that satisfies post-work-check.
- Resolve spec/code alignment for CompletionRequest (trace_id) and Flight Recorder event IDs (requires code changes or explicit spec version bump).

Task Board Update:
- Move WP-1-LLM-Core from Done -> Ready for Dev (Revalidation FAIL).

Packet Status Update (append-only):
- **Status:** Ready for Dev

Timestamp: 2025-12-30
Validator: Codex CLI (Validator role)



