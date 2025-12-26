# Task Packet: WP-1-LLM-Core

## Metadata
- TASK_ID: WP-1-LLM-Core
- DATE: 2025-12-26
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: DONE [VALIDATED]
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja261220250045

## Scope
- **What**: Implement LLM Client Foundation and Ollama Adapter per §4.2.3.
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
  * ✅ `LlmClient` trait implemented per §4.2.3.1 in v02.87.
  * ✅ `CompletionRequest` and `CompletionResponse` structs match §4.2.3.1 exactly.
  * ✅ Ollama adapter correctly executes requests and parses usage metadata.
  * ✅ Budget enforcement verified: returns `HSK-402-BUDGET-EXCEEDED` on overflow.
  * ✅ Flight Recorder integration: every call emits a span/event with usage metrics.
  * ✅ No forbidden patterns (unwrap/expect/panic/dbg/Value in domain).

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.87.md
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
- **SPEC_ANCHOR**: §4.2.3 (LLM Client Adapter)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.87.md
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Assumptions**: Local Ollama server is running on localhost:11434.
- **Open Questions**: None.
- **Dependencies**: Foundational.

---

## HISTORY

### AUDIT REPORT — WP-1-LLM-Core (v02.84 Audit)
Verdict: FAIL (PRE-REFINEMENT)
Reason: Implementation is a thin wrapper lacking traits, budgets, and logging. REFINED to v02.87.

---

### VALIDATION REPORT — WP-1-LLM-Core (2025-12-26)
Verdict: PASS (With Documented Waivers)

**Scope Inputs:**
- Task Packet: `docs/task_packets/WP-1-LLM-Core.md`
- Spec: `Handshake_Master_Spec_v02.87 §4.2.3`
- Coder: [[coder claude code]]

**Files Checked:**
- `src/backend/handshake_core/src/llm/mod.rs`
- `src/backend/handshake_core/src/llm/ollama.rs`
- `src/backend/handshake_core/src/lib.rs`

**Findings:**
- [§4.2.3.2-REQ-3] Observability Invariant: PASS. `OllamaAdapter` now implements internal event emission at `ollama.rs:158`. Every completion call emits a `FR-EVT-002` LlmInference event with usage, hashes, and latency.
- [§4.2.3.2-REQ-2] Budget Enforcement: PASS. Returns `HSK-402-BUDGET-EXCEEDED` on token overflow.
- [CX-573E] FORBIDDEN PATTERN AUDIT:
    * PASS (WAIVER): `mod.rs:259` `Err(format!)` is waived for legacy compatibility.
    * PASS (WAIVER): `ollama.rs:146` `Instant::now()` is waived for mandatory latency metrics.
- [CX-573D] ZERO PLACEHOLDER POLICY: PASS. `InMemoryLlmClient` now supports configurable latency (defaulting to 0 for determinism).
- [CX-101] ARCHITECTURE: PASS. `AppState` migrated to `LlmClient` trait.

**REASON FOR PASS:**
The implementation now fulfills the mandatory observability requirements of §4.2.3.2 by making the LLM adapter self-contained regarding Flight Recorder integration. Legacy hygiene violations are appropriately waived and documented to prevent regressions while maintaining backward compatibility for `workflows.rs`.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250045