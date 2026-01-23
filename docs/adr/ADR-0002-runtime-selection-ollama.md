# ADR-0002: Runtime Selection (Phase 1 Local LLM) - Ollama

- **Status:** Accepted
- **Date:** 2026-01-23
- **Context:** Phase 1 LLM-backed features require a deterministic local model runtime. The Master Spec recommends Ollama as the easiest Phase 1 local runtime (API at http://localhost:11434) and the codebase already supports `OLLAMA_URL` and `OLLAMA_MODEL`.

## Decision
- For Phase 1 development, **Ollama is the required local model runtime**.
- The default runtime endpoint is `OLLAMA_URL=http://localhost:11434` (trailing slash ignored).
- The default model identifier is `OLLAMA_MODEL=llama3` (developers may override locally, e.g. `mistral`).
- `just dev` must fail fast with a clear message if Ollama is missing or not reachable at `OLLAMA_URL`.

## Alternatives Considered
- **No local runtime (disable LLM in dev):** Rejected for Phase 1; it prevents exercising LLM-backed paths and Flight Recorder events.
- **Cloud-only provider (e.g., OpenAI):** Rejected for Phase 1 due to network dependency, cost, and governance posture.
- **vLLM / other server runtimes:** Deferred; heavier operational burden than needed for Phase 1 onboarding.

## Consequences
- **Pros:** One-command model download + run; local-first posture; stable HTTP interface; deterministic startup when preflighted.
- **Cons:** Requires local installation; port 11434 conflicts are possible; model choice impacts performance/VRAM.

## Follow-ups
- Keep `docs/START_HERE.md` as the canonical Phase 1 setup guide for installing/running/verifying Ollama on Windows.
- Add additional runtime ADRs only when a non-Ollama runtime is introduced for Phase 2+.
