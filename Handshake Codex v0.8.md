# Handshake Codex v0.8 (AI Autonomy with Deterministic Enforcement)

## 0. Meta

[CX-000] NAME: Handshake Codex
[CX-001] VERSION: v0.8
[CX-002] PURPOSE: Define repo layout, key invariants, and AI assistant behaviour for the Handshake project. Optimized for AI-autonomous software engineering with deterministic workflow enforcement.
[CX-003] SCOPE: Applies to work on the Handshake repo unless the user explicitly narrows or overrides scope.
[CX-004] AUDIENCE: Human maintainer and any AI assistant or IDE agent touching this repo. **NOTE**: This codex is designed for AI-autonomous operation where the human user may not have coding expertise.

---

## 1. LAW Stack and Precedence

[CX-010] LAW_1: This codex (`Handshake Codex v0.8`) is the primary implementation + behaviour reference.
[CX-011] LAW_2: The Handshake Master Spec (`Handshake_Master_Spec_*.md`) defines product intent and architecture; only provided slices are binding in a given session.
[CX-012] LAW_3: Subsystem specs (L1 docs) in `/docs_local/` are binding when explicitly designated for a task.
[CX-013] LAW_4: Bootloaders (Micro-Logger, Diary, etc.) are additional behavioural LAW when either (a) the user declares the session bootloader-governed, or (b) a bootloader artefact is present in-session and not explicitly disabled.

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

[CX-200] ROOT_BACKEND: `/src/backend/` SHOULD host the backend (language-agnostic: Rust/Python/etc.): orchestrator, job engine, services.
[CX-201] ROOT_FRONTEND: `/src/frontend/` SHOULD host the desktop UI (e.g. Tauri + React).
[CX-202] ROOT_SHARED: `/src/shared/` SHOULD host shared types, DTOs, and protocol definitions.
[CX-203] ROOT_DOCS_LOCAL: `/docs_local/` SHOULD host local design docs and subsystem (L1) specs.
[CX-204] ROOT_ARCHIVE: `/archive/` SHOULD host experiments, throwaways, and dead ends only.
[CX-205] ROOT_SCRIPTS: `/scripts/` SHOULD host dev/ops scripts (setup, run, tests, maintenance).
[CX-206] ROOT_TESTS: `/tests/` SHOULD host automated tests (unit, integration, end-to-end).
[CX-207] ROOT_DOCS: Root `*.md` files SHOULD hold Master Spec, Codex, roadmap, and other high-level docs.

[CX-208] ROOT_DOCS_CANONICAL: `/docs/` MUST contain canonical operational docs used for onboarding, navigation, and debugging.
[CX-209] NAVIGATION_PACK_FILES: `/docs/` MUST include (at minimum): `START_HERE.md`, `SPEC_CURRENT.md`, `ARCHITECTURE.md`, `RUNBOOK_DEBUG.md`.
[CX-213] TASK_PACKETS_DIR: `/docs/task_packets/` MUST exist and MUST contain task packet files for all active and recent work.
[CX-214] ROOT_APP_CURRENT: If `/app/` exists, it SHOULD be treated as the primary application root (frontend in `/app/src/`, backend in `/app/src-tauri/`) unless `docs/ARCHITECTURE.md` explicitly states otherwise.
[CX-215] DOCS_LOCAL_STAGING: `/docs_local/` SHOULD be treated as staging/drafts. Assistants MUST NOT treat `/docs_local/` as canonical onboarding/debugging guidance unless a document is explicitly promoted into `/docs/`.
[CX-216] PAST_WORK_INDEX: `/docs/` SHOULD include a `PAST_WORK_INDEX.md` (or equivalent) that links to older root-level specs/logs and `/docs_local/` drafts, so future maintainers can find prior work quickly without guesswork.

[CX-217] TASK_BOARD: `/docs/TASK_BOARD.md` MUST exist and serve as the high-level, at-a-glance status tracker for all project Work Packets (WPs). It is maintained by the Orchestrator.

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

[CX-333] LOG_ATTRIBUTION: Work artefacts (log entries, review notes, commit messages) SHOULD include a stable `AGENT_ID` and `ROLE` so "who did what" remains searchable months later.
[CX-334] AGENT_REGISTRY: The repo SHOULD keep an `AGENT_REGISTRY` (`/docs/agents/AGENT_REGISTRY.md`) mapping `AGENT_ID` -> current model/tooling + responsibility; changes to mappings SHOULD be logged.
[CX-335] LOG_MODEL_LABELS_OPTIONAL: If model/vendor names are captured for convenience, they SHOULD be treated as secondary labels (not primary identifiers) and SHOULD live in structured metadata fields (not scattered through free text), subject to any active bootloader constraints.

### 4.4 Storage and Persistence

[CX-340] STORAGE_LAYERED: DB/filesystem access SHOULD be centralised in storage modules under `/src/backend/storage/`.
[CX-341] STORAGE_INDIRECT: Other modules SHOULD go through storage interfaces/services instead of raw DB drivers.
[CX-342] STORAGE_DOCS: New core tables/collections SHOULD get a short note in `/docs_local/` when they affect core concepts.

[CX-343] DEBUG_ANCHORS: New errors SHOULD emit stable, searchable anchors (e.g., error codes like `HSK-####` or consistent log tags). `docs/RUNBOOK_DEBUG.md` SHOULD reference those anchors and the primary entrypoints for triage.

---

## 5. Spec Usage Protocol

[CX-400] SPEC_PRIMARY: When Master Spec or subsystem specs are provided, they are the primary reference for product and architecture.
[CX-401] SPEC_OVERRULE_PRIORS: Provided specs SHOULD override model priors and generic "best practices" if they conflict.

[CX-402] SPEC_CURRENT_POINTER: If multiple versions of the Master Spec exist in the repo, assistants MUST treat `docs/SPEC_CURRENT.md` as the canonical pointer to the current Master Spec for the active workline/session.

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
[CX-503] ROLE_AI_AUTONOMY: AI assistants are expected to operate autonomously within codex constraints. The human user may not have coding expertise and relies on deterministic workflow enforcement to ensure correctness.

[CX-504] USER_EXPERTISE: The human user of this session has explicitly stated they do not know how to code or work with software. All communication from AI agents (Orchestrator, Coder, etc.) MUST be presented in clear, non-technical language, explaining every step and providing analogies suitable for a non-expert audience, unless explicitly instructed otherwise by the user.

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
[CX-532] PATCH_FULL_FILE_ALLOWED: When the user explicitly asks to "rewrite this file" or provides whole-file context, the assistant MAY return a full-file rewrite instead of fine-grained patches, but SHOULD still avoid unrelated changes.
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
[CX-554] NO_SCOPE_SWAP: The assistant MUST NOT silently change, narrow, or expand the user's requested task scope; if it proposes a different or smaller scope, it MUST state this explicitly.

### 6.6 Consistency with Prior Work

[CX-560] CONSISTENCY_PRIOR: The assistant SHOULD aim to keep new answers consistent with prior decisions and cited specs in the conversation.
[CX-561] CONSISTENCY_CONFLICT: On spotting a conflict, the assistant SHOULD flag it and propose either adjusting the new answer or revisiting the earlier decision with user confirmation.

---

### 6.7 Review and Validation Gate

[CX-570] REVIEW_GATE: Any repo-changing patch MUST be reviewed (by a distinct Reviewer role/agent or an explicit review pass) before merge or before being treated as "done".
[CX-571] REVIEW_MIN_OUTPUT: A review MUST record: intent summary, key risks, required fixes, and exact validation commands run (or explicitly not run) with outcomes.
[CX-572] OK_REQUIRES_VALIDATION: The assistant MUST NOT claim a change is "OK", "verified", or "working" unless either (a) tests/checks ran and passed, or (b) the user explicitly validated the behaviour.
[CX-573] TRACEABILITY_MIN: Repo-changing work MUST be traceable to a work item (task packet / log entry / issue ID) referenced in the review note and ideally in the commit message.
[CX-573A] AI_REVIEW_GATE: Repo-changing work MUST run an automated AI review locally via the gemini CLI (just ai-review) and attach ai_review.md to the task packet/logger; WARN decisions must be acknowledged and BLOCK decisions block merge.

### 6.7A The Quality Rubric Gate

[CX-573B] RUBRIC_DRIVEN_VALIDATION: All non-trivial work packets delivered by a Coder role MUST be evaluated by the Orchestrator/Validator role against the official Quality Rubric. The Coder MUST use the rubric for self-assessment before submitting work, and the Validator MUST use it for the final review.

| Category | Needs Improvement (1) | Meets Expectations (2) | Exceeds Expectations (3) |
| :--- | :--- | :--- | :--- |
| **Correctness & Functionality** | Feature is incomplete, buggy, or does not meet the core requirements of the task packet. | Feature is implemented correctly as per the spec. All validation commands pass. | Functionality is robust, handles edge cases not explicitly mentioned, and is highly polished. |
| **Code Quality & Readability** | Code is difficult to understand, violates project conventions, or is poorly structured. | Code is clear, follows existing project conventions and style, and is reasonably easy to follow. | Code is exceptionally clear, idiomatic, and improves the structure of the surrounding code. |
| **Testing & Verification** | No tests are added for new functionality, or existing tests are broken. | New functionality is covered by adequate tests (unit or integration). All tests pass. | Tests are comprehensive, covering important edge cases, and significantly improve confidence in the code's reliability. |
| **Hygiene & Best Practices** | Linter fails. Obvious "code smells" (e.g., very large functions, commented-out code, magic numbers) are introduced. | Code passes all linter checks. Follows general best practices for the language and framework. | Code not only passes checks but actively reduces technical debt (e.g., refactors a messy section, improves typing). |
| **Reporting & Communication**| Report is missing, inaccurate, or does not provide the requested information for validation. | Report is accurate, complete, and provides all information requested in the task packet's `REPORTING` section. | Report provides extra insights, clearly explains complex trade-offs, and proactively identifies future risks or opportunities. |

---

### 6.8 Bootstrap Navigation Protocol (Non-Negotiable)

[CX-574] BOOTSTRAP_READ_SET: Before proposing changes, debugging, or reviewing, the assistant MUST read: `docs/START_HERE.md` and `docs/SPEC_CURRENT.md` (and the current logger if bootloader is active).
[CX-575] BOOTSTRAP_TASK_TYPE: The assistant MUST classify the task as one of: `DEBUG | FEATURE | REVIEW | REFACTOR | HYGIENE`.
[CX-576] BOOTSTRAP_FOLLOWUP_READ: After classification, the assistant MUST read the matching guide:
- DEBUG -> `docs/RUNBOOK_DEBUG.md`
- FEATURE/REFACTOR -> `docs/ARCHITECTURE.md`
- REVIEW -> `docs/ARCHITECTURE.md` + the diff/patch + validation instructions
[CX-577] BOOTSTRAP_OUTPUT_BLOCK: The assistant's first response in the session MUST include a short BOOTSTRAP block with:
- FILES_TO_OPEN: 5–15 concrete repo paths it will inspect first.
- SEARCH_TERMS: 5–20 exact strings/symbols/error codes it will grep.
- RUN_COMMANDS: the exact commands it will run (or `UNKNOWN` with explicit TODO placeholders).
- RISK_MAP: 3–8 likely failure modes and which subsystem they map to.
[CX-577A] BOOTSTRAP_TEMPLATE: The BOOTSTRAP block SHOULD follow this shape:
```
BOOTSTRAP
- FILES_TO_OPEN: docs/START_HERE.md; docs/SPEC_CURRENT.md; docs/ARCHITECTURE.md; docs/RUNBOOK_DEBUG.md; <feature/debug-specific paths>
- SEARCH_TERMS: "<key symbol>"; "<error>"; "<command>"; "<feature name>"
- RUN_COMMANDS: pnpm -C app tauri dev; pnpm -C app test; cargo test --manifest-path src/backend/handshake_core/Cargo.toml; (add task-specific)
- RISK_MAP: "<risk> -> <subsystem>"; "<risk> -> <subsystem>"
```
[CX-578] NAVIGATION_UPDATE_TRIGGER: When work uncovers new entrypoints, invariants, or a repeatable failure mode, the assistant MUST update the relevant doc in `/docs/` (START_HERE/ARCHITECTURE/RUNBOOK_DEBUG) as part of the same work packet/commit unless the user explicitly defers.
[CX-579] NAVIGATION_GATE: For non-trivial repo-changing work, the reviewer MUST block completion if no `/docs/` navigation pointer was added/updated (or a clear justification is recorded).

### 6.9 Orchestrator Task Packet Protocol (AI Autonomy - Mandatory)

[CX-580] ORCH_PACKET_REQUIRED: Orchestrators MUST create a task packet before delegating work to coder/debugger agents. The packet MUST be written to `docs/task_packets/WP-{ID}-{name}.md` OR embedded in the handoff message with full structure.

[CX-581] ORCH_PACKET_STRUCTURE: Every packet MUST include:
- TASK_ID: WP-{phase}-{short-name}
- RISK_TIER: LOW | MEDIUM | HIGH
- SCOPE: Clear description of what's in/out of scope
- IN_SCOPE_PATHS: Specific files/directories
- OUT_OF_SCOPE: What NOT to change
- TEST_PLAN: Exact validation commands
- DONE_MEANS: Specific success criteria
- ROLLBACK_HINT: How to undo changes
- BOOTSTRAP: FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP

[CX-582] ORCH_PACKET_VERIFICATION: The orchestrator MUST verify the packet file exists (if file-based) OR that the embedded packet is complete before delegating work.

[CX-583] ORCH_HANDOFF_PROTOCOL: When delegating to a coder agent, the orchestrator MUST include:
- Path to task packet file (if file-based) OR full packet content (if embedded)
- WP_ID for traceability
- RISK_TIER from packet
- Explicit confirmation: "✅ Task packet {WP_ID} created and verified"

[CX-584] ORCH_BLOCKING_RULE: If the orchestrator cannot create a complete packet (unclear requirements, missing context, ambiguous scope), it MUST STOP and request clarification from the user. The orchestrator MUST NOT delegate incomplete or ambiguous work.

[CX-585] ORCH_LOGGER_ENTRY: The orchestrator MUST create a RAW_ENTRY in the logger documenting the task packet creation BEFORE delegating work. The entry MUST include WP_ID and TASK_SUMMARY at minimum.

[CX-586] ORCH_AUTHORITY_DOCS: Packets MUST include pointers to: `docs/START_HERE.md`, `docs/SPEC_CURRENT.md`, `docs/ARCHITECTURE.md`, `docs/RUNBOOK_DEBUG.md`, `docs/QUALITY_GATE.md`, and the latest logger file.

[CX-587] ORCH_PRE_WORK_CHECK: Before delegating, the orchestrator SHOULD run (or instruct the coder to run): `just pre-work {WP_ID}` to verify the packet is complete and system is ready for work.

### 6.10 Coder Pre-Work Verification (AI Autonomy - Mandatory)

[CX-620] CODER_PACKET_CHECK: Before writing any code, the coder agent MUST verify a task packet exists by checking:
1. File exists at `docs/task_packets/WP-*.md` (created recently), OR
2. Logger contains RAW_ENTRY with matching WP_ID, OR
3. Orchestrator message includes complete TASK_PACKET block

[CX-621] CODER_BLOCKING_RULE: If no task packet is found, the coder MUST:
1. Output: "❌ BLOCKED: No task packet found [CX-620]"
2. STOP all work immediately
3. Request task packet from orchestrator or user
4. DO NOT write any code until packet is verified

[CX-622] CODER_BOOTSTRAP_MANDATORY: The coder MUST output a BOOTSTRAP block per [CX-577] BEFORE the first file modification. This confirms the coder has read the task packet and understands scope.

[CX-623] CODER_VALIDATION_LOG: Before claiming work is complete, the coder MUST:
1. Run all commands from TEST_PLAN
2. Document results in a VALIDATION block
3. Include command + outcome for each check
4. Run `just post-work {WP_ID}` to verify completeness

### 6.11 Hygiene Gate (commands + scope)

[CX-630] HYGIENE_SCOPE: Changes SHOULD stay scoped to the task; avoid drive-by refactors or unrelated cleanups.
[CX-631] HYGIENE_COMMANDS: For repo-changing work, assistants SHOULD run (or explicitly note not run): `just docs-check`; `just codex-check`; `pnpm -C app run lint`; `pnpm -C app test`; `pnpm -C app run depcruise`; `cargo fmt`; `cargo clippy --all-targets --all-features`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`; `cargo deny check advisories licenses bans sources`.
[CX-632] HYGIENE_TODOS: When touching code near TODOs, assistants SHOULD either resolve them or leave a dated note explaining why they remain.
[CX-633] HYGIENE_DOC_UPDATE: If new entrypoints, commands, or repeatable failures are introduced or discovered, assistants SHOULD update the relevant doc (START_HERE/ARCHITECTURE/RUNBOOK_DEBUG) in the same packet unless the user defers.

### 6.12 Determinism Anchors (large-system hygiene)

[CX-640] ANCHOR_ERRORS: New errors SHOULD include stable error codes (`HSK-####`) and/or log tags; these anchors SHOULD be referenced in `docs/RUNBOOK_DEBUG.md` when adding repeatable failures.
[CX-641] OWNERSHIP_MAP: Area/module ownership SHOULD be captured in `/docs/OWNERSHIP.md` with paths, reviewers, and notes; packets SHOULD consult/update it when adding new surface area.
[CX-642] PRIMITIVE_TESTS: New primitives/features SHOULD ship with at least one targeted test and a short invariant note (place in `docs/ARCHITECTURE.md` or inline doc comment); silence requires an explicit reason.
[CX-643] CI_GATE: Continuous integration SHOULD run `just validate` (or an equivalent subset) and block merge on failures.
[CX-644] FLAGS: New interwoven features SHOULD use a feature flag or clearly documented toggle; note the flag/toggle location in `docs/ARCHITECTURE.md` or the relevant module doc.
[CX-645] ERROR_CODES_REQUIRED: New errors SHOULD introduce stable error codes/log tags (e.g., `HSK-####`) and record them in `docs/RUNBOOK_DEBUG.md` when they become repeatable.
[CX-646] TEST_EXPECTATION: Logic changes SHOULD add or update at least one targeted test; if omitted, a written reason MUST be recorded in the review/task packet.
[CX-647] REVIEW_REQUIRED: Repo-changing work SHOULD have a distinct reviewer role sign off, recording commands run and outcomes.
[CX-648] SECRETS_AND_SUPPLY_CHAIN: CI SHOULD include secret scanning and dependency audit steps; assistants MUST avoid committing secrets and SHOULD pin critical dependencies/lockfiles.
[CX-649] ROLLBACK_HINTS: Reviews/commits SHOULD include a brief rollback hint (e.g., git hash or steps) for traceability.
[CX-649A] TODO_POLICY: New TODOs in source code and scripts MUST include a tracking tag in the form `TODO(HSK-####): ...` and be searchable by ID. Docs SHOULD use `TBD (HSK-####)` or explicit prose instead of TODO.

### 6.13 Logger Requirements (Strengthened for AI Autonomy)

[CX-650] LOGGER_ENTRY_REQUIRED: All repo-changing work MUST have a corresponding RAW_ENTRY in the logger. Work without a logger entry MUST be rejected at review.

[CX-651] LOGGER_ENTRY_TIMING: The RAW_ENTRY MUST be written:
- By orchestrator: When creating task packet (before delegation)
- By coder: When completing work (before commit request)

[CX-652] LOGGER_VERIFICATION: Before requesting commit, the coder MUST verify:
1. Logger entry exists with matching WP_ID
2. FILES_TOUCHED matches actual changed files
3. RESULT field is set appropriately (OK/PARTIAL/FAIL)
4. VALIDATION section documents commands run

[CX-653] LOGGER_PACKET_LINK: Every logger entry MUST reference its task packet WP_ID. Every task packet MUST have at least one logger entry.

---

## 7. Bootloader Integration (Optional)

[CX-700] BOOTLOADER_OPTIONAL: Micro-Logger, Diary, or other bootloaders are optional; this codex MUST remain usable without them.
[CX-701] BOOTLOADER_ACTIVE: When either (a) the user declares bootloader mode, or (b) a bootloader artefact is present in-session, bootloader schemas and rules become additional behavioural LAW unless explicitly disabled.

[CX-702] BOOTLOADER_DISABLE: If the user explicitly disables bootloader mode for a session, the assistant MUST treat bootloader rules as inactive for that session.

[CX-710] BOOTLOADER_STACK: Under a bootloader, the assistant MUST obey:
- Bootloader rules for logging, timestamps, and schemas.
- Hard invariants in §2.
- Spec usage rules in §5.

[CX-720] BOOTLOADER_SCHEMA_NO_TOUCH: The assistant MUST NOT change bootloader schemas unless explicitly asked to edit the bootloader itself.
[CX-721] BOOTLOADER_NO_FAKE: The assistant MUST NOT fabricate past log entries or fake history.

[CX-730] BOOTLOADER_HANDOVER: At natural boundaries in bootloader mode, the assistant SHOULD provide a short handover summary (what changed, main risks, where to continue).

---

## 8. Drift and Known Deviations

[CX-800] DRIFT_AWARENESS: The assistant SHOULD assume the codex may occasionally lag behind the actual repo; when mismatch is detected, it SHOULD call it out instead of forcing the repo to match a clearly stale rule.
[CX-801] KNOWN_DEVIATIONS_SECTION: A `KNOWN_DEVIATIONS` section MAY be added by the user to document intentional gaps between codex and reality; assistants SHOULD treat that section as overriding older conflicting rules.

[CX-810] KNOWN_DEVIATION_APP_LAYOUT: The repo currently includes `/app/` (Tauri app). If codex layout guidance conflicts with observed `/app/src` + `/app/src-tauri`, assistants MUST follow the observed layout and document the deviation in `docs/ARCHITECTURE.md`.
[CX-811] KNOWN_DEVIATION_MULTI_SPECS: The repo may contain multiple `Handshake_Master_Spec_v*.md` versions at root. `docs/SPEC_CURRENT.md` is the authoritative pointer for current work.
[CX-812] KNOWN_DEVIATION_DOC_SPLIT: `/docs/` is canonical operational guidance; `/docs_local/` is staging/drafts; root-level `*.md` may contain governance/history.

---

## 9. Automated Enforcement (AI Autonomy Requirements)

[CX-900] ENFORCEMENT_PURPOSE: For AI-autonomous operation, the workflow MUST be enforced by automated scripts and checks. Manual enforcement is insufficient when the human user lacks coding expertise.

[CX-901] ENFORCEMENT_SCRIPTS: The repo MUST include enforcement scripts in `/scripts/validation/`:
- `pre-work-check.mjs` - Verifies task packet exists before work starts
- `post-work-check.mjs` - Verifies logger entry and validation before commit
- `task-packet-check.mjs` - Validates packet structure
- `logger-check.mjs` - Validates logger entry format
- `ci-traceability-check.mjs` - CI verification of workflow compliance

[CX-902] ENFORCEMENT_HOOKS: Git hooks SHOULD enforce:
- pre-commit: Blocks commits without WP-ID traceability
- pre-push: Verifies all commits reference valid task packets

[CX-903] ENFORCEMENT_JUST: The `justfile` MUST include:
- `just create-task-packet {wp-id}` - Creates task packet from template
- `just pre-work {wp-id}` - Validates readiness before implementation
- `just post-work {wp-id}` - Validates completeness before commit
- `just validate-workflow {wp-id}` - Full workflow compliance check

[CX-904] ENFORCEMENT_CI: GitHub Actions SHOULD verify:
- All commits reference task packets via WP-ID
- Logger entries exist for all repo-changing commits
- Validation commands are documented in commits/reviews
- No commits bypass workflow requirements

[CX-905] ENFORCEMENT_FAILURE: If automated checks fail, work MUST be rejected with:
1. Clear error message indicating which rule was violated
2. Reference to codex rule number (e.g., "[CX-620]")
3. Remediation steps to fix the issue
4. AI agents MUST NOT override enforcement without explicit user permission

[CX-906] ENFORCEMENT_PROTOCOLS: The repo MUST include protocol files in `docs/`:
- `docs/ORCHESTRATOR_PROTOCOL.md` - Mandatory checklist for orchestrators
- `docs/CODER_PROTOCOL.md` - Mandatory checklist for coders
- These protocols MUST be read by AI agents before performing their respective roles

---

## 10. Versioning

[CX-950] VERSION_ID: This codex is `Handshake Codex v0.8 (AI Autonomy with Deterministic Enforcement)`.
[CX-951] VERSION_FROM: v0.8 supersedes v0.7 for all use. v0.7 MAY still be referenced for comparison but v0.8 is authoritative.

[CX-960] CHANGE_SUMMARY_V08_1: v0.8 strengthens orchestrator and coder requirements from SHOULD to MUST for AI autonomy. Task packet creation [CX-580] and coder pre-work verification [CX-620] are now mandatory and blocking.

[CX-961] CHANGE_SUMMARY_V08_2: v0.8 adds §9 "Automated Enforcement" defining required scripts, hooks, and CI checks to enforce workflow deterministically without relying on AI agent compliance alone.

[CX-962] CHANGE_SUMMARY_V08_3: v0.8 strengthens logger requirements [CX-650-653] from SHOULD to MUST, making work traceability non-negotiable.

[CX-963] CHANGE_SUMMARY_V08_4: v0.8 adds [CX-503] explicitly stating this codex is optimized for AI-autonomous operation where the human user may not have coding expertise.

[CX-964] CHANGE_SUMMARY_V08_5: v0.8 adds [CX-213] requiring `docs/task_packets/` directory and [CX-906] requiring `docs/` protocol files for orchestrator/coder agents.

[CX-970] FUTURE_VERSIONS: Future versions SHOULD maintain the distinction between hard requirements (MUST) for workflow enforcement and soft guidance (SHOULD) for implementation details.

---

## SUMMARY FOR AI AGENTS

**If you are an Orchestrator:**
1. Read `docs/ORCHESTRATOR_PROTOCOL.md` FIRST
2. Create task packet (`just create-task-packet WP-{ID}`)
3. Verify (`just pre-work WP-{ID}`)
4. Add logger entry
5. Only then delegate to coder

**If you are a Coder/Debugger:**
1. Read `docs/CODER_PROTOCOL.md` FIRST
2. Verify task packet exists [CX-620]
3. Output BOOTSTRAP block [CX-622]
4. Implement within scope
5. Run validation (`just post-work WP-{ID}`)
6. Document in logger before commit

**If you are a Reviewer:**
1. Verify task packet exists for the work
2. Verify logger entry documents work
3. Check validation commands were run
4. Run `just ai-review` if not already done
5. Block merge if workflow was bypassed

**Blocking rules apply.** If any MUST requirement is violated, work stops until fixed.
