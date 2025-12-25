# Work Packet: WP-1-Tokenization-Service

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §4.6 Tokenization Service  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Implement the unified TokenizationService required by §4.6 to ensure budget compliance and correct token counts across model architectures (OpenAI/GPT-4, Llama/Mistral via SentencePiece, fallback estimator). Integrate with AI job budgeting and enforce “no split_whitespace” for BPE models.

**Current State:**
- No explicit TokenizationService abstraction; risk of incorrect budgeting and truncation.
- No tests asserting correct token counts per model family.

**End State:**
- TokenizationService trait implemented with Tiktoken + SentencePiece + fallback estimator.
- Integrated into AI job budgeting and truncation logic.
- Tests covering GPT-like and Llama-like models; fallback used only on failure.

**Effort:** 8-12 hours  
**Phase 1 Blocking:** YES (Spec §4.6 is Phase 1 requirement)

---

## Technical Contract (LAW)
Governed by Master Spec §4.6:
- Implement `TokenizationService` with model-aware counting/truncation.
- Support Tiktoken (GPT-4), SentencePiece (Llama/Mistral), and a fallback estimator (char_count/4) only when primary tokenizers are unavailable.
- Enforce “no split_whitespace” for BPE models.
- Integrate with AI job budgeting (inputs/outputs) and truncation logic.

---

## Scope
### In Scope
1) TokenizationService trait + concrete implementations (Tiktoken, SentencePiece, fallback).
2) Wiring into AI job pipeline: budgeting, truncation, and metrics for GPT-like and Llama-like models.
3) Tests that validate token counts and truncation for both model families; fallback only on tokenizer failure.

### Out of Scope
- Model download/installation automation (handled separately in runtime docs).
- Perf optimization of tokenization (Phase 2+).

---

## Quality Gate
- **RISK_TIER:** HIGH
- **DONE_MEANS:**
  - TokenizationService trait implemented per §4.6 with Tiktoken + SentencePiece + fallback.
  - AI job budgeting/truncation uses TokenizationService; no split_whitespace for BPE.
  - Tests cover GPT-like + Llama-like paths; fallback used only on explicit failure path.
  - No forbidden patterns: unwrap/expect/panic/dbg in production; no serde_json::Value in domain.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`
  - `just validator-error-codes`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§4.6); src/backend/handshake_core/src/llm.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src; app/src (if any UI hooks)
- **SEARCH_TERMS:** "token", "budget", "truncate", "TokenizationService", "split_whitespace", "tiktoken", "sentencepiece"
- **RUN_COMMANDS:** cargo test; just validator-spec-regression; just validator-hygiene-full
- **RISK_MAP:** "Incorrect budgeting -> overrun costs"; "Missing tokenizer -> crash or bad truncation"; "Whitespace split for BPE -> wrong counts"

---

## Success Metrics
| Metric | Target | Verification |
|--------|--------|--------------|
| Correct counts | GPT-like + Llama-like counts match reference | Unit tests |
| Fallback | Used only on explicit tokenizer failure | Unit tests |
| Budgeting | AI job budgeting uses TokenizationService | Code evidence |

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
