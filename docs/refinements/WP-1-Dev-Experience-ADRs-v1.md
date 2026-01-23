## TECHNICAL_REFINEMENT

- WP_ID: WP-1-Dev-Experience-ADRs-v1
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.115.md
- SPEC_TARGET_SHA1: 61e500454062bacbe70578ada7989286c0742973

- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec provides normative guidance for the Phase 1 default local model runtime (Ollama) and the canonical developer command surface ("just dev") plus ADRs as canonical governance artifacts; this WP is implementing missing repo-level setup/docs/ADRs so the existing code can be exercised in a functional Phase 1 dev environment.

- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already defines (a) Ollama as the recommended Phase 1 runtime (and shows how to run it) and (b) the canonical repo "How to run" command surface and ADR placement; this WP only needs to bring the repo/docs/ADRs into alignment with that existing spec guidance.
- USER_REVIEW_STATUS: APPROVED
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Dev-Experience-ADRs-v1
- USER_SIGNATURE: ilja230120262310

#### ANCHOR
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 4.2.2.2 (Ollama - The Easy Choice)
- CONTEXT_START_LINE: 15625
- CONTEXT_END_LINE: 15638
- CONTEXT_TOKEN: #### 4.2.2.2 Ollama
- EXCERPT_ASCII_ESCAPED:
```text
Ollama is the recommended Phase 1 local runtime. The spec shows:

- Install and run a model in one command:
  ollama run mistral

- Or start as a server:
  ollama serve

- Then call via API at localhost:11434
```

#### ANCHOR
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 7.5.4.9.3 (Template bodies - docs/START_HERE.md: How to run)
- CONTEXT_START_LINE: 26290
- CONTEXT_END_LINE: 26335
- CONTEXT_TOKEN: ## How to run
- EXCERPT_ASCII_ESCAPED:
```text
The spec defines the canonical "How to run" command surface, including:

- pnpm -C app tauri dev
- just dev
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml

It also explicitly calls out that additional setup (DB seed/env/etc) must be documented once known.
```

#### ANCHOR
- SPEC_ANCHOR: Handshake_Master_Spec_v02.115.md 7.5.4.9.3 (AI_WORKFLOW_TEMPLATE: ADRs are canonical inputs)
- CONTEXT_START_LINE: 32906
- CONTEXT_END_LINE: 32912
- CONTEXT_TOKEN: 5) ADRs (`docs/adr/`)
- EXCERPT_ASCII_ESCAPED:
```text
Canonical inputs and precedence include ADRs under docs/adr/ as a first-class governance artifact.
```

## PROPOSED_SPEC_ENRICHMENT
```text
<not applicable; ENRICHMENT_NEEDED=NO>
```

## GAPS_IDENTIFIED
- Ollama is not installed on the Operator machine, so Phase 1 LLM-backed functionality cannot be exercised in dev.
- docs/START_HERE.md still contains: "If additional setup (DB seed, env) is required: TBD (HSK-1001) - document once known." This WP must replace that TBD with concrete, Phase 1-functional setup steps for Ollama and any required env vars.
- docs/adr/ exists (ADR-0001), but it does not provide narrow, decision-specific ADRs for: runtime selection (Ollama), DB layout for jobs/Flight Recorder, and capability model shape.

## FLIGHT_RECORDER_INTERACTION
- This WP is dev-experience oriented, but it must ensure the local dev flow enables observable LLM usage:
  - When LLM features are exercised, Flight Recorder should record llm inference events (event_type "llm_inference") consistent with the spec's FR-EVT-006 (LlmInferenceEvent) shape.
- Validation/telemetry triggers expected during dev:
  - Ollama runtime is reachable at OLLAMA_URL (default http://localhost:11434) before starting the app.
  - A minimal "smoke" interaction exists to cause at least one llm_inference event so developers can verify end-to-end wiring.

## RED_TEAM_ADVISORY
- Supply-chain risk: installing a system-wide runtime (Ollama) is a privileged action; installation method must be explicit and reproducible (and never commit model weights into git).
- Data exfil risk: ensure docs do NOT instruct users to log prompts/responses in a way that leaks secrets; prefer hashes/usage only (consistent with FR-EVT-006).
- Port collision risk: Ollama binds localhost:11434; dev docs must call out how to diagnose/resolve port conflicts.
- Resource risk: large models can thrash GPU/VRAM; default dev instructions should pick a reasonable model and explain how to override OLLAMA_MODEL.

## PRIMITIVES
- Runtime + env:
  - Ollama installed (Windows) and running locally
  - Env vars used by Handshake: OLLAMA_URL (default http://localhost:11434), OLLAMA_MODEL (default "llama3")
- Command surface:
  - just dev (dev startup entrypoint; should assert Ollama is available when Phase 1 requires it)
- Docs artifacts:
  - docs/START_HERE.md updated to remove the HSK-1001 TBD for required Phase 1 setup
  - docs/adr/ADR-0002-*.md (runtime selection: Ollama)
  - docs/adr/ADR-0003-*.md (DB layout for jobs + Flight Recorder)
  - docs/adr/ADR-0004-*.md (capability model shape)
