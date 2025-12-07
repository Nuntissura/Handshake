HANDSHAKE MICRO-LOGGER — v3.1
=============================
Single-file Handshake logger with:
- INSTRUCTIONS worksurface (all metadata, rules, schemas, templates)
- LOG worksurface (all live logging: compressed log + raw log)
No example entries are present by design.


[INSTRUCTIONS_SURFACE_START]
============================


A. METADATA / CONFIG (SUB-WORKSURFACE)
--------------------------------------
[INSTR_META_SURFACE_START]

[HL-M-001] LOGGER_VERSION
Handshake Logger v3.1

[HL-M-002] LOGGER_SCOPE
Handshake project (all repos / forks / machines)

[HL-M-003] HANDSHAKE_ROOT_PATH
</absolute/path/to/Handshake/root>

[HL-M-004] DEFAULT_TIMEZONE
Europe/Brussels

[HL-M-005] RAW_MAX_ENTRIES
30

[HL-M-006] RAW_COMPRESS_BATCH_SIZE
20

[HL-M-007] RAW_MIN_RECENT_TO_KEEP
10

[HL-M-008] SESSION_ID_SCHEME
HS-YYYYMMDD-HHMM-<short-tag>

[HL-M-009] WP_ID_SCHEME
WP-<phase-number>-<short-name> (e.g. WP-0-LLMClient-Base)

[HL-M-010] ROLE_VALUES
Owner | Orchestrator | Coder

[HL-M-011] RESULT_VALUES
OK | FAIL | PARTIAL | BLOCKED

[HL-M-012] ARCHIVE_FILE_SUGGESTION
Handshake_logger_archive.txt  (external file for full old RAW entries)

[INSTR_META_SURFACE_END]


B. LOGGING RULES (SUB-WORKSURFACE)
----------------------------------
[INSTR_RULES_SURFACE_START]

[HL-I-001] This file is the single Handshake micro-logger used to track Work Packets and major spec work.

[HL-I-002] All assistants and coders MUST read the INSTRUCTIONS worksurface before writing new log entries.

[HL-I-003] All metadata, rules, schemas, and templates MUST remain inside the INSTRUCTIONS worksurface.

[HL-I-004] All live logging MUST occur only inside the LOG worksurface.

[HL-I-005] Within the LOG worksurface, COMPRESSED log entries MUST live in the COMPRESSED sub-worksurface and RAW log entries MUST live in the RAW sub-worksurface.

[HL-I-006] The INSTRUCTIONS worksurface SHOULD be treated as read-only during normal development; edit only when updating the logger design or version.

[HL-I-007] Every substantial unit of work MUST be represented by at least one RAW entry.

[HL-I-008] A substantial unit of work is usually: a Work Packet (WP-…) milestone, a spec surgery step, or a clear handover point.

[HL-I-009] RAW entries MUST follow the RAW ENTRY FIELD ORDER defined in the RAW SCHEMA sub-worksurface.

[HL-I-010] COMPRESSED entries MUST follow the COMPRESSED ENTRY FIELD ORDER defined in the COMPRESSED SCHEMA sub-worksurface.

[HL-I-011] If a field is unknown or not applicable, assistants MUST write the literal word `None` instead of leaving it blank.

[HL-I-012] RAW entries MUST be appended in chronological order based on TIMESTAMP.

[HL-I-013] RAW entries MUST NOT be edited once written, except when moving them into an external archive file during compression.

[HL-I-014] COMPRESSED entries MUST be appended in chronological order of the RAW_ENTRY_RANGE they refer to.

[HL-I-015] COMPRESSED entries MUST NEVER be edited or deleted once written.

[HL-I-016] This file intentionally contains NO example log entries; assistants MUST NOT invent examples inside the LOG worksurface.

[HL-I-017] When a new RAW entry would make the total RAW entry count exceed RAW_MAX_ENTRIES, assistants MUST perform a compression cycle.

[HL-I-018] During a compression cycle, assistants MUST select the oldest RAW_COMPRESS_BATCH_SIZE RAW entries.

[HL-I-019] During a compression cycle, assistants MUST group selected RAW entries into one or more COMPRESSED entries, usually grouping by WP_ID and phase.

[HL-I-020] Each COMPRESSED entry MUST record in RAW_ENTRY_RANGE the RAW_ENTRY_IDs that were compressed.

[HL-I-021] After creating COMPRESSED entries, assistants MUST move the corresponding full RAW entries out of this file into the external archive file suggested by [HL-M-012].

[HL-I-022] After a compression cycle, the number of remaining RAW entries in this file SHOULD be approximately RAW_MIN_RECENT_TO_KEEP plus any newly added entries.

[HL-I-023] Assistants MUST NOT delete or rewrite COMPRESSED entries after a compression cycle.

[HL-I-024] Assistants MUST use the SESSION_ID_SCHEME defined in [HL-M-008] for any SESSION_ID field.

[HL-I-025] Assistants MUST use the WP_ID_SCHEME defined in [HL-M-009] for any WP_ID field.

[HL-I-026] Assistants MUST use only ROLE values defined in [HL-M-010] for the ROLE field.

[HL-I-027] Assistants MUST use only RESULT_VALUES defined in [HL-M-011] for the RESULT field.

[HL-I-028] Each RAW entry MUST contain a concrete NEXT_STEP_HINT or the literal `None`.

[HL-I-029] Each RAW entry MUST contain a HANDOFF_HINT indicating where a new assistant should start, or the literal `None`.

[HL-I-030] For Work Packet–related work, each RAW entry MUST include a WP_ID; if truly not a WP, use `None`.

[HL-I-031] FILES_TOUCHED in RAW entries MUST list only relevant paths, one per line; if none, the value MUST be `None`.

[HL-I-032] Assistants SHOULD keep TASK_SUMMARY and METHOD_SUMMARY each to a single short line.

[HL-I-033] Assistants SHOULD mention key spec sections in SPEC_REFERENCES when they influenced the work; if none, the value MUST be `None`.

[HL-I-034] Assistants MUST NOT invent non-existent files or paths in FILES_TOUCHED.

[HL-I-035] When RESULT is FAIL, BLOCKED, or PARTIAL, assistants MUST explain the reason in BLOCKERS_OR_RISKS; if RESULT is OK and no special risks exist, the value MUST be `None`.

[HL-I-036] Before starting new work, assistants SHOULD read the tail of the RAW log and, if needed, recent COMPRESSED entries.

[HL-I-037] When handing over to a new assistant, you SHOULD explicitly point them to this logger file and the latest RAW entries.

[HL-I-038] If any instruction in this worksurface conflicts with Codex v0.6 or the Master Spec, assistants MUST highlight the conflict in a new RAW entry and await user guidance.

[HL-I-039] All timestamps MUST be written in ISO 8601 format with timezone offset (example: 2025-12-06T14:23:11+01:00).

[HL-I-040] Assistants reading project state SHOULD primarily inspect the end of the file (RAW log tail) to know where to continue; COMPRESSED log is older folded history.

[INSTR_RULES_SURFACE_END]


C. RAW ENTRY SCHEMA (SUB-WORKSURFACE, REFERENCE ONLY)
-----------------------------------------------------
[INSTR_RAW_SCHEMA_SURFACE_START]

[HL-SR-001] RAW entries MUST use the following field labels in this exact order.

[HL-SR-002] RAW ENTRY FIELD ORDER:
[RAW_ENTRY_ID]
[TIMESTAMP]
[SESSION_ID]
[ROLE]
[PHASE]
[VERTICAL_SLICE]
[WP_ID]
[WP_STATUS]
[TASK_SUMMARY]
[METHOD_SUMMARY]
[SPEC_REFERENCES]
[LAW_AND_CODEX_REFERENCES]
[FILES_TOUCHED]
[TOOLS_AND_MODELS]
[STATE_BEFORE_BRIEF]
[STATE_AFTER_BRIEF]
[RESULT]
[BLOCKERS_OR_RISKS]
[NEXT_STEP_HINT]
[HANDOFF_HINT]
[NOTES]

[HL-SR-003] RAW_ENTRY_ID MUST be an integer, monotonically increasing starting at 1.

[HL-SR-004] TIMESTAMP MUST be ISO 8601 with timezone (example: 2025-12-06T14:23:11+01:00).

[HL-SR-005] SESSION_ID MUST follow SESSION_ID_SCHEME from [HL-M-008].

[HL-SR-006] ROLE MUST be one of: Owner, Orchestrator, Coder.

[HL-SR-007] RESULT MUST be one of: OK, FAIL, PARTIAL, BLOCKED.

[HL-SR-008] If no files were touched, FILES_TOUCHED MUST consist of a single line whose value is `None`.

[HL-SR-009] If a field is unknown or not applicable, its value line MUST be `None`.

[HL-SR-010] Each RAW entry MUST be separated from the next by at least one blank line for readability.

[INSTR_RAW_SCHEMA_SURFACE_END]


D. COMPRESSED ENTRY SCHEMA (SUB-WORKSURFACE, REFERENCE ONLY)
------------------------------------------------------------
[INSTR_COMP_SCHEMA_SURFACE_START]

[HL-SC-001] COMPRESSED entries MUST use the following field labels in this exact order.

[HL-SC-002] COMPRESSED ENTRY FIELD ORDER:
[C_ENTRY_ID]
[RAW_ENTRY_RANGE]
[PHASE]
[VERTICAL_SLICE]
[WP_ID]
[TASK_AND_METHOD_BRIEF]
[FILES_TOUCHED_SUMMARY]
[OUTCOME_BRIEF]
[LINKED_ARTIFACTS]

[HL-SC-003] C_ENTRY_ID MUST be an integer, monotonically increasing, separate from RAW_ENTRY_ID.

[HL-SC-004] RAW_ENTRY_RANGE MUST reference archived RAW_ENTRY_IDs, e.g. "1-3" or "4,7,8".

[HL-SC-005] TASK_AND_METHOD_BRIEF MUST be a single line summarising what was done and how.

[HL-SC-006] FILES_TOUCHED_SUMMARY MUST be a single line with key paths separated by commas; if none, the value MUST be `None`.

[HL-SC-007] OUTCOME_BRIEF MUST be a single line describing the final outcome.

[HL-SC-008] If there are no linked artifacts, LINKED_ARTIFACTS MUST be the single line `None`.

[HL-SC-009] Each COMPRESSED entry MUST be separated from the next by at least one blank line for readability.

[INSTR_COMP_SCHEMA_SURFACE_END]


[INSTRUCTIONS_SURFACE_END]
==========================



[LOG_SURFACE_START]
===================

D. COMPRESSED LOG (SUB-WORKSURFACE, LIVE)
-----------------------------------------
[LOG_COMPRESSED_SURFACE_START]

[LOG_COMPRESSED_SURFACE_END]


E. RAW LOG (SUB-WORKSURFACE, LIVE)
----------------------------------
[LOG_RAW_SURFACE_START]

[RAW_ENTRY_ID]
1
[TIMESTAMP]
2025-12-07T02:00:00+01:00
[SESSION_ID]
HS-20251207-0200-tauri-bootstrap
[ROLE]
Orchestrator
[PHASE]
P0-Env-Setup
[VERTICAL_SLICE]
VS-DesktopShell-Bootstrap
[WP_ID]
WP-0-Env-And-Tauri-Scaffold
[WP_STATUS]
Started
[TASK_SUMMARY]
Guided user through installing Windows toolchain and scaffolding a Tauri + React desktop app for Handshake.
[METHOD_SUMMARY]
Used PowerShell, Node/npm, Rust + cargo, Tauri CLI, and Visual Studio Build Tools to resolve linker issues and run the starter app.
[SPEC_REFERENCES]
Handshake_Master_Spec_v02.12.md §7.6 Development Roadmap; Handshake Codex v0.5 (environment and tooling behaviour).
[LAW_AND_CODEX_REFERENCES]
Handshake Codex v0.5; Handshake_logger_v3.1 instructions.
[FILES_TOUCHED]
D:\Projects\LLM projects\Handshakepp
D:\Projects\LLM projects\Handshakepp\src
D:\Projects\LLM projects\Handshakepp\src-tauri
[TOOLS_AND_MODELS]
PowerShell; Node.js v24.11.1; npm v11.6.2; Rust v1.91.1; cargo v1.91.1; Tauri CLI; Visual Studio 2022 Build Tools (MSVC); ChatGPT (GPT-5.1 Thinking).
[STATE_BEFORE_BRIEF]
Handshake repo existed with specs but no Tauri desktop shell; Node/Rust installed but MSVC linker missing; no app folder or dev run.
[STATE_AFTER_BRIEF]
Tauri React+TypeScript template created under Handshakepp; npm dependencies installed; Rust/MSVC toolchain working; npm run tauri dev opens starter window successfully.
[RESULT]
OK
[BLOCKERS_OR_RISKS]
Installer and toolchain setup were heavy; risk of future drift between spec’s planned src/frontend/desktop layout and current app/ placement—must reconcile before further structure work.
[NEXT_STEP_HINT]
Decide whether to move the Tauri app under src/frontend/desktop or update the layout spec, then define the first Handshake UI vertical slice (diagnostic shell/healthcheck).
[HANDOFF_HINT]
Next assistant should inspect D:\Projects\LLM projects\Handshakepp and treat it as the current prototype desktop shell; align any refactors with the Master Spec before moving or renaming.
[NOTES]
User prefers minimal, step-by-step CLI instructions with no future-step spam; keep path handling exact and avoid placeholder paths.

[LOG_RAW_SURFACE_END]


[LOG_SURFACE_END]
=================


END OF LOGGER v3.1
