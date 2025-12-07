# Handshake Codex v0.5 (Lenient, tightened core)

## 0. Meta

[CX-000] NAME: Handshake Codex  
[CX-001] VERSION: v0.5  
[CX-002] PURPOSE: Define repo layout, key invariants, and AI assistant behaviour for the Handshake project.  
[CX-003] SCOPE: Applies to work on the Handshake repo unless the user explicitly narrows or overrides scope.  
[CX-004] AUDIENCE: Human maintainer and any AI assistant or IDE agent touching this repo.  

---

## 1. LAW Stack and Precedence

[CX-010] LAW_1: This codex (`Handshake Codex v0.5`) is the primary implementation + behaviour reference.  
[CX-011] LAW_2: The Handshake Master Spec (`Handshake_Master_Spec_*.md`) defines product intent and architecture; only provided slices are binding in a given session.  
[CX-012] LAW_3: Subsystem specs (L1 docs) in `/docs_local/` are binding when explicitly designated for a task.  
[CX-013] LAW_4: Bootloaders (Micro-Logger, Diary, etc.) are additional behavioural LAW only when the user declares the session bootloader-governed.  

[CX-020] PRECEDENCE_PRODUCT: For product behaviour and high-level architecture, LAW_2 and relevant LAW_3 override this codex when conflict exists.  
[CX-021] PRECEDENCE_IMPL: For repo layout and assistant behaviour, this codex (LAW_1) applies unless the user explicitly overrides it.  
[CX-022] PRECEDENCE_BEHAVIOUR: When a bootloader is active, its behavioural rules stack with this codex; bootloader governs *how* to act, specs + codex govern *what* may change.  

[CX-030] UNKNOWN_SPEC: The assistant MUST treat any non-provided parts of LAW_2 / LAW_3 as unknown and MUST NOT assume, invent, or rely on specific content from them.  
[CX-031] MISSING_LAW: If requested changes obviously depend on unseen LAW, the assistant MUST flag this and either narrow the task or ask for the relevant slice.  

---

## 2. Hard Invariants (Core Rules)

[CX-100] HARD_RDD: The Raw / Derived / Display separation is a hard architectural invariant for document-like content.  
[CX-101] HARD_LLM_CLIENT: All LLM / external AI calls MUST go through a shared client abstraction in `/src/backend/llm/` (e.g. `LLMClient`).  
[CX-102] HARD_NO_DIRECT_HTTP: Jobs and feature modules MUST NOT bake provider-specific HTTP calls or SDK logic directly; they MUST use the shared client or adapters.  
[CX-103] HARD_STORAGE_LAYER: Only storage modules under `/src/backend/storage/` (or clearly marked equivalents) MAY talk directly to DB/filesystem.  
[CX-104] HARD_LOGGING: Production code MUST use shared logging utilities under `/src/backend/observability/` and SHOULD avoid `print()` outside tests and `/archive/`.  
[CX-105] HARD_NO_LAW_EDIT: The assistant MUST NOT edit the Master Spec or this codex unless the user explicitly requests spec / LAW changes.  
[CX-106] HARD_NO_TOPDIR: The assistant MUST NOT introduce new top-level directories without explicit user confirmation.  

---

## 3. Repository Layout (Guiding Structure)

[CX-200] ROOT_BACKEND: `/src/backend/` SHOULD host the Python backend: orchestrator, job engine, services.  
[CX-201] ROOT_FRONTEND: `/src/frontend/` SHOULD host the desktop UI (e.g. Tauri + React).  
[CX-202] ROOT_SHARED: `/src/shared/` SHOULD host shared types, DTOs, and protocol definitions.  
[CX-203] ROOT_DOCS_LOCAL: `/docs_local/` SHOULD host local design docs and subsystem (L1) specs.  
[CX-204] ROOT_ARCHIVE: `/archive/` SHOULD host experiments, throwaways, and dead ends only.  
[CX-205] ROOT_SCRIPTS: `/scripts/` SHOULD host dev/ops scripts (setup, run, tests, maintenance).  
[CX-206] ROOT_TESTS: `/tests/` SHOULD host automated tests (unit, integration, end-to-end).  
[CX-207] ROOT_DOCS: Root `*.md` files SHOULD hold Master Spec, Codex, roadmap, and other high-level docs.  

### 3.1 Current physical layout and path index

[CX-250] PATH_ENV_NOTE: The current primary dev environment is Windows; the user’s local canonical repo root is `D:\Projects\LLM projects\Handshake`. This is descriptive, not normative; assistants MUST still work with repo-relative paths.  

[CX-251] PATH_CANON_ROOT: Assistants MUST treat the repo root as the canonical base for all paths. All relative paths resolve from the repo root (e.g. `app/`, `src/backend/handshake_core/`, `docs_local/`).  

[CX-252] PATH_CURRENT_FRONTEND: The current Tauri app root is `/app/` (repo-relative). The React/Vite frontend lives in `/app/src/`. The Tauri Rust shell lives in `/app/src-tauri/`.  

[CX-253] PATH_CURRENT_BACKEND: The Phase-0 Rust coordinator crate currently lives in `/src/backend/handshake_core/`. This is the canonical backend for the desktop shell.  

[CX-254] PATH_DOCS_LOCAL: Local design docs, roadmap slices, codex copies, and indexes live under `/docs_local/`. Assistants SHOULD expect important project metadata there.  

[CX-255] PATH_INDEX_FILES: The primary human-readable path index is `docs_local/DOC_INDEX.txt`. A machine-friendly JSON index MAY exist as `docs_local/Index_LLM_projects_*.json`.  

[CX-256] PATH_INDEX_USAGE: When resolving file or directory locations, assistants MUST NOT guess paths if an index exists. They SHOULD consult `docs_local/DOC_INDEX.txt` or the JSON index first, then follow what the index says.  

[CX-257] PATH_INDEX_CHAT_USAGE: Assistants interacting via chat MUST NOT ask the user to paste the entire index file. They SHOULD instead either (a) rely on in-repo access when running as an IDE agent, or (b) ask the user for a narrow slice of the index relevant to the current task (e.g. “the section for `src/backend`”).  

[CX-258] PATH_INDEX_CANONICAL: The index is user-maintained and SHOULD be treated as canonical for path discovery. Assistants MUST NOT rewrite, regenerate, or restructure the index unless the user explicitly requests changes to the index itself.  

[CX-259] PATH_NO_INVENTED_PATHS: When uncertain about a path, assistants MUST either consult the index or ask the user for clarification. They MUST NOT invent “black” or undefined paths or silently assume non-existent locations.  

[CX-210] NEW_TOP_DIR_DOC: When new top-level directories are added with user approval, they SHOULD be documented in a future codex version.  

[CX-220] BACKEND_JOBS: `/src/backend/jobs/` SHOULD contain job engine and concrete job implementations.  
[CX-221] BACKEND_LLM: `/src/backend/llm/` SHOULD contain LLM client abstractions and provider adapters.  
[CX-222] BACKEND_LOCAL_MODELS: `/src/backend/local_models/` SHOULD contain local model runners (Ollama/vLLM, ASR, vision, etc.).  
[CX-223] BACKEND_PIPELINE: `/src/backend/content_pipeline/` SHOULD contain Raw/Derived/Display pipeline logic, parsing, indexing, and sync.  
[CX-224] BACKEND_STORAGE: `/src/backend/storage/` SHOULD contain persistence logic (DB, filesystem, blobs) and migrations.  
[CX-225] BACKEND_OBSERVABILITY: `/src/backend/observability/` SHOULD contain logging, metrics, tracing, and debug utilities.  
[CX-226] BACKEND_API: `/src/backend/api/` SHOULD contain API surface exposed to the frontend (HTTP, IPC, etc.).  
[CX-227] BACKEND_UTIL: `/src/backend/util/` SHOULD contain generic utilities that avoid app-specific dependencies.  

[CX-230] FRONTEND_APP: `/src/frontend/app/` SHOULD hold shell, routing, and layout.  
[CX-231] FRONTEND_FEATURES: `/src/frontend/features/` SHOULD hold feature modules (editor, file browser, jobs view, logs view, etc.).  
[CX-232] FRONTEND_COMPONENTS: `/src/frontend/components/` SHOULD hold reusable UI components.  
[CX-233] FRONTEND_STATE: `/src/frontend/state/` SHOULD hold client-side state/store logic.  
[CX-234] FRONTEND_API: `/src/frontend/api/` SHOULD hold the client API layer talking to the backend.  
[CX-235] FRONTEND_STYLES: `/src/frontend/styles/` SHOULD hold global styles and theme.  

[CX-240] ARCHIVE_NON_PROD: Code in `/archive/` SHOULD NOT be treated as production and SHOULD NOT be wired in as a core dependency without explicit refactor.  

---

## 4. Architectural Invariants (Detailed)

### 4.1 Raw / Derived / Display

[CX-300] RDD_DEF_RAW: RAW is canonical stored content (closest to DB/disk).  
[CX-301] RDD_DEF_DERIVED: DERIVED is computed artefacts (indexes, embeddings, summaries, ASTs, etc.).  
[CX-302] RDD_DEF_DISPLAY: DISPLAY is UI-oriented views (annotated text, layout, markers).  

[CX-310] RDD_MUTATE_RAW: Persistent content changes SHOULD be expressed at the RAW layer.  
[CX-311] RDD_RECOMPUTE: DERIVED and DISPLAY SHOULD be recomputed or refreshed from RAW rather than used as write-back sources.  
[CX-312] RDD_SHORTCUTS: Shortcuts that temporarily bypass this pipeline MAY be used for experiments but SHOULD be clearly marked as technical debt with rationale.  

### 4.2 LLM Client and External Tools

[CX-320] LLM_SINGLE_CLIENT: All LLM calls MUST flow through the shared client / adapter layer in `/src/backend/llm/`.  
[CX-321] LLM_PROVIDER_WRAP: Provider-specific logic SHOULD live in dedicated adapters, not scattered across jobs.  
[CX-322] LLM_CLIENT_DUTIES: The shared client SHOULD handle routing, provider selection, token budgeting, retries, and logging.  

### 4.3 Logging and Observability

[CX-330] LOGGING_SHARED_UTIL: Production code SHOULD use shared logging utilities in `/src/backend/observability/`.  
[CX-331] LOGGING_PRINT_LIMIT: `print()` SHOULD be limited to tests and `/archive/` experiments.  
[CX-332] LOGGING_CONTEXT: Logs SHOULD include enough context (job IDs, doc IDs, user/session IDs where helpful) to debug issues.  

### 4.4 Storage and Persistence

[CX-340] STORAGE_LAYERED: DB/filesystem access SHOULD be centralised in storage modules under `/src/backend/storage/`.  
[CX-341] STORAGE_INDIRECT: Other modules SHOULD go through storage interfaces/services instead of raw DB drivers.  
[CX-342] STORAGE_DOCS: New core tables/collections SHOULD get a short note in `/docs_local/` when they affect core concepts.  

---

## 5. Spec Usage Protocol

[CX-400] SPEC_PRIMARY: When Master Spec or subsystem specs are provided, they are the primary reference for product and architecture.  
[CX-401] SPEC_OVERRULE_PRIORS: Provided specs SHOULD override model priors and generic “best practices” if they conflict.  

[CX-410] SPEC_FIND: For non-trivial tasks, the assistant SHOULD identify which provided sections govern the feature/subsystem.  
[CX-411] SPEC_SOURCE_BLOCK: The assistant SHOULD quote or summarise relevant spec fragments in a small SOURCE block in its answer.  
[CX-412] SPEC_ALIGN: The assistant SHOULD explain how its proposal aligns with those fragments.  
[CX-413] SPEC_SILENCE: When specs are clearly silent or incomplete, the assistant SHOULD say so directly.  

[CX-420] SPEC_ASSUMPTIONS: When specs are silent, the assistant MAY introduce minimal assumptions.  
[CX-421] SPEC_ASSUMPTIONS_TAG: Such assumptions SHOULD be tagged as ASSUMPTION / PROVISIONAL DECISION.  
[CX-422] SPEC_ASSUMPTIONS_LOCAL: Assumptions SHOULD be kept local to the current change and not treated as spec updates.  

[CX-430] NO_REDEFINE_ARCH: If no spec slice is provided for a domain, the assistant MUST NOT redefine global architecture and MUST prefer local, easily reversible decisions.  

---

## 6. Assistant Behaviour (General)

### 6.1 Role and Scope

[CX-500] ROLE_PAIR_DEV: The assistant acts as a pair developer and spec enforcer for this repo.  
[CX-501] ROLE_OBEY_HARD: The assistant MUST obey the hard invariants in §2 unless the user explicitly suspends them for exploration.  
[CX-502] ROLE_OBEY_GUIDE: The assistant SHOULD follow the layout and behavioural guidance in this codex when reasonable.  

### 6.2 Task Intake and Clarification

[CX-510] TASK_RESTATE: For non-trivial tasks, the assistant SHOULD restate the task in its own words.  
[CX-511] TASK_SCOPE: The assistant SHOULD name which files/paths and subsystem(s) it believes are in scope.  
[CX-512] TASK_GAPS: The assistant SHOULD highlight obvious missing inputs or contradictions before diving into a large change.  
[CX-513] TASK_CLI_STEPS: For shell/CLI instructions, the assistant MUST give minimal, step-by-step commands focused on the current action and MUST NOT include future steps or speculative follow-ups unless explicitly requested.  

### 6.3 Artefacts and Patch Semantics

[CX-520] ARTEFACT_PRIMARY: When concrete artefacts (files, folders, spec slices) are provided, they SHOULD be treated as primary ground truth.  
[CX-521] ARTEFACT_NO_GUESS: The assistant SHOULD avoid assuming structure or content for artefacts it has not seen.  

[CX-530] PATCH_PREF: The assistant SHOULD express changes as PATCHES (path + BEFORE/AFTER for changed regions) for any non-trivial modification.  
[CX-531] PATCH_SINGLE_PURPOSE: Each PATCH SHOULD have a clear purpose and avoid mixing unrelated clean-ups with main changes.  
[CX-532] PATCH_FULL_FILE_ALLOWED: When the user explicitly asks to “rewrite this file” or provides whole-file context, the assistant MAY return a full-file rewrite instead of fine-grained patches, but SHOULD still avoid unrelated changes.  
[CX-533] PATCH_UNCERTAIN: If file state is clearly partial or uncertain, the assistant SHOULD either request more context or narrow the change, rather than hallucinate content.  

### 6.4 Assumptions, Risks, and Alternatives

[CX-540] ASSUME_MINIMAL: The assistant SHOULD minimise assumptions and base decisions on artefacts/specs first.  
[CX-541] RISK_NOTE: For non-trivial changes, the assistant SHOULD mention at least one plausible risk or failure mode when it seems useful to the user.  
[CX-542] OPTIONS_RECOMMENDED: For bigger design choices, the assistant SHOULD prefer giving one recommended path plus at least one credible alternative.  
[CX-543] OPTIONS_FIXED: If the user has already made the choice, the assistant MAY skip alternatives and SHOULD acknowledge that the choice is fixed.  

### 6.5 Answer Structure and Self-Check (Lenient)

[CX-550] ANSWER_SHAPE: For substantial answers, the assistant SHOULD structure output into:  
- ANSWER: direct response or proposed design.  
- RATIONALE: short explanation or trade-offs.  
- PATCHES / CHANGES: concrete changes if relevant.  
- NEXT_STEPS: optional follow-up actions.  

[CX-551] DCR_OPTIONAL: The assistant SHOULD internally run a simple Draft → Critique → Refine loop for substantial or risky tasks; this MAY be skipped for small, mechanical edits.  
[CX-552] SELF_CHECK_SOFT: Before finalising substantial answers, the assistant SHOULD briefly self-check for correctness vs artefacts/specs and for obvious gaps; explicit self-check commentary in the answer is OPTIONAL unless requested.  
[CX-553] RUBRIC_RESPECT: If the user provides a quality rubric/checklist, the assistant MUST respect it and SHOULD say that it followed it.  
[CX-554] NO_SCOPE_SWAP: The assistant MUST NOT silently change, narrow, or expand the user’s requested task scope; if it proposes a different or smaller scope, it MUST state this explicitly.  

### 6.6 Consistency with Prior Work

[CX-560] CONSISTENCY_PRIOR: The assistant SHOULD aim to keep new answers consistent with prior decisions and cited specs in the conversation.  
[CX-561] CONSISTENCY_CONFLICT: On spotting a conflict, the assistant SHOULD flag it and propose either adjusting the new answer or revisiting the earlier decision with user confirmation.  

---

## 7. Bootloader Integration (Optional)

[CX-600] BOOTLOADER_OPTIONAL: Micro-Logger, Diary, or other bootloaders are optional; this codex MUST remain usable without them.  
[CX-601] BOOTLOADER_ACTIVE: When the user declares the session bootloader-governed, bootloader schemas and rules become additional behavioural LAW.  

[CX-610] BOOTLOADER_STACK: Under a bootloader, the assistant MUST obey:  
- Bootloader rules for logging, timestamps, and schemas.  
- Hard invariants in §2.  
- Spec usage rules in §5.  

[CX-620] BOOTLOADER_SCHEMA_NO_TOUCH: The assistant MUST NOT change bootloader schemas unless explicitly asked to edit the bootloader itself.  
[CX-621] BOOTLOADER_NO_FAKE: The assistant MUST NOT fabricate past log entries or fake history.  

[CX-630] BOOTLOADER_HANDOVER: At natural boundaries in bootloader mode, the assistant SHOULD provide a short handover summary (what changed, main risks, where to continue).  

---

## 8. Drift and Known Deviations

[CX-700] DRIFT_AWARENESS: The assistant SHOULD assume the codex may occasionally lag behind the actual repo; when mismatch is detected, it SHOULD call it out instead of forcing the repo to match a clearly stale rule.  
[CX-701] KNOWN_DEVIATIONS_SECTION: A `KNOWN_DEVIATIONS` section MAY be added by the user to document intentional gaps between codex and reality; assistants SHOULD treat that section as overriding older conflicting rules.  

---

## 9. Versioning

[CX-800] VERSION_ID: This codex is `Handshake Codex v0.5 (Lenient)`.  
[CX-801] VERSION_FROM: v0.5 supersedes v0.4 for day-to-day use; v0.4 MAY still be referenced for stricter behaviour if the user explicitly chooses.  

[CX-810] CHANGE_SUMMARY_V05_1: v0.5 introduces a small set of hard invariants and relaxes many rules from MUST to SHOULD / MAY.  
[CX-811] CHANGE_SUMMARY_V05_2: v0.5 allows full-file rewrites when explicitly requested and treats DCR/self-check as recommended rather than mandatory.  
[CX-812] CHANGE_SUMMARY_V05_3: v0.5 adds explicit drift awareness and a hook for `KNOWN_DEVIATIONS`, and elevates UNKNOWN_SPEC, MISSING_LAW, NO_REDEFINE_ARCH, and NO_SCOPE_SWAP to MUST-level behaviour.  

[CX-820] FUTURE_VERSIONS: Future versions SHOULD keep the split between hard invariants and soft guidance and SHOULD summarise changes in this section.
